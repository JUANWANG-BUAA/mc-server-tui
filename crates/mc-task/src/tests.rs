use super::*;
use chrono::Utc;
use mc_core::{
    AppError, ErrorCode, InstanceSummary, TaskKind, TaskPhase, TaskProgress, TaskResult,
    TaskSnapshot, TaskStatus,
};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
use tempfile::TempDir;
use tokio::{
    sync::{mpsc, Notify, Semaphore},
    time::{timeout, Duration},
};
use uuid::Uuid;

fn database(temp: &TempDir) -> std::path::PathBuf {
    temp.path().join("index.sqlite3")
}

fn instance_result() -> TaskResult {
    TaskResult::InstanceCreated(InstanceSummary {
        id: Uuid::new_v4(),
        name: "test instance".to_owned(),
        provider: "paper".to_owned(),
        game_version: "1.21.1".to_owned(),
        build_id: 123,
        java_path: "/usr/bin/java".into(),
        min_memory_mb: 1024,
        max_memory_mb: 2048,
        port: 25565,
        directory: "/tmp/test-instance".into(),
        created_at: Utc::now(),
    })
}

async fn wait_until_finished(manager: &TaskManager, id: Uuid) -> TaskSnapshot {
    timeout(Duration::from_secs(5), async {
        loop {
            let snapshot = manager.get(id).await.expect("task should remain readable");
            if matches!(
                snapshot.status,
                TaskStatus::Succeeded
                    | TaskStatus::Failed
                    | TaskStatus::Interrupted
                    | TaskStatus::Cancelled
            ) {
                return snapshot;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("task should finish before the test timeout")
}

#[tokio::test]
async fn task_phases_and_events_are_durable_and_ordered() {
    let temp = TempDir::new().expect("temporary state directory");
    let manager = TaskManager::with_limits(database(&temp), 2, 4)
        .await
        .expect("task manager opens");
    let queued = manager
        .submit(TaskKind::CreatePaperInstance, None, |reporter| async move {
            reporter
                .set_phase(
                    TaskPhase::DownloadingArtifact,
                    "Downloading fixture.",
                    Some(TaskProgress {
                        completed_steps: 1,
                        total_steps: 3,
                        transfer: None,
                    }),
                )
                .await?;
            Ok(instance_result())
        })
        .await
        .expect("task should be accepted");
    let completed = wait_until_finished(&manager, queued.id).await;

    assert_eq!(completed.status, TaskStatus::Succeeded);
    assert_eq!(completed.phase, TaskPhase::Completed);
    assert_eq!(completed.last_event_sequence, 4);
    assert_eq!(
        completed.progress,
        Some(TaskProgress {
            completed_steps: 3,
            total_steps: 3,
            transfer: None,
        })
    );
    assert!(completed.instance_id.is_some());

    let events = manager
        .events(queued.id, 0, 20)
        .await
        .expect("task event history should be readable");
    assert_eq!(events.len(), 4);
    assert!(events
        .windows(2)
        .all(|pair| pair[0].sequence < pair[1].sequence));
    assert_eq!(events[0].status, TaskStatus::Queued);
    assert_eq!(events[1].status, TaskStatus::Running);
    assert_eq!(events[2].phase, TaskPhase::DownloadingArtifact);
    assert_eq!(events[3].phase, TaskPhase::Completed);
    assert_eq!(
        manager
            .events(queued.id, events[1].sequence, 20)
            .await
            .expect("event cursor query should work")
            .len(),
        2
    );

    let reopened = TaskStore::open(database(&temp))
        .await
        .expect("task database should reopen");
    assert_eq!(
        reopened.get(queued.id).await.expect("persisted task"),
        completed
    );
}

#[tokio::test]
async fn backup_task_records_wait_phase_and_persists_its_backup_identity() {
    let temp = TempDir::new().expect("temporary state directory");
    let manager = TaskManager::with_limits(database(&temp), 1, 2)
        .await
        .expect("task manager opens");
    let instance_id = Uuid::new_v4();
    let backup_id = Uuid::new_v4();
    let queued = manager
        .submit(
            TaskKind::CreateWorldBackup,
            Some(instance_id),
            move |_| async move {
                Ok(TaskResult::WorldBackupCreated {
                    instance_id,
                    backup_id,
                })
            },
        )
        .await
        .expect("backup task is accepted");
    let completed = wait_until_finished(&manager, queued.id).await;

    assert_eq!(completed.status, TaskStatus::Succeeded);
    assert_eq!(completed.progress.unwrap().completed_steps, 1);
    assert_eq!(
        completed.result,
        Some(TaskResult::WorldBackupCreated {
            instance_id,
            backup_id,
        })
    );
    let events = manager
        .events(queued.id, 0, 10)
        .await
        .expect("read backup events");
    assert_eq!(events[1].phase, TaskPhase::WaitingForInstance);
}

#[tokio::test]
async fn duplicate_active_backup_submissions_reuse_the_same_task() {
    let temp = TempDir::new().expect("temporary state directory");
    let manager = TaskManager::with_limits(database(&temp), 1, 2)
        .await
        .expect("task manager opens");
    let instance_id = Uuid::new_v4();
    let started = Arc::new(Notify::new());
    let release = Arc::new(Notify::new());
    let work_calls = Arc::new(AtomicUsize::new(0));

    let first_started = Arc::clone(&started);
    let first_release = Arc::clone(&release);
    let first_calls = Arc::clone(&work_calls);
    let first_submission = manager.submit_or_reuse_active(
        TaskKind::CreateWorldBackup,
        instance_id,
        move |_| async move {
            first_calls.fetch_add(1, Ordering::SeqCst);
            first_started.notify_one();
            first_release.notified().await;
            Ok(TaskResult::WorldBackupCreated {
                instance_id,
                backup_id: Uuid::new_v4(),
            })
        },
    );
    let second_started = Arc::clone(&started);
    let second_release = Arc::clone(&release);
    let second_calls = Arc::clone(&work_calls);
    let second_submission = manager.submit_or_reuse_active(
        TaskKind::CreateWorldBackup,
        instance_id,
        move |_| async move {
            second_calls.fetch_add(1, Ordering::SeqCst);
            second_started.notify_one();
            second_release.notified().await;
            Ok(TaskResult::WorldBackupCreated {
                instance_id,
                backup_id: Uuid::new_v4(),
            })
        },
    );
    let (first, duplicate) = tokio::join!(first_submission, second_submission);
    let first = first.expect("first backup task is accepted");
    let duplicate = duplicate.expect("duplicate request returns the active task");
    started.notified().await;

    assert_eq!(duplicate.id, first.id);
    assert_eq!(work_calls.load(Ordering::SeqCst), 1);
    release.notify_one();
    let completed = wait_until_finished(&manager, first.id).await;
    assert_eq!(completed.status, TaskStatus::Succeeded);

    let next = manager
        .submit_or_reuse_active(
            TaskKind::CreateWorldBackup,
            instance_id,
            move |_| async move {
                Ok(TaskResult::WorldBackupCreated {
                    instance_id,
                    backup_id: Uuid::new_v4(),
                })
            },
        )
        .await
        .expect("a completed backup can be followed by a new backup");
    assert_ne!(next.id, first.id);
    assert_eq!(
        wait_until_finished(&manager, next.id).await.status,
        TaskStatus::Succeeded
    );
}

#[tokio::test]
async fn task_admission_and_execution_are_bounded() {
    let temp = TempDir::new().expect("temporary state directory");
    let manager = TaskManager::with_limits(database(&temp), 2, 3)
        .await
        .expect("task manager opens");
    let active = Arc::new(AtomicUsize::new(0));
    let maximum = Arc::new(AtomicUsize::new(0));
    let release = Arc::new(Semaphore::new(0));
    let (started_sender, mut started_receiver) = mpsc::channel(3);
    let mut ids = Vec::new();

    for _ in 0..3 {
        let active = Arc::clone(&active);
        let maximum = Arc::clone(&maximum);
        let release = Arc::clone(&release);
        let started_sender = started_sender.clone();
        let task = manager
            .submit(
                TaskKind::CreatePaperInstance,
                None,
                move |_reporter| async move {
                    let count = active.fetch_add(1, Ordering::SeqCst) + 1;
                    maximum.fetch_max(count, Ordering::SeqCst);
                    started_sender
                        .send(())
                        .await
                        .expect("test observer remains available");
                    let permit = release
                        .acquire()
                        .await
                        .expect("release semaphore remains open");
                    drop(permit);
                    active.fetch_sub(1, Ordering::SeqCst);
                    Ok(instance_result())
                },
            )
            .await
            .expect("all outstanding slots should accept three tasks");
        ids.push(task.id);
    }
    drop(started_sender);

    let queue_error = manager
        .submit(
            TaskKind::CreatePaperInstance,
            None,
            |_reporter| async move { Ok(instance_result()) },
        )
        .await
        .expect_err("a fourth outstanding task should be rejected");
    assert_eq!(queue_error.code, ErrorCode::TaskQueueFull);
    assert!(queue_error.retryable);

    started_receiver.recv().await.expect("first worker started");
    started_receiver
        .recv()
        .await
        .expect("second worker started");
    assert!(timeout(Duration::from_millis(50), started_receiver.recv())
        .await
        .is_err());
    assert_eq!(maximum.load(Ordering::SeqCst), 2);

    release.add_permits(2);
    timeout(Duration::from_secs(2), started_receiver.recv())
        .await
        .expect("third worker should start after a permit is released")
        .expect("third worker started");
    release.add_permits(1);

    for id in ids {
        assert_eq!(
            wait_until_finished(&manager, id).await.status,
            TaskStatus::Succeeded
        );
    }
    assert_eq!(maximum.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn operations_for_one_instance_serialize_while_other_instances_can_run() {
    let temp = TempDir::new().expect("temporary state directory");
    let manager = TaskManager::with_limits(database(&temp), 2, 4)
        .await
        .expect("task manager opens");
    let instance_a = Uuid::new_v4();
    let instance_b = Uuid::new_v4();
    let gate = Arc::new(Semaphore::new(0));
    let (started_sender, mut started_receiver) = mpsc::channel(3);

    let first_gate = Arc::clone(&gate);
    let first_sender = started_sender.clone();
    let first = manager
        .submit(
            TaskKind::CreatePaperInstance,
            Some(instance_a),
            move |_reporter| async move {
                first_sender.send(1).await.expect("test observer is open");
                let permit = first_gate.acquire().await.expect("gate remains open");
                drop(permit);
                Ok(instance_result())
            },
        )
        .await
        .expect("first operation is accepted");
    assert_eq!(started_receiver.recv().await, Some(1));

    let second_gate = Arc::clone(&gate);
    let second_sender = started_sender.clone();
    let second = manager
        .submit(
            TaskKind::CreatePaperInstance,
            Some(instance_a),
            move |_reporter| async move {
                second_sender.send(2).await.expect("test observer is open");
                let permit = second_gate.acquire().await.expect("gate remains open");
                drop(permit);
                Ok(instance_result())
            },
        )
        .await
        .expect("second operation is accepted");

    let other_sender = started_sender.clone();
    let other = manager
        .submit(
            TaskKind::CreatePaperInstance,
            Some(instance_b),
            move |_reporter| async move {
                other_sender.send(3).await.expect("test observer is open");
                Ok(instance_result())
            },
        )
        .await
        .expect("an independent instance may run concurrently");
    drop(started_sender);

    assert_eq!(
        timeout(Duration::from_secs(2), started_receiver.recv())
            .await
            .expect("independent instance should start")
            .expect("start event exists"),
        3
    );
    assert!(timeout(Duration::from_millis(50), started_receiver.recv())
        .await
        .is_err());

    gate.add_permits(1);
    assert_eq!(
        timeout(Duration::from_secs(2), started_receiver.recv())
            .await
            .expect("queued same-instance operation should start after release")
            .expect("start event exists"),
        2
    );
    gate.add_permits(1);

    for id in [first.id, second.id, other.id] {
        assert_eq!(
            wait_until_finished(&manager, id).await.status,
            TaskStatus::Succeeded
        );
    }
}

#[tokio::test]
async fn startup_marks_queued_and_running_tasks_interrupted() {
    let temp = TempDir::new().expect("temporary state directory");
    let store = TaskStore::open(database(&temp)).await.expect("store opens");
    let queued = store
        .create(TaskKind::CreatePaperInstance, None)
        .await
        .expect("queued task created");
    let mut running = queued.clone();
    running.id = Uuid::new_v4();
    running.status = TaskStatus::Running;
    running.phase = TaskPhase::DownloadingArtifact;
    store
        .persist(&mut running)
        .await
        .expect("running task persisted");
    drop(store);

    let recovered = TaskStore::open(database(&temp))
        .await
        .expect("store recovers after restart");
    for id in [queued.id, running.id] {
        let task = recovered.get(id).await.expect("interrupted task exists");
        assert_eq!(task.status, TaskStatus::Interrupted);
        assert_eq!(task.phase, TaskPhase::Interrupted);
        assert_eq!(
            task.error.expect("interruption has structured error").code,
            ErrorCode::TaskInterrupted
        );
    }
}

#[tokio::test]
async fn panicking_worker_becomes_a_failed_task_instead_of_a_stuck_task() {
    let temp = TempDir::new().expect("temporary state directory");
    let manager = TaskManager::with_limits(database(&temp), 1, 2)
        .await
        .expect("task manager opens");
    let queued = manager
        .submit(
            TaskKind::CreatePaperInstance,
            None,
            |_reporter| async move {
                panic!("simulated worker panic");
                #[allow(unreachable_code)]
                Ok(instance_result())
            },
        )
        .await
        .expect("task is accepted");
    let failed = wait_until_finished(&manager, queued.id).await;

    assert_eq!(failed.status, TaskStatus::Failed);
    assert_eq!(
        failed.error.expect("panic is surfaced").code,
        ErrorCode::Internal
    );
}

#[tokio::test]
async fn task_store_rejects_a_future_schema_without_overwriting_it() {
    let temp = TempDir::new().expect("temporary state directory");
    let path = database(&temp);
    let connection = rusqlite::Connection::open(&path).expect("database opens");
    connection
        .execute_batch(
            "CREATE TABLE mc_task_schema (id INTEGER PRIMARY KEY, version INTEGER NOT NULL);
             INSERT INTO mc_task_schema (id, version) VALUES (1, 99);",
        )
        .expect("future schema fixture created");
    drop(connection);

    let error = match TaskStore::open(&path).await {
        Ok(_) => panic!("future task schema must be rejected"),
        Err(error) => error,
    };
    assert_eq!(error.code, ErrorCode::SourceSchemaChanged);
    let connection = rusqlite::Connection::open(&path).expect("database remains readable");
    let version: i64 = connection
        .query_row(
            "SELECT version FROM mc_task_schema WHERE id = 1",
            [],
            |row| row.get(0),
        )
        .expect("schema version remains intact");
    assert_eq!(version, 99);
}

#[tokio::test]
async fn task_schema_migrates_without_changing_the_instance_index_version() {
    let temp = TempDir::new().expect("temporary state directory");
    let path = database(&temp);
    let connection = rusqlite::Connection::open(&path).expect("database opens");
    connection
        .execute_batch("PRAGMA user_version = 1;")
        .expect("instance index version fixture created");
    drop(connection);

    let store = TaskStore::open(&path)
        .await
        .expect("task schema can share the index database");
    store
        .create(TaskKind::CreatePaperInstance, None)
        .await
        .expect("task should persist in shared database");

    let connection = rusqlite::Connection::open(&path).expect("database remains readable");
    let index_version: i64 = connection
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .expect("instance index version remains queryable");
    let task_tables: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' \
             AND name IN ('mc_tasks', 'mc_task_events')",
            [],
            |row| row.get(0),
        )
        .expect("task tables remain queryable");
    assert_eq!(index_version, 1);
    assert_eq!(task_tables, 2);
}

#[tokio::test]
async fn cooperative_cancellation_reaches_a_distinct_terminal_state() {
    let temp = TempDir::new().expect("temporary state directory");
    let manager = TaskManager::with_limits(database(&temp), 2, 4)
        .await
        .expect("task manager opens");
    let queued = manager
        .submit(TaskKind::CreatePaperInstance, None, |reporter| async move {
            while !reporter.is_cancelled() {
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
            Err(AppError::new(
                ErrorCode::TaskCancelled,
                "The task was cancelled by user request.",
            ))
        })
        .await
        .expect("task should be accepted");
    let accepted = manager.cancel(queued.id).await.expect("cancel accepted");
    assert_eq!(accepted.id, queued.id);
    let finished = wait_until_finished(&manager, queued.id).await;
    assert_eq!(finished.status, TaskStatus::Cancelled);
    assert_eq!(finished.phase, TaskPhase::Cancelled);
    assert!(finished.error.is_none());
    assert!(finished.message.contains("cancelled"));
}
