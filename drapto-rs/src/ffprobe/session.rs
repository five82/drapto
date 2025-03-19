use std::path::Path;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::error::Result;
use super::media::MediaInfo;

/// FFprobe session to cache probe results
pub struct ProbeSession {
    cache: Arc<Mutex<HashMap<String, MediaInfo>>>,
}

impl ProbeSession {
    /// Create a new probe session
    pub fn new() -> Self {
        Self {
            cache: Arc::new(Mutex::new(HashMap::new())),
        }
    }
    
    /// Get media info for a file, using cache if available
    pub fn get_media_info<P: AsRef<Path>>(&self, path: P) -> Result<MediaInfo> {
        let path_str = path.as_ref()
            .to_str()
            .unwrap_or_default()
            .to_string();
        
        {
            // Check cache first
            let cache = self.cache.lock().unwrap();
            if let Some(info) = cache.get(&path_str) {
                return Ok(info.clone());
            }
        }
        
        // Not in cache, probe the file
        let info = MediaInfo::from_path(&path)?;
        
        // Store in cache
        {
            let mut cache = self.cache.lock().unwrap();
            cache.insert(path_str, info.clone());
        }
        
        Ok(info)
    }
    
    /// Clear the cache
    pub fn clear_cache(&self) {
        let mut cache = self.cache.lock().unwrap();
        cache.clear();
    }
    
    /// Get the number of items in the cache
    pub fn cache_size(&self) -> usize {
        let cache = self.cache.lock().unwrap();
        cache.len()
    }
}

impl Default for ProbeSession {
    fn default() -> Self {
        Self::new()
    }
}