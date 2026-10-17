//! Offline world backup primitives with per-instance lifecycle exclusion.
//!
//! The public API is asynchronous. Large copies and checksum passes run on
//! Tokio's blocking pool; creating a backup consumes a stopped-instance lease
//! so a concurrent lifecycle request cannot start the server mid-copy.

mod error;
mod listing;
mod manifest;
mod store;
mod tree;
pub mod worlds;

pub use error::BackupError;
pub use manifest::{BackupEntry, BackupEntryKind, BackupManifest};
pub use mc_core::{BackupConsistency, BackupCursor, BackupPage, BackupSummary};
pub use store::BackupService;

#[cfg(test)]
mod tests;
