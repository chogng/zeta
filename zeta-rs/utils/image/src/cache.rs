use std::collections::VecDeque;
use std::sync::Mutex;

struct CacheEntry<K, V> {
    key: K,
    value: V,
    bytes: usize,
}

pub(crate) struct BoundedLruCache<K, V> {
    entries: Mutex<VecDeque<CacheEntry<K, V>>>,
    max_entries: usize,
    max_bytes: usize,
}

impl<K, V> BoundedLruCache<K, V>
where
    K: Eq,
    V: Clone,
{
    pub(crate) const fn new(max_entries: usize, max_bytes: usize) -> Self {
        Self {
            entries: Mutex::new(VecDeque::new()),
            max_entries,
            max_bytes,
        }
    }

    pub(crate) fn get(&self, key: &K) -> Option<V> {
        let mut entries = self
            .entries
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let index = entries.iter().position(|entry| &entry.key == key)?;
        let entry = entries.remove(index)?;
        let value = entry.value.clone();
        entries.push_back(entry);
        Some(value)
    }

    pub(crate) fn insert(&self, key: K, value: V, bytes: usize) {
        if bytes > self.max_bytes || self.max_entries == 0 {
            return;
        }

        let mut entries = self
            .entries
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(index) = entries.iter().position(|entry| entry.key == key) {
            entries.remove(index);
        }
        entries.push_back(CacheEntry { key, value, bytes });

        let mut cached_bytes = entries.iter().map(|entry| entry.bytes).sum::<usize>();
        while entries.len() > self.max_entries || cached_bytes > self.max_bytes {
            let Some(evicted) = entries.pop_front() else {
                break;
            };
            cached_bytes = cached_bytes.saturating_sub(evicted.bytes);
        }
    }
}
