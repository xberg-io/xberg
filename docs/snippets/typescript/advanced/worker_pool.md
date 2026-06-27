# Worker Thread Pool Example

Use worker pools for CPU-bound batch processing to maximize parallelism and throughput:

```typescript title="worker_pool.ts"
import {
  createWorkerPool,
  extractInWorker,
  extractBatchInWorker,
  closeWorkerPool,
} from "@xberg-io/xberg";

// Create a pool with 4 worker threads
const pool = createWorkerPool(4);

try {
  // Extract single file in worker
  const result = await extractInWorker(pool, "document.pdf", null, {
    useCache: true,
  });
  console.log(result.content);

  // Extract multiple files concurrently
  const files = ["doc1.pdf", "doc2.docx", "doc3.xlsx"];
  const results = await extractBatchInWorker(pool, files, {
    useCache: true,
  });

  results.forEach((result, i) => {
    console.log(`File ${i + 1}: ${result.content.length} characters`);
  });
} finally {
  // Always close the pool when done
  await closeWorkerPool(pool);
}
```

**Performance Benefits:**

- **Parallel Processing**: Multiple documents extracted simultaneously
- **CPU Usage**: Maximizes multi-core CPU usage for large batches
- **Queue Management**: Automatically distributes work across available workers
- **Resource Control**: Prevents thread exhaustion with configurable pool size

**Best Practices:**

- Use worker pools for batches of 10+ documents
- Set pool size to number of CPU cores (default behavior)
- Always close pools with `closeWorkerPool()` to prevent resource leaks
- Reuse pools across multiple batch operations for efficiency
