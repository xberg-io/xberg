/**
 * Worker Memory Tests
 *
 * Tests for WASM worker pool memory management including pooling behavior,
 * shared memory patterns, memory isolation, and efficient resource utilization.
 *
 * @group worker-pool
 */

import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

/**
 * Memory statistics for a worker
 */
interface MemoryStats {
	estimated: number;
	allocations: number;
	deallocations: number;
	leaked: number;
}

/**
 * Shared memory region for inter-worker communication
 */
interface SharedMemoryBuffer {
	buffer: ArrayBuffer;
	views: Map<string, ArrayBufferView>;
	accessLog: Array<{ worker: string; type: string; timestamp: number }>;
}

/**
 * Memory-aware mock worker
 */
class MemoryAwareWorker {
	private memoryStats: MemoryStats = {
		estimated: 0,
		allocations: 0,
		deallocations: 0,
		leaked: 0,
	};
	private buffers: Set<ArrayBuffer> = new Set();
	private sharedBuffers: Map<string, SharedMemoryBuffer> = new Map();
	private terminated = false;

	allocateBuffer(size: number): ArrayBuffer {
		if (this.terminated) {
			throw new Error("Cannot allocate: worker is terminated");
		}

		const buffer = new ArrayBuffer(size);
		this.buffers.add(buffer);
		this.memoryStats.allocations++;
		this.memoryStats.estimated += size;

		return buffer;
	}

	deallocateBuffer(buffer: ArrayBuffer): boolean {
		if (!this.buffers.has(buffer)) {
			return false;
		}

		this.buffers.delete(buffer);
		this.memoryStats.deallocations++;
		this.memoryStats.estimated -= buffer.byteLength;

		return true;
	}

	registerSharedBuffer(name: string, buffer: ArrayBuffer): void {
		if (this.terminated) {
			throw new Error("Cannot register: worker is terminated");
		}

		if (!this.sharedBuffers.has(name)) {
			this.sharedBuffers.set(name, {
				buffer,
				views: new Map(),
				accessLog: [],
			});
		}
	}

	createView(bufferName: string, viewType: string, offset: number, length: number): ArrayBufferView {
		const shared = this.sharedBuffers.get(bufferName);
		if (!shared) {
			throw new Error(`Shared buffer not found: ${bufferName}`);
		}

		let view: ArrayBufferView;

		switch (viewType) {
			case "uint8":
				view = new Uint8Array(shared.buffer, offset, length);
				break;
			case "int32":
				view = new Int32Array(shared.buffer, offset, length);
				break;
			case "float64":
				view = new Float64Array(shared.buffer, offset, length);
				break;
			default:
				throw new Error(`Unknown view type: ${viewType}`);
		}

		const viewKey = `${bufferName}-${viewType}-${offset}`;
		shared.views.set(viewKey, view);
		shared.accessLog.push({
			worker: this.constructor.name,
			type: "view_created",
			timestamp: Date.now(),
		});

		return view;
	}

	accessSharedBuffer(bufferName: string, operation: string): void {
		const shared = this.sharedBuffers.get(bufferName);
		if (!shared) {
			throw new Error(`Shared buffer not found: ${bufferName}`);
		}

		shared.accessLog.push({
			worker: this.constructor.name,
			type: operation,
			timestamp: Date.now(),
		});
	}

	getMemoryStats(): Readonly<MemoryStats> {
		const stats = { ...this.memoryStats };
		stats.leaked = stats.allocations - stats.deallocations;
		return Object.freeze(stats);
	}

	getBufferCount(): number {
		return this.buffers.size;
	}

	getSharedBufferCount(): number {
		return this.sharedBuffers.size;
	}

	getAccessLog(bufferName: string): Array<{
		worker: string;
		type: string;
		timestamp: number;
	}> {
		const shared = this.sharedBuffers.get(bufferName);
		return shared ? [...shared.accessLog] : [];
	}

	terminate(): void {
		this.terminated = true;
		this.buffers.clear();
		this.sharedBuffers.clear();
		this.memoryStats.estimated = 0;
	}

	isTerminated(): boolean {
		return this.terminated;
	}
}

/**
 * Worker pool with memory management
 */
class MemoryPoolManager {
	private workers: Map<string, MemoryAwareWorker> = new Map();
	private poolSize: number;
	private sharedBuffers: Map<string, SharedMemoryBuffer> = new Map();
	private nextWorkerId = 0;

	constructor(size: number) {
		this.poolSize = size;
	}

