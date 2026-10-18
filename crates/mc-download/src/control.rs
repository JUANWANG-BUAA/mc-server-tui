use std::sync::{
    atomic::{AtomicBool, AtomicU64, Ordering},
    Arc,
};

/// Shared, lock-free transfer state used by the task layer to display byte
/// progress and request a safe cancellation between network chunks.
#[derive(Debug, Clone)]
pub struct DownloadControl {
    cancelled: Arc<AtomicBool>,
    downloaded_bytes: Arc<AtomicU64>,
}

impl DownloadControl {
    pub fn new(cancelled: Arc<AtomicBool>) -> Self {
        Self {
            cancelled,
            downloaded_bytes: Arc::new(AtomicU64::new(0)),
        }
    }

    pub fn downloaded_bytes(&self) -> u64 {
        self.downloaded_bytes.load(Ordering::Relaxed)
    }

    pub fn request_cancel(&self) {
        self.cancelled.store(true, Ordering::Relaxed);
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Relaxed)
    }

    pub(crate) fn set_downloaded_bytes(&self, bytes: u64) {
        self.downloaded_bytes.store(bytes, Ordering::Relaxed);
    }
}

impl Default for DownloadControl {
    fn default() -> Self {
        Self::new(Arc::new(AtomicBool::new(false)))
    }
}
