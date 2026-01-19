//! Thread-safe reusable string buffer pool for reducing allocations.
//!
//! This module provides a pool of reusable String buffers that can be acquired,
//! used, and automatically returned to the pool when dropped.

use once_cell::sync::Lazy;
use std::collections::VecDeque;
use std::sync::Arc;

#[cfg(feature = "pool-metrics")]
use std::sync::atomic::AtomicUsize;

#[cfg(feature = "pool-metrics")]
use std::sync::atomic::Ordering;

/// Configuration for the string buffer pool.
pub struct PoolConfig {
    /// Maximum buffers per size bucket
    pub max_buffers_per_size: usize,
    /// Initial capacity for new buffers
    pub initial_capacity: usize,
    /// Maximum capacity before discarding
    pub max_capacity_before_discard: usize,
}

impl Default for PoolConfig {
    fn default() -> Self {
        Self {
            max_buffers_per_size: 4,
            initial_capacity: 4096,
            max_capacity_before_discard: 65536,
        }
    }
}

/// Thread-safe reusable string buffer pool.
///
/// This pool allows allocation and reuse of String buffers to reduce memory allocations
/// during document extraction. Buffers are returned to the pool with cleared contents
/// but preserved capacity, ready for reuse.
///
/// # Thread Safety
///
/// The pool uses DashMap for lock-free concurrent access. Multiple threads can
/// acquire and release buffers simultaneously.
///
/// # Usage
///
/// ```rust,ignore
/// use kreuzberg::utils::string_pool::STRING_BUFFER_POOL;
///
/// // Acquire a buffer from the pool
/// let mut buffer = STRING_BUFFER_POOL.acquire();
/// buffer.push_str("some content");
/// // Automatically returned to pool when dropped
/// drop(buffer);
/// ```
pub struct StringBufferPool {
    pool: dashmap::DashMap<usize, VecDeque<String>>,
    config: PoolConfig,
    #[cfg(feature = "pool-metrics")]
    acquire_count: AtomicUsize,
    #[cfg(feature = "pool-metrics")]
    reuse_count: AtomicUsize,
}

impl StringBufferPool {
    /// Create a new string buffer pool with given configuration.
    pub fn new(config: PoolConfig) -> Self {
        StringBufferPool {
            pool: dashmap::DashMap::new(),
            config,
            #[cfg(feature = "pool-metrics")]
            acquire_count: AtomicUsize::new(0),
            #[cfg(feature = "pool-metrics")]
            reuse_count: AtomicUsize::new(0),
        }
    }

    /// Find the appropriate bucket size for a given capacity.
    fn find_bucket(&self, capacity: usize) -> usize {
        if capacity <= 1024 {
            1024
        } else if capacity <= 4096 {
            4096
        } else if capacity <= 16384 {
            16384
        } else if capacity <= 65536 {
            65536
        } else {
            262144
        }
    }

    /// Try to acquire a buffer from a specific bucket, returning it if found.
    fn try_acquire_from_bucket(&self, bucket: usize) -> Option<String> {
        if let Some(mut entry) = self.pool.get_mut(&bucket) {
            entry.pop_front()
        } else {
            None
        }
    }

    /// Acquire a string buffer from the pool, or allocate a new one if pool is exhausted.
    ///
    /// The returned buffer is automatically returned to the pool when dropped.
    /// Must be called with the pool wrapped in Arc.
    pub fn acquire(self: Arc<Self>) -> PooledString {
        #[cfg(feature = "pool-metrics")]
        self.acquire_count.fetch_add(1, Ordering::Relaxed);

        let default_bucket = self.config.initial_capacity;
        if let Some(buffer) = self.try_acquire_from_bucket(default_bucket) {
            #[cfg(feature = "pool-metrics")]
            self.reuse_count.fetch_add(1, Ordering::Relaxed);
            return PooledString { buffer, pool: self };
        }

        for &bucket in &[1024, 16384, 65536] {
            if let Some(buffer) = self.try_acquire_from_bucket(bucket) {
                #[cfg(feature = "pool-metrics")]
                self.reuse_count.fetch_add(1, Ordering::Relaxed);
                return PooledString { buffer, pool: self };
            }
        }

        PooledString {
            buffer: String::with_capacity(self.config.initial_capacity),
            pool: self,
        }
    }

    /// Return a buffer to the pool for reuse.
    pub fn release(&self, mut buffer: String) {
        if buffer.capacity() > self.config.max_capacity_before_discard {
            return;
        }

        let bucket = self.find_bucket(buffer.capacity());
        buffer.clear();

        if let Some(mut queue) = self.pool.get_mut(&bucket) {
            if queue.len() < self.config.max_buffers_per_size {
                queue.push_back(buffer);
            }
        } else {
            let mut queue = VecDeque::with_capacity(self.config.max_buffers_per_size);
            queue.push_back(buffer);
            self.pool.insert(bucket, queue);
        }
    }

