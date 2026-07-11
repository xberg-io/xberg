import { afterEach, describe, expect, it, vi } from "vitest";
import { createBrowserVectorStore } from "./store-browser.js";

class MockWorker {
	static instances: MockWorker[] = [];
	onmessage?: (event: MessageEvent) => void;
	onerror?: (event: ErrorEvent) => void;
	terminated = false;
	failInit = false;

	constructor() {
		MockWorker.instances.push(this);
	}

	postMessage(request: { id: number; op: string }): void {
		queueMicrotask(() => {
			this.onmessage?.({
				data:
					this.failInit && request.op === "init"
						? { id: request.id, ok: false, error: "init failed" }
						: { id: request.id, ok: true },
			} as MessageEvent);
		});
	}

	terminate(): void {
		this.terminated = true;
	}
}

describe("browser vector store worker lifecycle", () => {
	afterEach(() => {
		MockWorker.instances = [];
		vi.unstubAllGlobals();
	});

	it("terminates the worker when initialization fails", async () => {
		class FailingWorker extends MockWorker {
			constructor() {
				super();
				this.failInit = true;
			}
		}
		vi.stubGlobal("Worker", FailingWorker);

		await expect(createBrowserVectorStore()).rejects.toThrow("init failed");
		expect(MockWorker.instances[0]?.terminated).toBe(true);
	});

	it("closes the database before terminating the worker", async () => {
		vi.stubGlobal("Worker", MockWorker);
		const store = await createBrowserVectorStore();

		await store.close();
		expect(MockWorker.instances[0]?.terminated).toBe(true);
	});
});
