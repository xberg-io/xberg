// tests/worker-client.test.ts
import { describe, it, expect, vi } from "vitest";
import { WorkerClient } from "../src/engine/worker-client.js";

class FakeWorker implements Partial<Worker> {
  onmessage: ((ev: MessageEvent) => void) | null = null;
  posted: unknown[] = [];
  postMessage(msg: unknown): void {
    this.posted.push(msg);
  }
  addEventListener(_type: string, listener: EventListenerOrEventListenerObject): void {
    this.onmessage = listener as (ev: MessageEvent) => void;
  }
  removeEventListener(): void {
    this.onmessage = null;
  }
  emit(data: unknown): void {
    this.onmessage?.({ data } as MessageEvent);
  }
}

describe("engine/worker-client", () => {
  it("resolves ingestFile with the final result on a 'result' message", async () => {
    const fake = new FakeWorker();
    const client = new WorkerClient(fake as unknown as Worker, "http://x:8080");
    const file = new File([new Uint8Array([1, 2, 3])], "a.pdf", { type: "application/pdf" });

    const promise = client.ingestFile(file, "c1", "pass1234");
    const sentMsg = fake.posted[0] as { type: string; requestId: string; filename: string; collection: string; mcpBaseUrl: string };
    expect(sentMsg.type).toBe("ingest");
    expect(sentMsg.filename).toBe("a.pdf");
    expect(sentMsg.collection).toBe("c1");
    expect(sentMsg.mcpBaseUrl).toBe("http://x:8080");

    fake.emit({
      type: "result",
      requestId: sentMsg.requestId,
      entry: { collection: "c1", externalId: "a.pdf", filename: "a.pdf", mime: "application/pdf", redactedText: "hi", piiCategoryCounts: {}, documentId: "doc-1", status: "synced", ingestedAt: 1 },
    });

    const result = await promise;
    expect(result.documentId).toBe("doc-1");
  });

  it("rejects ingestFile on an 'error' message", async () => {
    const fake = new FakeWorker();
    const client = new WorkerClient(fake as unknown as Worker, "http://x:8080");
    const file = new File([new Uint8Array([1])], "b.pdf", { type: "application/pdf" });

    const promise = client.ingestFile(file, "c1", "pass1234");
    const sentMsg = fake.posted[0] as { requestId: string };
    fake.emit({ type: "error", requestId: sentMsg.requestId, message: "collection not found: c1" });

    await expect(promise).rejects.toThrow(/collection not found/);
  });

  it("calls onProgress for intermediate 'progress' messages, ignoring other request ids", async () => {
    const fake = new FakeWorker();
    const client = new WorkerClient(fake as unknown as Worker, "http://x:8080");
    const file = new File([new Uint8Array([1])], "c.pdf", { type: "application/pdf" });
    const stages: string[] = [];

    const promise = client.ingestFile(file, "c1", "pass1234", (stage) => stages.push(stage));
    const sentMsg = fake.posted[0] as { requestId: string };
    fake.emit({ type: "progress", requestId: "some-other-request", stage: "extract" });
    fake.emit({ type: "progress", requestId: sentMsg.requestId, stage: "extract" });
    fake.emit({ type: "progress", requestId: sentMsg.requestId, stage: "ingest" });
    fake.emit({
      type: "result",
      requestId: sentMsg.requestId,
      entry: { collection: "c1", externalId: "c.pdf", filename: "c.pdf", mime: "application/pdf", redactedText: "", piiCategoryCounts: {}, documentId: "doc-2", status: "synced", ingestedAt: 1 },
    });
    await promise;

    expect(stages).toEqual(["extract", "ingest"]);
  });
});
