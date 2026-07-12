/**
 * In-memory only (not localStorage) — the token is only valid while the
 * MCP process that printed it is alive; persisting it across restarts
 * would let the UI silently use a stale/invalid token.
 */
let token: string | null = null;

export function setAuthToken(value: string | null): void {
  token = value;
}

export function getAuthToken(): string | null {
  return token;
}

/** Reads `?token=` from the current page URL and stores it, if present. */
export function captureAuthTokenFromLocation(): void {
  if (typeof window === "undefined") return;
  const fromUrl = new URL(window.location.href).searchParams.get("token");
  if (fromUrl) setAuthToken(fromUrl);
}

export function authedUrl(baseUrl: string, path: string): string {
  if (!token) throw new Error("auth token not set — call captureAuthTokenFromLocation() first");
  const url = new URL(path, baseUrl);
  url.searchParams.set("token", token);
  return url.toString();
}

export function authHeaders(): Record<string, string> {
  if (!token) throw new Error("auth token not set — call captureAuthTokenFromLocation() first");
  return { Authorization: `Bearer ${token}` };
}
