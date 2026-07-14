import { describe, it, expect, beforeEach, vi, afterEach } from "vitest";
import { postCollection, postIngest, postMap } from "../src/lib/sync-client.js";
import { setAuthToken } from "../src/lib/auth-client.js";

function jsonResponse(status: number, body: unknown): Response {
  return { status, ok: status < 300, json: async () => body } as Response;
}

describe("lib/sync-client", () => {
  beforeEach(() => setAuthToken("tok"));
  afterEach(() => {
    vi.unstubAllGlobals();
    vi.useRealTimers();
  });

  it("postCollection posts to /collection with the token", async () => {
    const fetchMock = vi.fn().mockResolvedValue(jsonResponse(200, { created: true }));
    vi.stubGlobal("fetch", fetchMock);
    await postCollection("http://x:8080", { name: "c1", embedding_dim: 1024 });
    const [url, init] = fetchMock.mock.calls[0] as [string, RequestInit];
    expect(url).toBe("http://x:8080/collection?token=tok");
    expect(init.method).toBe("POST");
    expect(JSON.parse(init.body as string)).toEqual({ name: "c1", embedding_dim: 1024 });
  });

  it("postIngest posts JSON and returns the document_id", async () => {
    const fetchMock = vi.fn().mockResolvedValue(jsonResponse(200, { document_id: "doc-1" }));
    vi.stubGlobal("fetch", fetchMock);
    const result = await postIngest("http://x:8080", {
      collection: "c1",
      external_id: "doc-1",
      full_text: "hello",
      chunks: [],
    });
    expect(result).toEqual({ document_id: "doc-1" });
  });

  it("postMap posts the raw blob with document_id in the query string", async () => {
    const fetchMock = vi.fn().mockResolvedValue(jsonResponse(200, { status: "stored" }));
    vi.stubGlobal("fetch", fetchMock);
    const blob = new Uint8Array([1, 2, 3]);
    await postMap("http://x:8080", "doc-1", blob);
    const [url, init] = fetchMock.mock.calls[0] as [string, RequestInit];
    expect(url).toBe("http://x:8080/map?document_id=doc-1&token=tok");
    expect(init.body).toBe(blob);
  });

  it("retries on a 500 then succeeds", async () => {
    vi.useFakeTimers();
    const fetchMock = vi
      .fn()
      .mockResolvedValueOnce(jsonResponse(500, { error: "boom" }))
      .mockResolvedValueOnce(jsonResponse(200, { document_id: "doc-1" }));
    vi.stubGlobal("fetch", fetchMock);
    const promise = postIngest("http://x:8080", { collection: "c1", external_id: "d", full_text: "t", chunks: [] });
    await vi.advanceTimersByTimeAsync(400);
    const result = await promise;
    expect(result).toEqual({ document_id: "doc-1" });
    expect(fetchMock).toHaveBeenCalledTimes(2);
  });

  it("throws (does not retry) on a 400", async () => {
    const fetchMock = vi.fn().mockResolvedValue(jsonResponse(400, { error: "invalid payload" }));
    vi.stubGlobal("fetch", fetchMock);
    await expect(
      postIngest("http://x:8080", { collection: "c1", external_id: "d", full_text: "t", chunks: [] })
    ).rejects.toThrow(/invalid payload/);
    expect(fetchMock).toHaveBeenCalledTimes(1);
  });

  it("retries on a network-level fetch rejection then succeeds", async () => {
    vi.useFakeTimers();
    const fetchMock = vi
      .fn()
      .mockRejectedValueOnce(new TypeError("fetch failed"))
      .mockResolvedValueOnce(jsonResponse(200, { document_id: "doc-1" }));
    vi.stubGlobal("fetch", fetchMock);
    const promise = postIngest("http://x:8080", { collection: "c1", external_id: "d", full_text: "t", chunks: [] });
    await vi.advanceTimersByTimeAsync(400);
    const result = await promise;
    expect(result).toEqual({ document_id: "doc-1" });
    expect(fetchMock).toHaveBeenCalledTimes(2);
  });

  it("throws a labeled error after exhausting retries on repeated network failure", async () => {
    vi.useFakeTimers();
    const fetchMock = vi.fn().mockRejectedValue(new TypeError("fetch failed"));
    vi.stubGlobal("fetch", fetchMock);
    const promise = postIngest("http://x:8080", { collection: "c1", external_id: "d", full_text: "t", chunks: [] });
    // Attach the rejection assertion before advancing timers -- postIngest
    // rejects mid-advance, and a handler attached only after all three
    // advances complete leaves a window where Node flags the rejection as
    // unhandled (a real "Unhandled Error" in the vitest run, even though
    // the test itself passes).
    const assertion = expect(promise).rejects.toThrow(/postIngest failed: network error/);
    await vi.advanceTimersByTimeAsync(400);
    await vi.advanceTimersByTimeAsync(800);
    await vi.advanceTimersByTimeAsync(1600);
    await assertion;
    expect(fetchMock).toHaveBeenCalledTimes(4);
  });
});
