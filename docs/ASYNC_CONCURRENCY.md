# Async and concurrency contract

This contract keeps slow network, filesystem, and SQLite work from blocking the daemon's Tokio workers, while limiting how much work the process accepts at once.

## Request path

- The Unix socket listener admits at most 64 in-flight connections. Excess connections are closed immediately; CLI clients treat an early close as a retryable daemon-busy result. CLI connect/write waits are limited to 5/10 seconds. Incomplete daemon frames and daemon writes time out after 10 seconds; CLI response waits after 40 seconds.
- Socket framing, provider calls, and Tokio filesystem operations are asynchronous. A Paper create request validates its local inputs, durably creates a queued task, and returns its task ID without waiting for metadata resolution or artifact download.
- Synchronous Java path resolution, SQLite operations, available-space checks, and port probes run through `spawn_blocking`.
- SQLite task access has four blocking slots and a five-second busy timeout. Its task schema is versioned independently in `mc_task_schema` while sharing `index.sqlite3` with the instance index.

## Background task limits

- At most two task bodies run concurrently; at most eight tasks may be queued or running. Admission beyond that returns retryable `TASK_QUEUE_FULL`.
- The limit covers spawned workers waiting for execution permits as well as active work, so repeated submissions cannot create an unbounded number of Tokio tasks.
- Operations tied to an existing instance can submit its UUID as a lock key. The task manager serializes work for that UUID while allowing work for different UUIDs to run concurrently. New-instance creation has no existing key.
- A worker panic becomes a structured failed task. Queued and running tasks found on daemon startup become `interrupted`; operations are not replayed automatically because their external side effects may already have occurred.

## Progress and UI contract

- The task row and its event are committed in one SQLite transaction before a phase update is reported complete.
- Progress counts completed phases, not bytes. Download currently reports an indeterminate phase because the downloader does not yet expose byte-level progress.
- Clients submit long operations once, then request `GetTask` and `ListTaskEvents(after_sequence)` as needed. Event sequence numbers let a TUI reconnect without losing transitions.
- The CLI exposes the same records through `task list`, `task show <id>`, and `task events <id>`.
- The TUI visual feedback and test loop is specified in [`TUI_TESTING.md`](TUI_TESTING.md). Its fake daemons use the shared versioned socket protocol and keep each screen's renderer test separate from process-level PTY behavior.
- The TUI reads keys through Crossterm's async event stream and selects alongside a two-second dashboard refresh. It allows one refresh in flight, coalesces a manual refresh while busy, and retains the last successful frame if a request fails.
- Paper wizard submission runs in a separate Tokio task with a capacity-one result channel. The wizard disables duplicate submission while the request is in flight but continues drawing and handling Ctrl-C; a daemon error restores editing and retains the entered values. A successful queued task is shown immediately, then the dashboard refreshes.
- Task detail loads the snapshot and event batch concurrently. It polls at the dashboard interval with one detail request in flight, sends `ListTaskEvents(after_sequence)` using the last event actually received, deduplicates replayed events, and retains the task view and cursor after transient errors or when the user returns to the dashboard.
- Instance detail fetches server state and a bounded log chunk concurrently. It keeps one refresh and one lifecycle command in flight; console editing pauses polling, and an accepted command resumes incremental log reads from the last byte offset.
- Lifecycle state reads use asynchronous `/proc` access on Linux and a timed `ps` subprocess on macOS. Lifecycle lock-file acquisition runs through `spawn_blocking` with asynchronous retry for brief contention; one instance is locked at a time, while a graceful stop releases the lock after sending `stop` so status and log reads remain responsive during shutdown. A resident runner owns Java pipes and serves bounded console requests over a private Unix socket, surviving management daemon restarts.
- Start is idempotent after verifying a per-instance process marker. Stop waits at most 25 seconds by default/API policy and never escalates to a signal. Unknown process identity blocks stop and console input. The console accepts at most 512 printable bytes; each log response is capped at 64 KiB.

## Requirements for later modules

- Do not call synchronous filesystem, process, socket, or database APIs from a request handler. Put unavoidable blocking work in `spawn_blocking` and bound its admission.
- Do not spawn one task per request without a semaphore or bounded queue.
- Keep network timeouts and transfer limits in the provider/download boundary. Keep one instance's lifecycle operations mutually exclusive; operations on separate instances must not hold a global lock while waiting on I/O.
- Cancellation and safe resume are not exposed yet. Until their side effects are defined, daemon restart marks active work interrupted and asks the caller to review and retry.
