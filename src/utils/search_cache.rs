use std::collections::HashMap;
use std::time::{Duration, Instant};

use crate::utils::music_queue::SongItem;

#[derive(Clone)]
struct CacheEntry {
    value: Vec<SongItem>,
    created_at: Instant,
}

pub struct SearchCache {
    entries: HashMap<String, CacheEntry>,
    ttl: Duration,
}

impl SearchCache {
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
            ttl: Duration::from_secs(600),
        }
    }

    pub fn constructor() -> Self {
        Self::new()
    }

    pub fn store_results(&mut self, key: String, results: Vec<SongItem>) {
        self.entries.insert(
            key,
            CacheEntry {
                value: results,
                created_at: Instant::now(),
            },
        );
    }

    pub fn get(&self, key: &str) -> Option<Vec<SongItem>> {
        self.entries.get(key).and_then(|entry| {
            if entry.created_at.elapsed() <= self.ttl {
                Some(entry.value.clone())
            } else {
                None
            }
        })
    }

    pub fn has(&self, key: &str) -> bool {
        self.get(key).is_some()
    }

    pub fn cleanup(&mut self) {
        self.entries
            .retain(|_, entry| entry.created_at.elapsed() <= self.ttl);
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }
}

impl Default for SearchCache {
    fn default() -> Self {
        Self::new()
    }
}
