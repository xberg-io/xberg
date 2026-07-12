import { DOCUMENT_ID_PATTERN } from "./constants.js";

/**
 * Derive a `/map`-safe `document_id`/`external_id` from a user-supplied
 * filename. Deterministic (same input -> same output) so re-uploading the
 * same file idempotently replaces it, per `upsertDocument`'s contract.
 */
export function sanitizeExternalId(filename: string): string {
  const replaced = filename.replace(/[^A-Za-z0-9_.-]/g, "_");
  const trimmed = replaced.replace(/^_+|_+$/g, "");
  const safe = trimmed.length > 0 ? trimmed : "file";
  return DOCUMENT_ID_PATTERN.test(safe) ? safe : "file";
}
