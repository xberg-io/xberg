//! Object pooling utilities for reducing allocations in batch processing.
//!
//! This module provides reusable object pools to reduce garbage collection and allocator
//! pressure during batch document extraction. Instead of creating/destroying temporary objects
//! repeatedly, pools maintain a collection of pre-allocated objects that can be quickly reused.
//!
//! # Performance Benefits
//!
//! For batch extraction of N documents:
//! - Without pooling: N allocations + N deallocations per run
//! - With pooling: 1-2 allocations per run (pool creation), zero during document processing
//!
//! Expected improvement: 5-10% overall throughput improvement on batch operations.
//!
//! # Thread Safety
//!
//! All pools are `Send + Sync` and can be safely shared across threads using `Arc<Pool<T>>`.
//! Internal state is protected by `Mutex` for proper synchronization.
//!
//! # Example
//!
//! ```rust,no_run
//! use kreuzberg::utils::pool::StringBuffer;
//!
//! let pool = StringBuffer::new(4096, 10); // 10 buffers of 4KB each
//! let mut buffer = pool.acquire().unwrap();
//! buffer.push_str("some content");
//! // buffer is returned to pool when dropped
//! ```

use std::sync::{Arc, Mutex};

/// A thread-safe object pool that reuses instances to reduce allocations.
///
/// Generic over any type that implements `Recyclable`, allowing pooling of
/// different object types with custom reset logic.
#[derive(Clone)]
pub struct Pool<T: Recyclable> {
    factory: Arc<dyn Fn() -> T + Send + Sync>,
    objects: Arc<Mutex<Vec<T>>>,
    max_size: usize,
}

/// Trait for types that can be pooled and reused.
///
/// Implementing this trait allows a type to be used with `Pool<T>`.
/// The `reset()` method should clear the object's state for reuse.
pub trait Recyclable: Send + 'static {
    /// Reset the object to a reusable state.
    ///
    /// This is called when returning an object to the pool.
    /// Should clear any internal data while preserving capacity.
    fn reset(&mut self);
}

impl<T: Recyclable> Pool<T> {
    /// Create a new pool with a given factory and maximum size.
    ///
    /// # Arguments
    ///
    /// * `factory` - Function that creates new instances
    /// * `max_size` - Maximum number of objects to keep in the pool
    ///
    /// # Returns
    ///
    /// A new pool ready to use.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use kreuzberg::utils::pool::Pool;
    ///
    /// let pool = Pool::new(|| String::new(), 10);
    /// ```
    pub fn new<F>(factory: F, max_size: usize) -> Self
    where
        F: Fn() -> T + Send + Sync + 'static,
    {
        Pool {
            factory: Arc::new(factory),
            objects: Arc::new(Mutex::new(Vec::with_capacity(max_size))),
            max_size,
        }
    }

    /// Acquire an object from the pool or create a new one if empty.
    ///
    /// # Returns
    ///
    /// A `PoolGuard<T>` that will return the object to the pool when dropped.
    ///
    /// # Errors
    ///
    /// Returns `PoolError` if the mutex is poisoned.
    pub fn acquire(&self) -> Result<PoolGuard<T>, PoolError> {
        let mut objects = self.objects.lock().map_err(|_| PoolError::LockPoisoned)?;

        let object = if let Some(mut obj) = objects.pop() {
            obj.reset();
            obj
        } else {
            (self.factory)()
        };

        Ok(PoolGuard {
            object: Some(object),
            pool: Pool {
                factory: Arc::clone(&self.factory),
                objects: Arc::clone(&self.objects),
                max_size: self.max_size,
            },
        })
    }

    /// Get the current number of objects in the pool.
    pub fn size(&self) -> usize {
        self.objects.lock().map(|objs| objs.len()).unwrap_or(0)
    }

    /// Clear the pool, discarding all pooled objects.
    pub fn clear(&self) -> Result<(), PoolError> {
        self.objects.lock().map_err(|_| PoolError::LockPoisoned)?.clear();
        Ok(())
    }
}

/// RAII guard that returns an object to the pool when dropped.
pub struct PoolGuard<T: Recyclable> {
    object: Option<T>,
    pool: Pool<T>,
}

impl<T: Recyclable> std::ops::Deref for PoolGuard<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.object.as_ref().expect("object should never be None")
    }
}

impl<T: Recyclable> std::ops::DerefMut for PoolGuard<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.object.as_mut().expect("object should never be None")
    }
}

impl<T: Recyclable> Drop for PoolGuard<T> {
    fn drop(&mut self) {
        if let Some(mut object) = self.object.take() {
            object.reset();

            if let Ok(mut objects) = self.pool.objects.lock()
                && objects.len() < self.pool.max_size
            {
                objects.push(object);
                // If pool is full, object is dropped and deallocated
            }
            // If lock is poisoned, object is dropped and deallocated
        }
    }
}

/// Pooled string buffer for text accumulation.
///
/// Reuses a single String allocation across multiple uses to reduce allocations
/// when processing multiple documents.
impl Recyclable for String {
    fn reset(&mut self) {
        self.clear();
    }
}

