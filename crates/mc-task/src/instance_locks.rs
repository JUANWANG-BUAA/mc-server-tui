use std::{
    collections::HashMap,
    sync::{Arc, Weak},
};
use tokio::sync::{Mutex, OwnedMutexGuard};
use uuid::Uuid;

#[derive(Clone, Default)]
pub(crate) struct InstanceLockRegistry {
    entries: Arc<Mutex<HashMap<Uuid, Weak<Mutex<()>>>>>,
}

impl InstanceLockRegistry {
    pub(crate) async fn acquire(&self, instance_id: Uuid) -> OwnedMutexGuard<()> {
        let lock = {
            let mut entries = self.entries.lock().await;
            entries.retain(|_, lock| lock.strong_count() > 0);
            match entries.get(&instance_id).and_then(Weak::upgrade) {
                Some(lock) => lock,
                None => {
                    let lock = Arc::new(Mutex::new(()));
                    entries.insert(instance_id, Arc::downgrade(&lock));
                    lock
                }
            }
        };
        lock.lock_owned().await
    }
}
