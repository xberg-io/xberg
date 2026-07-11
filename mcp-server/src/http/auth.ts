import { randomBytes, timingSafeEqual } from "node:crypto";
import type { IncomingHttpHeaders } from "node:http";

const BEARER_PREFIX = "Bearer ";

/** Generate a random 256-bit token, hex-encoded, for the localhost UI auth gate. */
export function generateAuthToken(): string {
  return randomBytes(32).toString("hex");
}

/**
 * Read the auth token from an `Authorization: Bearer <token>` header (used by
 * fetch calls from the UI's JS) or, failing that, from a `?token=` query
 * param (used for the initial page navigation, where custom headers aren't
 * available).
 */
export function extractToken(req: { headers: IncomingHttpHeaders }, url: URL): string | null {
  const header = req.headers.authorization;
  if (header?.startsWith(BEARER_PREFIX)) return header.slice(BEARER_PREFIX.length);
  return url.searchParams.get("token");
}

/** Constant-time comparison against the server's expected token. */
export function isValidToken(candidate: string | null, expected: string): boolean {
  if (!candidate) return false;
  const a = Buffer.from(candidate, "utf-8");
  const b = Buffer.from(expected, "utf-8");
  if (a.length !== b.length) return false;
  return timingSafeEqual(a, b);
}