/// Pooled byte vector for binary data handling.
///
/// Reuses a Vec<u8> allocation across multiple uses.
impl Recyclable for Vec<u8> {
    fn reset(&mut self) {
        self.clear();
    }
}

/// Error type for pool operations.
#[derive(Debug, Clone)]
pub enum PoolError {
    /// The pool's internal mutex was poisoned.
    ///
    /// This indicates a panic occurred while holding the lock.
    /// The pool is in a locked state and cannot be recovered.
    LockPoisoned,
}

impl std::fmt::Display for PoolError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PoolError::LockPoisoned => write!(f, "pool lock poisoned"),
        }
    }
}

impl std::error::Error for PoolError {}

/// Convenience type alias for a pooled String.
pub type StringBufferPool = Pool<String>;

/// Convenience type alias for a pooled Vec<u8>.
pub type ByteBufferPool = Pool<Vec<u8>>;

/// Create a pre-configured string buffer pool for batch processing.
///
/// # Arguments
///
/// * `pool_size` - Maximum number of buffers to keep in the pool
/// * `buffer_capacity` - Initial capacity for each buffer in bytes
///
/// # Returns
///
/// A pool configured for text accumulation with reasonable defaults.
///
/// # Example
///
/// ```rust,no_run
/// use kreuzberg::utils::pool::create_string_buffer_pool;
///
/// let pool = create_string_buffer_pool(10, 8192);
/// let mut buffer = pool.acquire().unwrap();
/// buffer.push_str("content");
/// ```
pub fn create_string_buffer_pool(pool_size: usize, buffer_capacity: usize) -> StringBufferPool {
    Pool::new(move || String::with_capacity(buffer_capacity), pool_size)
}

/// Create a pre-configured byte buffer pool for batch processing.
///
/// # Arguments
///
/// * `pool_size` - Maximum number of buffers to keep in the pool
/// * `buffer_capacity` - Initial capacity for each buffer in bytes
///
/// # Returns
///
/// A pool configured for binary data handling with reasonable defaults.
///
/// # Example
///
/// ```rust,no_run
/// use kreuzberg::utils::pool::create_byte_buffer_pool;
///
/// let pool = create_byte_buffer_pool(10, 65536);
/// let mut buffer = pool.acquire().unwrap();
/// buffer.extend_from_slice(b"binary data");
/// ```
pub fn create_byte_buffer_pool(pool_size: usize, buffer_capacity: usize) -> ByteBufferPool {
    Pool::new(move || Vec::with_capacity(buffer_capacity), pool_size)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pool_acquire_and_reuse() {
        let pool = Pool::new(String::new, 5);

        // First acquire creates a new string
        {
            let mut s1 = pool.acquire().unwrap();
            s1.push_str("hello");
            assert_eq!(s1.len(), 5);
        } // s1 returned to pool

        // Second acquire should reuse the same buffer
        {
            let s2 = pool.acquire().unwrap();
            assert_eq!(s2.len(), 0, "string should be cleared when reused");
        }
    }

    #[test]
    fn test_pool_respects_max_size() {
        let pool = Pool::new(String::new, 2);

        let guard1 = pool.acquire().unwrap();
        let guard2 = pool.acquire().unwrap();
        let guard3 = pool.acquire().unwrap();

        drop(guard1);
        drop(guard2);
        drop(guard3);

        // Pool should contain at most 2 items
        assert!(pool.size() <= 2, "pool size should not exceed max_size");
    }

    #[test]
    fn test_pool_clear() {
        let pool = Pool::new(String::new, 5);

        let _g1 = pool.acquire().unwrap();
        let _g2 = pool.acquire().unwrap();

        drop(_g1);
        drop(_g2);

        assert!(pool.size() > 0, "pool should have items");
        pool.clear().unwrap();
        assert_eq!(pool.size(), 0, "pool should be empty after clear");
    }

    #[test]
    fn test_byte_buffer_pool() {
        let pool = Pool::new(Vec::new, 5);

        {
            let mut buf = pool.acquire().unwrap();
            buf.extend_from_slice(b"hello");
            assert_eq!(buf.len(), 5);
        }

        {
            let buf = pool.acquire().unwrap();
            assert_eq!(buf.len(), 0, "buffer should be cleared");
        }
    }

    #[test]
    fn test_pool_deref() {
        let pool = Pool::new(String::new, 5);
        let mut guard = pool.acquire().unwrap();

        // Test Deref
        let _len = guard.len();

        // Test DerefMut
        guard.push_str("test");
        assert_eq!(&*guard, "test");
    }

    #[test]
    fn test_concurrent_access() {
        use std::sync::Arc;
        use std::thread;

        let pool = Arc::new(Pool::new(String::new, 10));
        let mut handles = vec![];

        for i in 0..5 {
            let pool_clone = Arc::clone(&pool);
            let handle = thread::spawn(move || {
                let mut buf = pool_clone.acquire().unwrap();
                buf.push_str(&format!("thread_{}", i));
                std::thread::sleep(std::time::Duration::from_millis(10));
                drop(buf);
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.join().unwrap();
        }

        // All threads completed successfully
        assert!(pool.size() <= 10);
    }
}