    /// Get the current pool size across all buckets.
    #[allow(dead_code)]
    pub fn size(&self) -> usize {
        self.pool.iter().map(|entry| entry.value().len()).sum()
    }

    /// Get buffer reuse metrics (only available with `pool-metrics` feature).
    #[cfg(feature = "pool-metrics")]
    pub fn metrics(&self) -> StringBufferPoolMetrics {
        let acquire = self.acquire_count.load(Ordering::Relaxed);
        let reuse = self.reuse_count.load(Ordering::Relaxed);
        let hit_rate = if acquire == 0 {
            0.0
        } else {
            (reuse as f64 / acquire as f64) * 100.0
        };

        StringBufferPoolMetrics {
            total_acquires: acquire,
            total_reuses: reuse,
            hit_rate,
        }
    }
}

/// Metrics for StringBufferPool (only available with `pool-metrics` feature).
#[cfg(feature = "pool-metrics")]
#[derive(Debug, Clone, Copy)]
pub struct StringBufferPoolMetrics {
    /// Total number of acquire calls
    pub total_acquires: usize,
    /// Total number of buffer reuses from pool
    pub total_reuses: usize,
    /// Hit rate as percentage (0.0-100.0)
    pub hit_rate: f64,
}

/// RAII wrapper for a pooled string buffer.
///
/// Automatically returns the buffer to the pool when dropped.
pub struct PooledString {
    buffer: String,
    pool: Arc<StringBufferPool>,
}

impl PooledString {
    /// Get mutable access to the underlying string buffer.
    pub fn buffer_mut(&mut self) -> &mut String {
        &mut self.buffer
    }

    /// Get immutable access to the underlying string buffer.
    pub fn as_str(&self) -> &str {
        self.buffer.as_str()
    }
}

impl std::ops::Deref for PooledString {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.buffer
    }
}

impl std::ops::DerefMut for PooledString {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.buffer
    }
}

impl Drop for PooledString {
    fn drop(&mut self) {
        let buffer = std::mem::take(&mut self.buffer);
        self.pool.release(buffer);
    }
}

impl std::fmt::Display for PooledString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.buffer)
    }
}

impl std::fmt::Debug for PooledString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("PooledString").field(&self.buffer).finish()
    }
}

/// Global string buffer pool for temporary allocations during extraction.
pub static STRING_BUFFER_POOL: Lazy<Arc<StringBufferPool>> =
    Lazy::new(|| Arc::new(StringBufferPool::new(PoolConfig::default())));

/// Acquire a string buffer from the global pool.
///
/// The returned buffer is automatically returned to the pool when dropped.
///
/// # Example
///
/// ```rust,ignore
/// let mut buffer = acquire_string_buffer();
/// buffer.push_str("content");
/// // Automatically returned to pool when buffer goes out of scope
/// ```
pub fn acquire_string_buffer() -> PooledString {
    Arc::clone(&*STRING_BUFFER_POOL).acquire()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_buffer_pool_acquire_and_release() {
        let config = PoolConfig::default();
        let pool = Arc::new(StringBufferPool::new(config));

        let mut buffer = pool.clone().acquire();
        buffer.push_str("test content");
        let capacity = buffer.capacity();

        drop(buffer);

        let buffer2 = pool.clone().acquire();
        assert_eq!(buffer2.capacity(), capacity);
        assert!(buffer2.is_empty());
    }

    #[test]
    fn test_buffer_pool_size() {
        let config = PoolConfig::default();
        let pool = Arc::new(StringBufferPool::new(config));

        assert_eq!(pool.size(), 0);

        let buffer1 = pool.clone().acquire();
        drop(buffer1);
        assert_eq!(pool.size(), 1);

        let buffer2 = pool.clone().acquire();
        drop(buffer2);
        assert_eq!(pool.size(), 1);
    }

    #[test]
    fn test_buffer_pool_global() {
        let buffer1 = acquire_string_buffer();
        drop(buffer1);

        let buffer2 = acquire_string_buffer();
        assert!(buffer2.capacity() >= 4096);
    }

    #[test]
    fn test_pooled_string_deref() {
        let mut buffer = acquire_string_buffer();
        buffer.push_str("hello");

        assert_eq!(&*buffer, "hello");
        assert_eq!(buffer.as_str(), "hello");
        assert!(!buffer.is_empty());
    }

    #[test]
    fn test_pooled_string_deref_mut() {
        let mut buffer = acquire_string_buffer();
        buffer.push_str("test");

        buffer.buffer_mut().push_str(" more");
        assert_eq!(buffer.as_str(), "test more");
    }
}