	initializePool(): string[] {
		const workerIds: string[] = [];

		for (let i = 0; i < this.poolSize; i++) {
			const id = `worker-${this.nextWorkerId++}`;
			const worker = new MemoryAwareWorker();
			this.workers.set(id, worker);
			workerIds.push(id);
		}

		return workerIds;
	}

	getWorker(id: string): MemoryAwareWorker | undefined {
		return this.workers.get(id);
	}

	getTotalMemoryUsage(): number {
		let total = 0;
		for (const worker of this.workers.values()) {
			total += worker.getMemoryStats().estimated;
		}
		return total;
	}

	getPoolMemoryStats() {
		let totalAllocations = 0;
		let totalDeallocations = 0;
		let totalLeaked = 0;
		let activeBuffers = 0;

		for (const worker of this.workers.values()) {
			const stats = worker.getMemoryStats();
			totalAllocations += stats.allocations;
			totalDeallocations += stats.deallocations;
			totalLeaked += stats.leaked;
			activeBuffers += worker.getBufferCount();
		}

		return {
			totalAllocations,
			totalDeallocations,
			totalLeaked,
			activeBuffers,
			totalMemory: this.getTotalMemoryUsage(),
		};
	}

	createSharedBuffer(name: string, size: number): void {
		const buffer = new ArrayBuffer(size);
		this.sharedBuffers.set(name, {
			buffer,
			views: new Map(),
			accessLog: [],
		});

		// Register with all workers
		for (const worker of this.workers.values()) {
			worker.registerSharedBuffer(name, buffer);
		}
	}

	getSharedBufferAccessLog(name: string): Array<{
		worker: string;
		type: string;
		timestamp: number;
	}> {
		const accessLog: Array<{
			worker: string;
			type: string;
			timestamp: number;
		}> = [];

		for (const worker of this.workers.values()) {
			accessLog.push(...worker.getAccessLog(name));
		}

		return accessLog.sort((a, b) => a.timestamp - b.timestamp);
	}

	terminatePool(): void {
		for (const worker of this.workers.values()) {
			worker.terminate();
		}
		this.workers.clear();
		this.sharedBuffers.clear();
	}
}

