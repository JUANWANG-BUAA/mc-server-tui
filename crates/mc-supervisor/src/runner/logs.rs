use std::io;

use mc_core::{AppError, ErrorCode, InstanceSummary};
use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWriteExt},
    sync::mpsc,
    task::JoinHandle,
};

use crate::{storage, MAX_CONSOLE_LOG_BYTES};

const LOG_QUEUE_DEPTH: usize = 32;
const LOG_READ_CHUNK: usize = 8 * 1024;

pub(crate) struct OutputCapture {
    stdout: JoinHandle<()>,
    stderr: JoinHandle<()>,
    writer: JoinHandle<Result<(), AppError>>,
}

impl OutputCapture {
    pub(crate) fn start<Out, Err>(instance: InstanceSummary, stdout: Out, stderr: Err) -> Self
    where
        Out: AsyncRead + Unpin + Send + 'static,
        Err: AsyncRead + Unpin + Send + 'static,
    {
        let (sender, receiver) = mpsc::channel(LOG_QUEUE_DEPTH);
        let stdout = tokio::spawn(forward_output(stdout, sender.clone()));
        let stderr = tokio::spawn(forward_output(stderr, sender.clone()));
        drop(sender);
        let writer = tokio::spawn(write_log_chunks(instance, receiver));
        Self {
            stdout,
            stderr,
            writer,
        }
    }

    pub(crate) async fn wait(self) -> Result<(), AppError> {
        let _ = self.stdout.await;
        let _ = self.stderr.await;
        self.writer.await.map_err(|error| {
            AppError::new(ErrorCode::Internal, "Server log writer task failed.")
                .with_evidence(error.to_string())
        })?
    }
}

async fn forward_output<R>(mut reader: R, sender: mpsc::Sender<Vec<u8>>)
where
    R: AsyncRead + Unpin,
{
    let mut buffer = vec![0; LOG_READ_CHUNK];
    loop {
        match reader.read(&mut buffer).await {
            Ok(0) | Err(_) => break,
            Ok(count) => {
                if sender.send(buffer[..count].to_vec()).await.is_err() {
                    break;
                }
            }
        }
    }
}

async fn write_log_chunks(
    instance: InstanceSummary,
    mut receiver: mpsc::Receiver<Vec<u8>>,
) -> Result<(), AppError> {
    let mut log = tokio::fs::File::from_std(storage::open_console_log(&instance).await?);
    let mut written = tokio::fs::metadata(instance.directory.join("logs/console.log"))
        .await
        .map(|metadata| metadata.len())
        .unwrap_or(0);
    while let Some(chunk) = receiver.recv().await {
        log.write_all(&chunk)
            .await
            .map_err(|error| io_error("Could not append server console output.", error))?;
        written = written.saturating_add(chunk.len() as u64);
        if written >= MAX_CONSOLE_LOG_BYTES {
            log.flush()
                .await
                .map_err(|error| io_error("Could not flush server console log.", error))?;
            drop(log);
            rotate_live_log(&instance).await?;
            log = tokio::fs::File::from_std(storage::open_console_log(&instance).await?);
            written = 0;
        }
    }
    log.flush()
        .await
        .map_err(|error| io_error("Could not flush server console log.", error))
}

async fn rotate_live_log(instance: &InstanceSummary) -> Result<(), AppError> {
    let current = instance.directory.join("logs/console.log");
    let previous = instance.directory.join("logs/console.log.1");
    if tokio::fs::try_exists(&previous)
        .await
        .map_err(|error| io_error("Could not inspect previous server log.", error))?
    {
        tokio::fs::remove_file(&previous)
            .await
            .map_err(|error| io_error("Could not remove previous server log.", error))?;
    }
    if tokio::fs::try_exists(&current)
        .await
        .map_err(|error| io_error("Could not inspect current server log.", error))?
    {
        tokio::fs::rename(&current, &previous)
            .await
            .map_err(|error| io_error("Could not rotate server console log.", error))?;
    }
    Ok(())
}

fn io_error(reason: &str, error: io::Error) -> AppError {
    AppError::new(ErrorCode::Internal, reason).with_evidence(error.to_string())
}
