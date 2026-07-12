// tests/providers/EngineProvider.test.tsx
import { describe, it, expect, vi } from "vitest";
import { render, screen, waitFor, act } from "@testing-library/react";
import { EngineProvider, useEngine } from "../../src/providers/EngineProvider.js";
import type { IngestHistoryEntry } from "../../src/lib/types.js";

function Probe({ onReady }: { onReady: (api: ReturnType<typeof useEngine>) => void }) {
  const api = useEngine();
  onReady(api);
  return <div data-testid="pending-count">{api.pendingCount}</div>;
}

describe("providers/EngineProvider", () => {
  it("exposes ingestFile and tracks pendingCount across an in-flight call", async () => {
    const entry: IngestHistoryEntry = {
      collection: "c1", externalId: "a.pdf", filename: "a.pdf", mime: "application/pdf",
      redactedText: "hi", piiCategoryCounts: {}, documentId: "doc-1", status: "synced", ingestedAt: 1,
    };
    let resolveIngest: (e: IngestHistoryEntry) => void = () => {};
    const fakeClient = { ingestFile: vi.fn(() => new Promise<IngestHistoryEntry>((r) => (resolveIngest = r))) };

    let api: ReturnType<typeof useEngine> | null = null;
    render(
      <EngineProvider baseUrl="http://x:8080" workerClient={fakeClient as never}>
        <Probe onReady={(a) => (api = a)} />
      </EngineProvider>
    );

    let promise!: Promise<IngestHistoryEntry>;
    await act(async () => {
      const file = new File([new Uint8Array([1])], "a.pdf", { type: "application/pdf" });
      promise = api!.ingestFile(file, "c1", "pass1234");
    });
    expect(screen.getByTestId("pending-count").textContent).toBe("1");

    await act(async () => {
      resolveIngest(entry);
      await promise;
    });
    await waitFor(() => expect(screen.getByTestId("pending-count").textContent).toBe("0"));
  });

  it("surfaces the error message via lastError when ingestFile rejects", async () => {
    const fakeClient = { ingestFile: vi.fn().mockRejectedValue(new Error("collection not found: c1")) };
    let api: ReturnType<typeof useEngine> | null = null;
    render(
      <EngineProvider baseUrl="http://x:8080" workerClient={fakeClient as never}>
        <Probe onReady={(a) => (api = a)} />
      </EngineProvider>
    );

    await act(async () => {
      const file = new File([new Uint8Array([1])], "a.pdf", { type: "application/pdf" });
      await expect(api!.ingestFile(file, "c1", "pass1234")).rejects.toThrow();
    });
    expect(api!.lastError).toMatch(/collection not found/);
  });
});
