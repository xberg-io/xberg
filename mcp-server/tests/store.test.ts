import { describe, expect, it, vi } from "vitest";

vi.mock("xberg-rag-node", () => ({
  RagStore: class {},
}));

import { ensureCollectionWithDim, withTimeout } from "../src/store.js";

describe("ensureCollectionWithDim", () => {
  it("rejects non-positive embedding dimensions before reading the store", async () => {
    const getCollection = vi.fn();
    const store = { getCollection } as never;

    await expect(ensureCollectionWithDim(store, "documents", 0)).rejects.toThrow(
      "Embedding dimension must be greater than 0",
    );
    expect(getCollection).not.toHaveBeenCalled();
  });
});

describe("withTimeout", () => {
  it("warns when work times out and later resolves", async () => {
    vi.useFakeTimers();
    const warn = vi.spyOn(console, "warn").mockImplementation(() => undefined);
    let resolveWork!: (value: string) => void;
    const work = new Promise<string>((resolve) => {
      resolveWork = resolve;
    });

    const result = withTimeout(work, 100, "embedding");
    const rejection = expect(result).rejects.toThrow("embedding timed out after 100ms");
    await vi.advanceTimersByTimeAsync(100);
    await rejection;
    expect(warn).toHaveBeenCalledWith("embedding timed out after 100ms; underlying work is still running");

    resolveWork("done");
    await Promise.resolve();
    expect(warn).toHaveBeenCalledWith("embedding resolved after timing out");

    warn.mockRestore();
    vi.useRealTimers();
  });

  it("warns when work rejects after timing out", async () => {
    vi.useFakeTimers();
    const warn = vi.spyOn(console, "warn").mockImplementation(() => undefined);
    let rejectWork!: (reason: Error) => void;
    const work = new Promise<string>((_resolve, reject) => {
      rejectWork = reject;
    });

    const result = withTimeout(work, 100, "embedding");
    const rejection = expect(result).rejects.toThrow("embedding timed out after 100ms");
    await vi.advanceTimersByTimeAsync(100);
    await rejection;

    const lateError = new Error("native failure");
    rejectWork(lateError);
    await Promise.resolve();
    expect(warn).toHaveBeenCalledWith("embedding rejected after timing out", lateError);

    warn.mockRestore();
    vi.useRealTimers();
  });
});
