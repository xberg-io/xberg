//! Benchmark tests for object pooling in batch extraction.
//!
//! This test suite demonstrates the performance benefits of object pooling
//! during batch document extraction operations.

#[cfg(feature = "tokio-runtime")]
mod tests {
    use kreuzberg::core::{BatchProcessor, ExtractionConfig};
    use kreuzberg::utils::pool::create_string_buffer_pool;

    #[test]
    fn test_batch_processor_initialization() {
        let processor = BatchProcessor::new();
        assert_eq!(processor.string_pool_size(), 0);
        assert_eq!(processor.byte_pool_size(), 0);
    }

    #[test]
    fn test_string_pool_reuse_reduces_allocations() {
        let pool = create_string_buffer_pool(5, 8192);

        let mut buffers = vec![];
        for _ in 0..3 {
            let buf = pool.acquire().unwrap();
            buffers.push(buf);
        }

        drop(buffers);

        assert_eq!(pool.size(), 3, "pool should have 3 buffers after first batch");

        let mut buffers = vec![];
        for _ in 0..3 {
            let buf = pool.acquire().unwrap();
            buffers.push(buf);
        }
        drop(buffers);

        assert!(pool.size() <= 5, "pool should not exceed max size");
    }

    #[test]
    fn test_batch_processor_multiple_operations() {
        let processor = BatchProcessor::new();

        for _batch in 0..3 {
            let mut results = vec![];

            for _i in 0..5 {
                let string_buf = processor.string_pool().acquire().unwrap();
                let byte_buf = processor.byte_pool().acquire().unwrap();

                results.push((string_buf, byte_buf));
            }

            drop(results);

            assert!(processor.string_pool_size() <= 10);
            assert!(processor.byte_pool_size() <= 10);
        }
    }

    #[test]
    fn test_pool_memory_efficiency() {
        let pool = create_string_buffer_pool(5, 4096);

        let capacity_initial = {
            let buf = pool.acquire().unwrap();
            buf.capacity()
        };

        for _ in 0..10 {
            let mut buf = pool.acquire().unwrap();
            buf.push_str("test data");
        }

        let capacity_final = {
            let buf = pool.acquire().unwrap();
            buf.capacity()
        };

        assert_eq!(
            capacity_initial, capacity_final,
            "buffer capacity should be maintained across reuses"
        );
    }

    #[tokio::test]
    async fn test_batch_processor_with_extraction_config() {
        let processor = BatchProcessor::new();
        let _config = ExtractionConfig::default();

        assert!(processor.config().string_pool_size > 0);
        assert!(processor.config().string_buffer_capacity > 0);
        assert!(processor.config().byte_pool_size > 0);
        assert!(processor.config().byte_buffer_capacity > 0);
    }

    #[test]
    fn test_pool_clear_resets_size() {
        let processor = BatchProcessor::new();

        {
            let _s1 = processor.string_pool().acquire().unwrap();
            let _s2 = processor.string_pool().acquire().unwrap();
            let _b1 = processor.byte_pool().acquire().unwrap();
        }

        assert!(processor.string_pool_size() > 0);
        assert!(processor.byte_pool_size() > 0);

        processor.clear_pools().unwrap();

        assert_eq!(processor.string_pool_size(), 0);
        assert_eq!(processor.byte_pool_size(), 0);
    }

    #[test]
    fn test_concurrent_pool_access() {
        use std::sync::Arc;
        use std::thread;

        let processor = Arc::new(BatchProcessor::new());
        let mut handles = vec![];

        for _thread_id in 0..4 {
            let processor_clone = Arc::clone(&processor);

            let handle = thread::spawn(move || {
                for _ in 0..5 {
                    let _buf1 = processor_clone.string_pool().acquire();
                    let _buf2 = processor_clone.byte_pool().acquire();
                }
            });

            handles.push(handle);
        }

        for handle in handles {
            handle.join().unwrap();
        }

        assert!(processor.string_pool_size() <= 10);
        assert!(processor.byte_pool_size() <= 10);
    }

    #[test]
    fn test_pool_respects_capacity_hints() {
        let pool = create_string_buffer_pool(3, 2048);

        let buf = pool.acquire().unwrap();
        assert!(buf.capacity() >= 2048, "buffer should respect capacity hint");
    }
}
