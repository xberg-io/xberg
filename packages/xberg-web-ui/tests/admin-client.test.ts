import { describe, it, expect, vi, afterEach } from "vitest";
import { postAdmin } from "../src/lib/admin-client.js";

function jsonResponse(status: number, body: unknown): Response {
  return { status, ok: status < 300, json: async () => body } as Response;
}

describe("lib/admin-client", () => {
  afterEach(() => vi.unstubAllGlobals());

  it("drop_collection posts to /admin with a bearer header, no token in the URL, and returns { dropped: true }", async () => {
    const fetchMock = vi.fn().mockResolvedValue(jsonResponse(200, { dropped: true }));
    vi.stubGlobal("fetch", fetchMock);
    const result = await postAdmin("http://x:8080", "tok", { op: "drop_collection", collection: "c1" });
    expect(result).toEqual({ dropped: true });
    const [url, init] = fetchMock.mock.calls[0] as [string, RequestInit];
    expect(url).toBe("http://x:8080/admin");
    expect((init.headers as Record<string, string>)["Authorization"]).toBe("Bearer tok");
  });

  it("does not retry a failed drop_collection (destructive mutation)", async () => {
    const fetchMock = vi.fn().mockResolvedValue(jsonResponse(500, { error: "boom" }));
    vi.stubGlobal("fetch", fetchMock);
    await expect(
      postAdmin("http://x:8080", "tok", { op: "drop_collection", collection: "c1" })
    ).rejects.toThrow();
    expect(fetchMock).toHaveBeenCalledTimes(1);
  });

  it("delete_documents returns { deleted: 2 }", async () => {
    const fetchMock = vi.fn().mockResolvedValue(jsonResponse(200, { deleted: 2 }));
    vi.stubGlobal("fetch", fetchMock);
    const result = await postAdmin("http://x:8080", "tok", {
      op: "delete_documents",
      collection: "c1",
      external_ids: ["a", "b"],
    });
    expect(result).toEqual({ deleted: 2 });
  });

  it("throws on a 400 without retrying", async () => {
    const fetchMock = vi.fn().mockResolvedValue(jsonResponse(400, { error: "bad" }));
    vi.stubGlobal("fetch", fetchMock);
    await expect(
      postAdmin("http://x:8080", "tok", { op: "stats", collection: "c1" })
    ).rejects.toThrow();
    expect(fetchMock).toHaveBeenCalledTimes(1);
  });
});