describe("Worker Memory Management", () => {
	let poolManager: MemoryPoolManager;
	let workerIds: string[] = [];

	beforeEach(() => {
		poolManager = new MemoryPoolManager(3);
		workerIds = poolManager.initializePool();
		vi.clearAllMocks();
	});

	afterEach(() => {
		poolManager.terminatePool();
		workerIds = [];
	});

	describe("Memory Allocation", () => {
		it("should allocate buffers in workers", () => {
			const worker = poolManager.getWorker(workerIds[0])!;

			const buffer = worker.allocateBuffer(1024);

			expect(buffer).toBeInstanceOf(ArrayBuffer);
			expect(buffer.byteLength).toBe(1024);
		});

		it("should track allocation count", () => {
			const worker = poolManager.getWorker(workerIds[0])!;

			worker.allocateBuffer(512);
			worker.allocateBuffer(1024);
			worker.allocateBuffer(2048);

			const stats = worker.getMemoryStats();
			expect(stats.allocations).toBe(3);
		});

		it("should calculate total memory usage", () => {
			const worker = poolManager.getWorker(workerIds[0])!;

			worker.allocateBuffer(1024);
			worker.allocateBuffer(512);

			const stats = worker.getMemoryStats();
			expect(stats.estimated).toBe(1536);
		});

		it("should prevent allocation on terminated worker", () => {
			const worker = poolManager.getWorker(workerIds[0])!;

			worker.terminate();

			expect(() => {
				worker.allocateBuffer(1024);
			}).toThrow("Cannot allocate: worker is terminated");
		});

		it("should track buffer count", () => {
			const worker = poolManager.getWorker(workerIds[0])!;

			worker.allocateBuffer(512);
			worker.allocateBuffer(1024);

			expect(worker.getBufferCount()).toBe(2);
		});

		it("should handle large buffer allocation", () => {
			const worker = poolManager.getWorker(workerIds[0])!;
			const largeSize = 10 * 1024 * 1024; // 10MB

			const buffer = worker.allocateBuffer(largeSize);

			expect(buffer.byteLength).toBe(largeSize);
			expect(worker.getMemoryStats().estimated).toBe(largeSize);
		});
	});

	describe("Memory Deallocation", () => {
		it("should deallocate buffers correctly", () => {
			const worker = poolManager.getWorker(workerIds[0])!;

			const buffer = worker.allocateBuffer(1024);
			expect(worker.getBufferCount()).toBe(1);

			const deallocated = worker.deallocateBuffer(buffer);

			expect(deallocated).toBe(true);
			expect(worker.getBufferCount()).toBe(0);
			expect(worker.getMemoryStats().estimated).toBe(0);
		});

		it("should track deallocation count accurately", () => {
			const worker = poolManager.getWorker(workerIds[0])!;

			const buf1 = worker.allocateBuffer(512);
			const buf2 = worker.allocateBuffer(512);

			worker.deallocateBuffer(buf1);
			worker.deallocateBuffer(buf2);

			const stats = worker.getMemoryStats();
			expect(stats.deallocations).toBe(2);
			expect(stats.allocations).toBe(2);
			expect(stats.leaked).toBe(0);
		});

		it("should reduce memory usage on deallocation", () => {
			const worker = poolManager.getWorker(workerIds[0])!;

			const buf1 = worker.allocateBuffer(1024);
			const buf2 = worker.allocateBuffer(512);
			expect(worker.getMemoryStats().estimated).toBe(1536);

			worker.deallocateBuffer(buf1);
			expect(worker.getMemoryStats().estimated).toBe(512);

			worker.deallocateBuffer(buf2);
			expect(worker.getMemoryStats().estimated).toBe(0);
		});

		it("should handle deallocation of non-existent buffers safely", () => {
			const worker = poolManager.getWorker(workerIds[0])!;

			const buf1 = worker.allocateBuffer(512);
			const unknownBuffer = new ArrayBuffer(512);

			const deallocResult1 = worker.deallocateBuffer(buf1);
			const deallocResult2 = worker.deallocateBuffer(unknownBuffer);

			// First deallocation should succeed
			expect(deallocResult1).toBe(true);
			// Second should fail (buffer doesn't belong to this worker)
			expect(deallocResult2).toBe(false);

			// Memory should only account for successful deallocation
			expect(worker.getMemoryStats().estimated).toBe(0);
		});

		it("should detect memory leaks by tracking allocated vs deallocated", () => {
			const worker = poolManager.getWorker(workerIds[0])!;

			const buf1 = worker.allocateBuffer(512);
			const buf2 = worker.allocateBuffer(512);
			const buf3 = worker.allocateBuffer(512);
			const buf4 = worker.allocateBuffer(512);

			// Only deallocate one buffer
			worker.deallocateBuffer(buf1);

			const stats = worker.getMemoryStats();
			// 4 allocations, 1 deallocation = 3 leaked
			expect(stats.allocations).toBe(4);
			expect(stats.deallocations).toBe(1);
			expect(stats.leaked).toBe(3);
			expect(stats.estimated).toBe(1536); // 3 remaining buffers * 512
		});
	});

	describe("Shared Memory Patterns", () => {
		it("should register shared buffer across all pool workers", () => {
			poolManager.createSharedBuffer("shared-data", 4096);

			// Verify all workers have the shared buffer
			for (const id of workerIds) {
				const worker = poolManager.getWorker(id)!;
				expect(worker.getSharedBufferCount()).toBe(1);
			}

			// And can access it without error
			for (const id of workerIds) {
				const worker = poolManager.getWorker(id)!;
				expect(() => {
					worker.accessSharedBuffer("shared-data", "read");
				}).not.toThrow();
			}
		});

		it("should support multiple workers accessing same shared buffer concurrently", () => {
			poolManager.createSharedBuffer("shared-data", 1024);

			const worker1 = poolManager.getWorker(workerIds[0])!;
			const worker2 = poolManager.getWorker(workerIds[1])!;
			const worker3 = poolManager.getWorker(workerIds[2])!;

			// Multiple workers should be able to access simultaneously
			expect(() => {
				worker1.accessSharedBuffer("shared-data", "read");
				worker2.accessSharedBuffer("shared-data", "write");
				worker3.accessSharedBuffer("shared-data", "read");
			}).not.toThrow();
		});

		it("should create typed views with correct data types", () => {
			poolManager.createSharedBuffer("shared-data", 1024);
			const worker = poolManager.getWorker(workerIds[0])!;

			const uint8View = worker.createView("shared-data", "uint8", 0, 64);
			const int32View = worker.createView("shared-data", "int32", 64, 32);
			const float64View = worker.createView("shared-data", "float64", 192, 8);

			// Verify correct types
			expect(uint8View).toBeInstanceOf(Uint8Array);
			expect(int32View).toBeInstanceOf(Int32Array);
			expect(float64View).toBeInstanceOf(Float64Array);

			// Verify they reference the same underlying buffer
			expect(uint8View.buffer).toBe(int32View.buffer);
			expect(int32View.buffer).toBe(float64View.buffer);
		});

		it("should track access patterns to identify contention", () => {
			poolManager.createSharedBuffer("shared-data", 1024);

			const worker1 = poolManager.getWorker(workerIds[0])!;
			const worker2 = poolManager.getWorker(workerIds[1])!;

			worker1.accessSharedBuffer("shared-data", "write");
			worker2.accessSharedBuffer("shared-data", "read");
			worker1.accessSharedBuffer("shared-data", "read");

			const accessLog = poolManager.getSharedBufferAccessLog("shared-data");

			// Should have 3 access records
			expect(accessLog.length).toBe(3);

			// Pattern should be write, read, read
			expect(accessLog[0].type).toBe("write");
			expect(accessLog[1].type).toBe("read");
			expect(accessLog[2].type).toBe("read");
		});

		it("should maintain chronological access order", () => {
			poolManager.createSharedBuffer("data", 1024);

			const worker1 = poolManager.getWorker(workerIds[0])!;
			const worker2 = poolManager.getWorker(workerIds[1])!;

			// Perform multiple accesses
			worker1.accessSharedBuffer("data", "write");
			worker2.accessSharedBuffer("data", "read");
			worker1.accessSharedBuffer("data", "write");

			const accessLog = poolManager.getSharedBufferAccessLog("data");

			// Should have 3 access records
			expect(accessLog.length).toBe(3);

			// Verify chronological ordering (timestamps should not decrease)
			for (let i = 1; i < accessLog.length; i++) {
				expect(accessLog[i].timestamp).toBeGreaterThanOrEqual(accessLog[i - 1].timestamp);
			}

			// Verify we have both read and write operations
			const writeCount = accessLog.filter((a) => a.type === "write").length;
			const readCount = accessLog.filter((a) => a.type === "read").length;
			expect(writeCount).toBe(2);
			expect(readCount).toBe(1);
		});

		it("should support multiple independent shared buffers", () => {
			poolManager.createSharedBuffer("buffer-a", 512);
			poolManager.createSharedBuffer("buffer-b", 1024);
			poolManager.createSharedBuffer("buffer-c", 2048);

			const worker = poolManager.getWorker(workerIds[0])!;

			// Worker should have all 3 buffers
			expect(worker.getSharedBufferCount()).toBe(3);

			// Should be able to access all independently
			expect(() => {
				worker.accessSharedBuffer("buffer-a", "read");
				worker.accessSharedBuffer("buffer-b", "write");
				worker.accessSharedBuffer("buffer-c", "read");
			}).not.toThrow();
		});
	});

	describe("Memory Pooling Behavior", () => {
		it("should track pool-wide memory usage", () => {
			const worker1 = poolManager.getWorker(workerIds[0])!;
			const worker2 = poolManager.getWorker(workerIds[1])!;

			worker1.allocateBuffer(1024);
			worker2.allocateBuffer(2048);

			const totalMemory = poolManager.getTotalMemoryUsage();
			expect(totalMemory).toBe(3072);
		});

		it("should provide pool memory statistics", () => {
			const worker1 = poolManager.getWorker(workerIds[0])!;
			const worker2 = poolManager.getWorker(workerIds[1])!;

			worker1.allocateBuffer(512);
			worker1.allocateBuffer(512);
			worker2.allocateBuffer(1024);

			const stats = poolManager.getPoolMemoryStats();

			expect(stats.totalAllocations).toBe(3);
			expect(stats.activeBuffers).toBe(3);
			expect(stats.totalMemory).toBe(2048);
		});

		it("should calculate pool-wide leak detection", () => {
			const worker1 = poolManager.getWorker(workerIds[0])!;
			const worker2 = poolManager.getWorker(workerIds[1])!;

			const buf1 = worker1.allocateBuffer(512);
			const buf2 = worker1.allocateBuffer(512);
			const buf3 = worker2.allocateBuffer(1024);

			worker1.deallocateBuffer(buf1);
			// buf2 and buf3 are not deallocated

			const stats = poolManager.getPoolMemoryStats();
			expect(stats.totalLeaked).toBe(2);
		});

		it("should track buffer distribution across pool", () => {
			const worker1 = poolManager.getWorker(workerIds[0])!;
			const worker2 = poolManager.getWorker(workerIds[1])!;
			const worker3 = poolManager.getWorker(workerIds[2])!;

			worker1.allocateBuffer(512);
			worker1.allocateBuffer(512);
			worker2.allocateBuffer(1024);
			worker3.allocateBuffer(2048);

			const stats = poolManager.getPoolMemoryStats();
			expect(stats.activeBuffers).toBe(4);
			expect(stats.totalAllocations).toBe(4);
		});
	});

	describe("Memory Isolation", () => {
		it("should keep worker buffers completely isolated", () => {
			const worker1 = poolManager.getWorker(workerIds[0])!;
			const worker2 = poolManager.getWorker(workerIds[1])!;

			const buf1 = worker1.allocateBuffer(1024);
			const buf2 = worker2.allocateBuffer(1024);

			// Different buffers
			expect(buf1).not.toBe(buf2);
			expect(buf1.byteLength).toBe(buf2.byteLength);

			// Each worker sees only its own buffer
			expect(worker1.getBufferCount()).toBe(1);
			expect(worker2.getBufferCount()).toBe(1);
		});

		it("should prevent cross-worker buffer manipulation", () => {
			const worker1 = poolManager.getWorker(workerIds[0])!;
			const worker2 = poolManager.getWorker(workerIds[1])!;

			const buffer = worker1.allocateBuffer(512);
			expect(worker1.getBufferCount()).toBe(1);

			// Worker2 cannot deallocate worker1's buffer
			const result = worker2.deallocateBuffer(buffer);
			expect(result).toBe(false);

			// Buffer must still exist in worker1
			expect(worker1.getBufferCount()).toBe(1);
			expect(worker1.getMemoryStats().estimated).toBe(512);

			// Only worker1 can deallocate its own buffer
			const result2 = worker1.deallocateBuffer(buffer);
			expect(result2).toBe(true);
			expect(worker1.getBufferCount()).toBe(0);
		});

		it("should maintain independent allocation/deallocation stats per worker", () => {
			const worker1 = poolManager.getWorker(workerIds[0])!;
			const worker2 = poolManager.getWorker(workerIds[1])!;

			// Worker1: allocate 1024
			worker1.allocateBuffer(1024);
			// Worker2: allocate 2048
			worker2.allocateBuffer(2048);

			const stats1 = worker1.getMemoryStats();
			const stats2 = worker2.getMemoryStats();

			// Each worker should have independent stats
			expect(stats1.allocations).toBe(1);
			expect(stats2.allocations).toBe(1);

			expect(stats1.estimated).toBe(1024);
			expect(stats2.estimated).toBe(2048);

			expect(stats1.deallocations).toBe(0);
			expect(stats2.deallocations).toBe(0);
		});

		it("should not allow one worker to affect another's memory statistics", () => {
			const worker1 = poolManager.getWorker(workerIds[0])!;
			const worker2 = poolManager.getWorker(workerIds[1])!;

			// Worker1 allocates and deallocates
			const buf1a = worker1.allocateBuffer(256);
			const buf1b = worker1.allocateBuffer(256);
			worker1.deallocateBuffer(buf1a);

			// Worker2 allocates and deallocates
			const buf2a = worker2.allocateBuffer(512);
			worker2.deallocateBuffer(buf2a);

			const stats1 = worker1.getMemoryStats();
			const stats2 = worker2.getMemoryStats();

			// Worker1: 2 allocs, 1 dealloc, 1 leaked, 256 bytes remaining
			expect(stats1.allocations).toBe(2);
			expect(stats1.deallocations).toBe(1);
			expect(stats1.leaked).toBe(1);
			expect(stats1.estimated).toBe(256);

			// Worker2: 1 alloc, 1 dealloc, 0 leaked, 0 bytes remaining
			expect(stats2.allocations).toBe(1);
			expect(stats2.deallocations).toBe(1);
			expect(stats2.leaked).toBe(0);
			expect(stats2.estimated).toBe(0);
		});
	});

	describe("Cleanup and Termination", () => {
		it("should clear buffers on worker termination", () => {
			const worker = poolManager.getWorker(workerIds[0])!;

			worker.allocateBuffer(512);
			worker.allocateBuffer(1024);
			expect(worker.getBufferCount()).toBe(2);

			worker.terminate();

			expect(worker.getBufferCount()).toBe(0);
		});

		it("should clear shared memory on pool termination", () => {
			poolManager.createSharedBuffer("shared", 1024);

			const worker = poolManager.getWorker(workerIds[0])!;
			expect(worker.getSharedBufferCount()).toBe(1);

			poolManager.terminatePool();

			expect(worker.getSharedBufferCount()).toBe(0);
		});

		it("should reflect termination in pool stats", () => {
			const worker1 = poolManager.getWorker(workerIds[0])!;
			const worker2 = poolManager.getWorker(workerIds[1])!;

			worker1.allocateBuffer(1024);
			worker2.allocateBuffer(1024);

			let stats = poolManager.getPoolMemoryStats();
			expect(stats.totalMemory).toBe(2048);

			worker1.terminate();

			stats = poolManager.getPoolMemoryStats();
			expect(stats.totalMemory).toBe(1024);
		});
	});
});
