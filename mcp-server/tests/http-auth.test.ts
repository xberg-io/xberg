import { describe, it, expect } from "vitest";
import { generateAuthToken, extractToken, isValidToken } from "../src/http/auth.js";

describe("http/auth", () => {
  it("generateAuthToken returns a 64-char hex string", () => {
    const token = generateAuthToken();
    expect(token).toMatch(/^[0-9a-f]{64}$/);
  });

  it("extractToken reads a Bearer header over a query param", () => {
    const req = { headers: { authorization: "Bearer header-token" } };
    const url = new URL("http://localhost/ingest?token=query-token");
    expect(extractToken(req, url)).toBe("header-token");
  });

  it("extractToken falls back to the token query param", () => {
    const req = { headers: {} };
    const url = new URL("http://localhost/ui/?token=query-token");
    expect(extractToken(req, url)).toBe("query-token");
  });

  it("extractToken returns null when neither is present", () => {
    const req = { headers: {} };
    const url = new URL("http://localhost/ingest");
    expect(extractToken(req, url)).toBeNull();
  });

  it("isValidToken accepts the exact expected token", () => {
    expect(isValidToken("secret", "secret")).toBe(true);
  });

  it("isValidToken rejects a wrong or missing token", () => {
    expect(isValidToken("wrong", "secret")).toBe(false);
    expect(isValidToken(null, "secret")).toBe(false);
  });
});
