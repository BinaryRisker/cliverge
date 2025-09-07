//! Caching system for CLIverge

use crate::{ConfigError, ToolStatus};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

// 类型别名以减少复杂度警告
type StatusCache = HashMap<String, CacheEntry<ToolStatus>>;
type HelpCache = HashMap<String, CacheEntry<String>>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntry<T> {
    pub data: T,
    pub timestamp: u64,
    pub ttl_seconds: u64,
}

impl<T> CacheEntry<T> {
    pub fn new(data: T, ttl_seconds: u64) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or(Duration::from_secs(0))
            .as_secs();

        Self {
            data,
            timestamp,
            ttl_seconds,
        }
    }

    pub fn is_expired(&self) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or(Duration::from_secs(0))
            .as_secs();

        now > self.timestamp + self.ttl_seconds
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ToolCache {
    pub status_cache: StatusCache,
    pub help_cache: HelpCache,
}

pub struct CacheManager {
    cache: ToolCache,
    cache_file: PathBuf,
}

impl CacheManager {
    pub fn new(cache_dir: PathBuf) -> Self {
        let cache_file = cache_dir.join("tool_cache.json");

        Self {
            cache: ToolCache::default(),
            cache_file,
        }
    }

    pub async fn load(&mut self) -> Result<(), ConfigError> {
        if self.cache_file.exists() {
            let content = tokio::fs::read_to_string(&self.cache_file).await?;
            self.cache = serde_json::from_str(&content)?;

            // Clean expired entries
            self.clean_expired();
        }

        Ok(())
    }

    pub async fn save(&self) -> Result<(), ConfigError> {
        if let Some(parent) = self.cache_file.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        let json = serde_json::to_string_pretty(&self.cache)?;
        tokio::fs::write(&self.cache_file, json).await?;

        Ok(())
    }

    pub fn get_tool_status(&self, tool_id: &str) -> Option<ToolStatus> {
        self.cache.status_cache.get(tool_id).and_then(|entry| {
            if entry.is_expired() {
                None
            } else {
                Some(entry.data.clone())
            }
        })
    }

    pub fn set_tool_status(&mut self, tool_id: &str, status: ToolStatus) {
        // 1 day TTL for status
        let entry = CacheEntry::new(status, 86400);
        self.cache.status_cache.insert(tool_id.to_string(), entry);
    }

    pub fn get_tool_help(&self, tool_id: &str) -> Option<String> {
        self.cache.help_cache.get(tool_id).and_then(|entry| {
            if entry.is_expired() {
                None
            } else {
                Some(entry.data.clone())
            }
        })
    }

    pub fn set_tool_help(&mut self, tool_id: &str, help: String) {
        // 7 days TTL for help text (help rarely changes)
        let entry = CacheEntry::new(help, 604800);
        self.cache.help_cache.insert(tool_id.to_string(), entry);
    }

    pub fn invalidate_tool(&mut self, tool_id: &str) {
        self.cache.status_cache.remove(tool_id);
        self.cache.help_cache.remove(tool_id);
    }

    pub fn clear_all(&mut self) {
        self.cache = ToolCache::default();
    }

    pub fn get_cache_stats(&self) -> (usize, usize) {
        (self.cache.status_cache.len(), self.cache.help_cache.len())
    }

    fn clean_expired(&mut self) {
        self.cache
            .status_cache
            .retain(|_, entry| !entry.is_expired());
        self.cache.help_cache.retain(|_, entry| !entry.is_expired());
    }
}
