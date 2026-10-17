//! Instance file formats, atomic staging, and the rebuildable SQLite index.

mod catalog;
mod filesystem;
mod installer;
mod model;
mod recovery;
mod stage;

pub use catalog::{InstanceCatalog, SqliteCatalog};
pub use installer::InstanceService;
pub use model::{InstanceConfig, InstanceLock, InstanceSpec};

#[cfg(test)]
mod tests;
