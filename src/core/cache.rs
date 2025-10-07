use crate::{Result, CobraError, Package, constants::*};
use bytes::Bytes;
use lru::LruCache;
use sled::Db;
use bloomfilter::Bloom;
use std::sync::Arc;
use tokio::sync::RwLock;
use std::num::NonZeroUsize;
use crate::utils::fs::get_cache_dir;

/// Multi-level cache: Memory -> Disk -> Network
pub struct MultiLevelCache {
    memory: Arc<RwLock<LruCache<String, Bytes>>>,
    disk: Db,
    bloom: Arc<RwLock<Bloom<String>>>,
    hits: Arc<RwLock<u64>>,
    misses: Arc<RwLock<u64>>,
}

impl MultiLevelCache {
    pub async fn new() -> Result<Self> {
        let cache_dir = get_cache_dir()?;
        let db_path = cache_dir.join("packages");
        
        let disk = sled::open(&db_path)
            .map_err(|e| CobraError::Cache(format!("Failed to open disk cache: {}", e)))?;
        
        // Initialize bloom filter for fast negative lookups
        let bloom = Bloom::new_for_fp_rate(10000, 0.01);
        
        Ok(Self {
            memory: Arc::new(RwLock::new(
                LruCache::new(NonZeroUsize::new(MEMORY_CACHE_ENTRIES).unwrap())
            )),
            disk,
            bloom: Arc::new(RwLock::new(bloom)),
            hits: Arc::new(RwLock::new(0)),
            misses: Arc::new(RwLock::new(0)),
        })
    }

    pub async fn get(&self, key: &str) -> Option<Bytes> {
        // Check bloom filter first (fastest)
        {
            let bloom = self.bloom.read().await;
            if !bloom.check(&key.to_string()) {
                *self.misses.write().await += 1;
                return None;
            }
        }

        // Check memory cache
        {
            let mut memory = self.memory.write().await;
            if let Some(data) = memory.get(key) {
                *self.hits.write().await += 1;
                return Some(data.clone());
            }
        }

        // Check disk cache
        match self.disk.get(key) {
            Ok(Some(data)) => {
                let bytes = Bytes::from(data.to_vec());
                // Promote to memory cache
                self.memory.write().await.put(key.to_string(), bytes.clone());
                *self.hits.write().await += 1;
                Some(bytes)
            }
            _ => {
                *self.misses.write().await += 1;
                None
            }
        }
    }

    pub async fn put(&self, key: String, data: Bytes) -> Result<()> {
        // Add to bloom filter
        self.bloom.write().await.set(&key);
        
        // Add to memory cache
        self.memory.write().await.put(key.clone(), data.clone());
        
        // Add to disk cache
        self.disk.insert(key.as_bytes(), data.as_ref())
            .map_err(|e| CobraError::Cache(format!("Failed to write to disk cache: {}", e)))?;
        
        Ok(())
    }

    pub async fn clear(&self) -> Result<()> {
        self.memory.write().await.clear();
        self.disk.clear()
            .map_err(|e| CobraError::Cache(format!("Failed to clear disk cache: {}", e)))?;
        *self.bloom.write().await = Bloom::new_for_fp_rate(10000, 0.01);
        *self.hits.write().await = 0;
        *self.misses.write().await = 0;
        Ok(())
    }

    pub async fn hit_rate(&self) -> f64 {
        let hits = *self.hits.read().await;
        let misses = *self.misses.read().await;
        let total = hits + misses;
        if total == 0 {
            0.0
        } else {
            hits as f64 / total as f64
        }
    }
}
