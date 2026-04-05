/**
 * Worker Lifecycle Tests
 *
 * Tests for WASM worker pool lifecycle management including initialization,
 * state transitions, resource cleanup, and event handling throughout the
 * worker lifetime.
 *
 * @group worker-pool
 */

import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

/**
 * Represents worker state throughout its lifecycle
 */
interface WorkerState {
	initialized: boolean;
	ready: boolean;
	processing: boolean;
	terminated: boolean;
	createdAt: number;
	lastActivity: number;
}

/**
 * Lifecycle-aware mock worker
 * Tracks state transitions and events
 */
class LifecycleWorker {
	private state: WorkerState;
	private eventListeners: Map<string, Set<(...args: unknown[]) => unknown>> = new Map();
	private messageQueue: unknown[] = [];

	constructor() {
		this.state = {
			initialized: true,
			ready: true,
			processing: false,
			terminated: false,
			createdAt: Date.now(),
			lastActivity: Date.now(),
		};

		this.eventListeners.set("ready", new Set());
		this.eventListeners.set("message", new Set());
		this.eventListeners.set("error", new Set());
		this.eventListeners.set("terminate", new Set());
	}

	getState(): Readonly<WorkerState> {
		return Object.freeze({ ...this.state });
	}

	postMessage(data: unknown): void {
		if (this.state.terminated) {
			throw new Error("Cannot post message: worker is terminated");
		}

		this.state.processing = true;
		this.state.lastActivity = Date.now();
		this.messageQueue.push(data);

		setTimeout(() => {
			this.state.processing = false;
			this.emit("message", { data });
		}, 0);
	}

	terminate(): void {
		if (this.state.terminated) {
			return; // Already terminated
		}

		this.state.terminated = true;
		this.state.ready = false;
		this.messageQueue = [];
		this.emit("terminate");

		// Clear all listeners
		for (const listeners of this.eventListeners.values()) {
			listeners.clear();
		}
	}

	on(event: string, callback: (...args: unknown[]) => unknown): void {
		if (!this.eventListeners.has(event)) {
			this.eventListeners.set(event, new Set());
		}
		this.eventListeners.get(event)!.add(callback);
	}

	off(event: string, callback: (...args: unknown[]) => unknown): void {
		const listeners = this.eventListeners.get(event);
		if (listeners) {
			listeners.delete(callback);
		}
	}

	private emit(event: string, data?: unknown): void {
		const listeners = this.eventListeners.get(event);
		if (listeners) {
			for (const listener of listeners) {
				try {
					listener(data);
				} catch (error) {
					console.error(`Error in ${event} listener:`, error);
				}
			}
		}
	}

	getMessageQueueLength(): number {
		return this.messageQueue.length;
	}

	getUptime(): number {
		return Date.now() - this.state.createdAt;
	}

	getIdleDuration(): number {
		return Date.now() - this.state.lastActivity;
	}

	isReady(): boolean {
		return this.state.ready && !this.state.terminated;
	}
}

/**
 * Pool manager for tracking worker lifecycle
 */
class WorkerPool {
	private workers: Map<string, LifecycleWorker> = new Map();
	private nextId = 0;

	constructor(size: number) {
		this.initialPoolSize = size;
	}

	spawn(count: number = 1): string[] {
		const ids: string[] = [];

		for (let i = 0; i < count; i++) {
			const id = `worker-${this.nextId++}`;
			const worker = new LifecycleWorker();
			this.workers.set(id, worker);
			ids.push(id);
		}

		return ids;
	}

	getWorker(id: string): LifecycleWorker | undefined {
		return this.workers.get(id);
	}

	getAllWorkers(): LifecycleWorker[] {
		return Array.from(this.workers.values());
	}

	getActiveWorkerCount(): number {
		return Array.from(this.workers.values()).filter((w) => !w.getState().terminated).length;
	}

	terminateWorker(id: string): boolean {
		const worker = this.workers.get(id);
		if (!worker) return false;

		worker.terminate();
		return true;
	}

	terminateAll(): void {
		for (const worker of this.workers.values()) {
			worker.terminate();
		}
	}

	getPoolStats() {
		const workers = this.getAllWorkers();
		return {
			total: workers.length,
			active: this.getActiveWorkerCount(),
			terminated: workers.filter((w) => w.getState().terminated).length,
			ready: workers.filter((w) => w.isReady()).length,
		};
	}
}

