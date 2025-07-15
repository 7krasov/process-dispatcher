use dashmap::DashMap;
use std::hash::Hash;
use std::sync::{Arc, Weak};

#[derive(Default)]
pub struct AsyncKeyedMutex<K, V = tokio::sync::Mutex<()>>
where
    K: Eq + Hash + Clone,
{
    map: DashMap<K, Weak<V>>,
}

impl<K, V> AsyncKeyedMutex<K, V>
where
    K: Eq + Hash + Clone,
    V: Default + 'static,
{
    pub fn new() -> Self {
        Self {
            map: DashMap::new(),
        }
    }

    pub fn get_mutex(&self, key: K) -> Arc<V> {
        use dashmap::mapref::entry::Entry;

        match self.map.entry(key.clone()) {
            Entry::Occupied(mut entry) => {
                if let Some(strong) = entry.get().upgrade() {
                    strong
                } else {
                    let arc = Arc::new(V::default());
                    entry.insert(Arc::downgrade(&arc));
                    arc
                }
            }
            Entry::Vacant(entry) => {
                let arc = Arc::new(V::default());
                entry.insert(Arc::downgrade(&arc));
                arc
            }
        }
    }

    pub fn cleanup(&self) {
        let dead_keys: Vec<K> = self
            .map
            .iter()
            .filter(|entry| entry.value().upgrade().is_none())
            .map(|entry| entry.key().clone())
            .collect();
        for k in dead_keys {
            self.map.remove(&k);
        }
    }
}
