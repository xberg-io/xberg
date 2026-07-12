import { describe, it, expect, beforeEach } from "vitest";
import { setAuthToken, getAuthToken, authedUrl, authHeaders } from "../src/lib/auth-client.js";

describe("lib/auth-client", () => {
  beforeEach(() => setAuthToken(null));

  it("returns null before a token is set", () => {
    expect(getAuthToken()).toBeNull();
  });

  it("stores and returns the token", () => {
    setAuthToken("abc123");
    expect(getAuthToken()).toBe("abc123");
  });

  it("authedUrl appends ?token= to a bare path", () => {
    setAuthToken("abc123");
    expect(authedUrl("http://x:8080", "/ingest")).toBe("http://x:8080/ingest?token=abc123");
  });

  it("authedUrl preserves existing query params", () => {
    setAuthToken("abc123");
    expect(authedUrl("http://x:8080", "/map?document_id=doc-1")).toBe(
      "http://x:8080/map?document_id=doc-1&token=abc123"
    );
  });

  it("authHeaders includes a Bearer header", () => {
    setAuthToken("abc123");
    expect(authHeaders()).toEqual({ Authorization: "Bearer abc123" });
  });

  it("authedUrl throws without a token", () => {
    expect(() => authedUrl("http://x:8080", "/ingest")).toThrow();
  });
});