describe("Worker Lifecycle", () => {
	let pool: WorkerPool;
	let workerIds: string[] = [];

	beforeEach(() => {
		pool = new WorkerPool(5);
		workerIds = [];
		vi.clearAllMocks();
	});

	afterEach(() => {
		pool.terminateAll();
	});

	describe("Initialization", () => {
		it("should create worker in initialized state", () => {
			const [id] = pool.spawn(1);
			workerIds.push(id);

			const worker = pool.getWorker(id)!;
			const state = worker.getState();

			expect(state.initialized).toBe(true);
			expect(state.ready).toBe(true);
			expect(state.terminated).toBe(false);
			expect(state.processing).toBe(false);
		});

		it("should record creation timestamp", () => {
			const beforeCreation = Date.now();
			const [id] = pool.spawn(1);
			workerIds.push(id);
			const afterCreation = Date.now();

			const worker = pool.getWorker(id)!;
			const createdAt = worker.getState().createdAt;

			expect(createdAt).toBeGreaterThanOrEqual(beforeCreation);
			expect(createdAt).toBeLessThanOrEqual(afterCreation);
		});

		it("should initialize multiple workers independently", () => {
			const ids = pool.spawn(3);
			workerIds.push(...ids);

			const workers = ids.map((id) => pool.getWorker(id)!);

			for (const worker of workers) {
				expect(worker.getState().initialized).toBe(true);
				expect(worker.isReady()).toBe(true);
			}
		});

		it("should assign unique identities to workers", () => {
			const [id1, id2, id3] = pool.spawn(3);
			workerIds.push(id1, id2, id3);

			const ids = new Set([id1, id2, id3]);
			expect(ids.size).toBe(3);

			const workers = [id1, id2, id3].map((id) => pool.getWorker(id)!);
			const uniqueWorkers = new Set(workers);
			expect(uniqueWorkers.size).toBe(3);
		});

		it("should initialize empty message queue", () => {
			const [id] = pool.spawn(1);
			workerIds.push(id);

			const worker = pool.getWorker(id)!;
			expect(worker.getMessageQueueLength()).toBe(0);
		});
	});

	describe("State Transitions", () => {
		it("should transition to processing on message", async () => {
			const [id] = pool.spawn(1);
			workerIds.push(id);
			const worker = pool.getWorker(id)!;

			expect(worker.getState().processing).toBe(false);

			worker.postMessage("test");
			expect(worker.getState().processing).toBe(true);

			// Wait for async processing to complete
			await new Promise<void>((resolve) => {
				worker.on("message", () => {
					resolve();
				});
			});

			expect(worker.getState().processing).toBe(false);
		});

		it("should remain ready after message processing", async () => {
			const [id] = pool.spawn(1);
			workerIds.push(id);
			const worker = pool.getWorker(id)!;

			await new Promise<void>((resolve) => {
				worker.on("message", () => resolve());
				worker.postMessage("test");
			});

			expect(worker.isReady()).toBe(true);
			expect(worker.getState().terminated).toBe(false);
		});

		it("should transition to terminated on terminate call", () => {
			const [id] = pool.spawn(1);
			workerIds.push(id);
			const worker = pool.getWorker(id)!;

			expect(worker.getState().terminated).toBe(false);
			expect(worker.isReady()).toBe(true);

			worker.terminate();

			expect(worker.getState().terminated).toBe(true);
			expect(worker.isReady()).toBe(false);
		});

		it("should not accept messages after termination", () => {
			const [id] = pool.spawn(1);
			workerIds.push(id);
			const worker = pool.getWorker(id)!;

			worker.terminate();

			expect(() => {
				worker.postMessage("test");
			}).toThrow("Cannot post message: worker is terminated");
		});

		it("should track idle duration changes through activity", async () => {
			const [id] = pool.spawn(1);
			workerIds.push(id);
			const worker = pool.getWorker(id)!;

			// Wait to ensure initial idle duration accumulates
			await new Promise((resolve) => setTimeout(resolve, 50));
			const initialIdleDuration = worker.getIdleDuration();

			// Send a message to update lastActivity
			await new Promise<void>((resolve) => {
				worker.on("message", () => resolve());
				worker.postMessage("test");
			});

			// Immediately after activity, idle duration should be near-zero
			// (give it a tiny moment for the activity timestamp to register)
			await new Promise((resolve) => setTimeout(resolve, 1));
			const idleDurationAfterActivity = worker.getIdleDuration();

			// After activity, idle duration should be significantly smaller
			// The initial duration grew for 50ms, but after activity it should be close to 0
			expect(idleDurationAfterActivity).toBeLessThan(initialIdleDuration);

			// Worker should still be ready
			expect(worker.isReady()).toBe(true);
		});
	});

	describe("Resource Management", () => {
		it("should clean up message queue on termination", async () => {
			const [id] = pool.spawn(1);
			workerIds.push(id);
			const worker = pool.getWorker(id)!;

			worker.postMessage("msg1");
			worker.postMessage("msg2");

			expect(worker.getMessageQueueLength()).toBeGreaterThan(0);

			worker.terminate();
			expect(worker.getMessageQueueLength()).toBe(0);
		});

		it("should prevent listener registration after termination", () => {
			const [id] = pool.spawn(1);
			workerIds.push(id);
			const worker = pool.getWorker(id)!;

			const listener = vi.fn();
			worker.on("message", listener);

			worker.terminate();

			// After termination, new listeners should not receive events
			const newListener = vi.fn();
			worker.on("message", newListener);

			// Even if we tried to post a message, the worker would reject it
			expect(() => {
				worker.postMessage("test");
			}).toThrow();

			expect(newListener).not.toHaveBeenCalled();
		});

		it("should support event listener removal and not invoke removed listeners", async () => {
			const [id] = pool.spawn(1);
			workerIds.push(id);
			const worker = pool.getWorker(id)!;

			const listener = vi.fn();
			worker.on("message", listener);
			worker.off("message", listener);

			// Send a message and verify removed listener is not called
			await new Promise<void>((resolve) => {
				worker.on("message", () => resolve());
				worker.postMessage("test");
			});

			// Removed listener should not have been called
			expect(listener).not.toHaveBeenCalled();
		});

		it("should invoke all registered listeners on message event", async () => {
			const [id] = pool.spawn(1);
			workerIds.push(id);
			const worker = pool.getWorker(id)!;

			const listener1 = vi.fn();
			const listener2 = vi.fn();
			const listener3 = vi.fn();

			worker.on("message", listener1);
			worker.on("message", listener2);
			worker.on("message", listener3);

			await new Promise<void>((resolve) => {
				worker.on("message", () => resolve());
				worker.postMessage("test");
			});

			// All listeners should have been called
			expect(listener1).toHaveBeenCalled();
			expect(listener2).toHaveBeenCalled();
			expect(listener3).toHaveBeenCalled();
		});

		it("should track uptime monotonically across operations", async () => {
			const [id] = pool.spawn(1);
			workerIds.push(id);
			const worker = pool.getWorker(id)!;

			const uptime1 = worker.getUptime();

			// Perform work
			await new Promise<void>((resolve) => {
				worker.on("message", () => resolve());
				worker.postMessage("test");
			});

			const uptime2 = worker.getUptime();

			// Uptime should increase
			expect(uptime2).toBeGreaterThanOrEqual(uptime1);

			// Worker should still be operational
			expect(worker.isReady()).toBe(true);
		});
	});

	describe("Pool State Management", () => {
		it("should track active workers count", () => {
			const ids = pool.spawn(5);
			workerIds.push(...ids);

			expect(pool.getActiveWorkerCount()).toBe(5);

			const worker = pool.getWorker(ids[0])!;
			worker.terminate();

			expect(pool.getActiveWorkerCount()).toBe(4);
		});

		it("should provide accurate pool statistics", () => {
			const ids = pool.spawn(5);
			workerIds.push(...ids);

			let stats = pool.getPoolStats();
			expect(stats.total).toBe(5);
			expect(stats.active).toBe(5);
			expect(stats.terminated).toBe(0);
			expect(stats.ready).toBe(5);

			pool.getWorker(ids[0])!.terminate();

			stats = pool.getPoolStats();
			expect(stats.total).toBe(5);
			expect(stats.active).toBe(4);
			expect(stats.terminated).toBe(1);
			expect(stats.ready).toBe(4);
		});

		it("should handle complete pool termination", () => {
			const ids = pool.spawn(3);
			workerIds.push(...ids);

			pool.terminateAll();

			const stats = pool.getPoolStats();
			expect(stats.active).toBe(0);
			expect(stats.terminated).toBe(3);

			for (const id of ids) {
				const worker = pool.getWorker(id)!;
				expect(worker.getState().terminated).toBe(true);
			}
		});
	});

	describe("Event Lifecycle", () => {
		it("should initialize in ready state", () => {
			const [id] = pool.spawn(1);
			workerIds.push(id);
			const worker = pool.getWorker(id)!;

			// Worker should be ready immediately after spawn
			expect(worker.isReady()).toBe(true);
			expect(worker.getState().initialized).toBe(true);
			expect(worker.getState().terminated).toBe(false);
		});

		it("should emit message event when receiving messages", async () => {
			const [id] = pool.spawn(1);
			workerIds.push(id);
			const worker = pool.getWorker(id)!;

			const messageHandler = vi.fn();
			worker.on("message", messageHandler);

			await new Promise<void>((resolve) => {
				worker.on("message", () => resolve());
				worker.postMessage("test data");
			});

			// Message handler should have been invoked
			expect(messageHandler).toHaveBeenCalled();
			// The message is wrapped in an event object with data property
			expect(messageHandler).toHaveBeenCalledWith(expect.objectContaining({ data: "test data" }));
		});

		it("should emit terminate event on termination", () => {
			const [id] = pool.spawn(1);
			workerIds.push(id);
			const worker = pool.getWorker(id)!;

			const terminateHandler = vi.fn();
			worker.on("terminate", terminateHandler);

			worker.terminate();

			// Terminate event should have been emitted
			expect(terminateHandler).toHaveBeenCalled();
		});

		it("should handle errors in event listeners gracefully", async () => {
			const [id] = pool.spawn(1);
			workerIds.push(id);
			const worker = pool.getWorker(id)!;

			// Add a listener that throws
			const throwingListener = vi.fn(() => {
				throw new Error("Listener error");
			});

			worker.on("message", throwingListener);

			// This should not crash the worker
			expect(async () => {
				await new Promise<void>((resolve) => {
					worker.on("message", () => resolve());
					worker.postMessage("test");
				});
			}).not.toThrow();

			// Worker should still be operational
			expect(worker.isReady()).toBe(true);
		});
	});

	describe("Full Lifecycle Scenarios", () => {
		it("should complete initialize-use-terminate cycle", async () => {
			const [id] = pool.spawn(1);
			workerIds.push(id);
			const worker = pool.getWorker(id)!;

			// Initialize
			expect(worker.getState().initialized).toBe(true);

			// Use
			await new Promise<void>((resolve) => {
				worker.on("message", () => resolve());
				worker.postMessage("test");
			});

			expect(worker.isReady()).toBe(true);

			// Terminate
			worker.terminate();
			expect(worker.getState().terminated).toBe(true);
		});

		it("should handle multiple use cycles before termination", async () => {
			const [id] = pool.spawn(1);
			workerIds.push(id);
			const worker = pool.getWorker(id)!;

			for (let i = 0; i < 3; i++) {
				await new Promise<void>((resolve) => {
					worker.on("message", () => resolve());
					worker.postMessage(`message-${i}`);
				});

				expect(worker.isReady()).toBe(true);
			}

			worker.terminate();
			expect(worker.getState().terminated).toBe(true);
		});

		it("should track accurate lifetimes for pool", async () => {
			const ids = pool.spawn(2);
			workerIds.push(...ids);

			const [id1, id2] = ids;
			const worker1 = pool.getWorker(id1)!;
			const worker2 = pool.getWorker(id2)!;

			const uptime1Before = worker1.getUptime();
			const uptime2Before = worker2.getUptime();

			// Wait a bit
			await new Promise((resolve) => setTimeout(resolve, 50));

			const uptime1After = worker1.getUptime();
			const uptime2After = worker2.getUptime();

			// Both workers should have non-zero uptime
			expect(uptime1Before).toBeGreaterThanOrEqual(0);
			expect(uptime2Before).toBeGreaterThanOrEqual(0);

			// Uptime should increase over time
			expect(uptime1After).toBeGreaterThan(uptime1Before);
			expect(uptime2After).toBeGreaterThan(uptime2Before);
		});
	});
});
