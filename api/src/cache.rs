// Cache layer for transaction lists
// Supports in-memory and Redis backends

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedResponse {
    pub data: Vec<u8>, // JSON serialized
    pub etag: String,
    pub cached_at: SystemTime,
}

pub trait Cache: Send + Sync {
    fn get(&self, key: &str) -> Option<CachedResponse>;
    fn set(&self, key: &str, value: CachedResponse, ttl_secs: u64);
    fn delete(&self, key: &str);
}

// In-memory cache implementation with TTL
pub struct MemoryCache {
    store: Arc<Mutex<HashMap<String, (CachedResponse, SystemTime)>>>,
    max_entries: usize,
}

impl MemoryCache {
    pub fn new(max_entries: usize) -> Self {
        Self {
            store: Arc::new(Mutex::new(HashMap::new())),
            max_entries,
        }
    }

    fn cleanup_expired(&self, ttl_secs: u64) {
        let mut store = self.store.lock().unwrap();
        let now = SystemTime::now();
        store.retain(|_, (_, expires_at)| {
            now.duration_since(*expires_at)
                .map(|d| d.as_secs() < ttl_secs)
                .unwrap_or(false)
        });
    }

    fn evict_if_needed(&self) {
        let mut store = self.store.lock().unwrap();
        if store.len() >= self.max_entries {
            // Remove oldest entries (simple LRU-like)
            if let Some(oldest_key) = store
                .iter()
                .min_by_key(|(_, (_, expires_at))| *expires_at)
                .map(|(k, _)| k.clone())
            {
                store.remove(&oldest_key);
            }
        }
    }
}

impl Cache for MemoryCache {
    fn get(&self, key: &str) -> Option<CachedResponse> {
        let store = self.store.lock().unwrap();
        store.get(key).and_then(|(cached, expires_at)| {
            let now = SystemTime::now();
            if now < *expires_at {
                Some(cached.clone())
            } else {
                None
            }
        })
    }

    fn set(&self, key: &str, value: CachedResponse, ttl_secs: u64) {
        self.evict_if_needed();
        let mut store = self.store.lock().unwrap();
        let expires_at = SystemTime::now() + Duration::from_secs(ttl_secs);
        store.insert(key.to_string(), (value, expires_at));
    }

    fn delete(&self, key: &str) {
        let mut store = self.store.lock().unwrap();
        store.remove(key);
    }
}

// Factory for creating cache instances
pub fn create_cache(backend: &str, max_entries: usize) -> Arc<dyn Cache> {
    match backend {
        "memory" => Arc::new(MemoryCache::new(max_entries)),
        "redis" => {
            // TODO: Implement Redis cache if needed
            tracing::warn!("Redis cache not implemented yet, falling back to memory");
            Arc::new(MemoryCache::new(max_entries))
        }
        _ => {
            tracing::warn!("Unknown cache backend '{}', using memory", backend);
            Arc::new(MemoryCache::new(max_entries))
        }
    }
}

