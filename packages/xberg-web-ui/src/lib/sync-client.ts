import { authedUrl, authHeaders } from "./auth-client.js";
import type { CollectionPayload, IngestPayload } from "./types.js";

const MAX_RETRIES = 3;
const BACKOFF_MS = 400;

/**
 * POST with retry+backoff on 5xx AND on a network-level `fetch` rejection
 * (DNS failure, connection refused — realistic when the local MCP dev
 * server isn't up yet). 4xx is a client error (bad payload, unknown
 * collection) and is never retried.
 */
async function postWithRetry(url: string, init: RequestInit, label: string): Promise<Response> {
  let lastResponse: Response | undefined;
  let lastError: unknown;
  for (let attempt = 0; attempt <= MAX_RETRIES; attempt++) {
    try {
      const res = await fetch(url, init);
      if (res.status < 500) return res;
      lastResponse = res;
      lastError = undefined;
    } catch (err) {
      lastError = err;
      lastResponse = undefined;
    }
    if (attempt < MAX_RETRIES) {
      await new Promise((resolve) => setTimeout(resolve, BACKOFF_MS * 2 ** attempt));
    }
  }
  if (lastResponse) return lastResponse;
  const message = lastError instanceof Error ? lastError.message : String(lastError);
  throw new Error(`${label} failed: network error: ${message}`);
}

async function throwOnError(res: Response, label: string): Promise<Response> {
  if (!res.ok) {
    let detail = "";
    try {
      const body = (await res.json()) as { error?: string };
      detail = body.error ?? "";
    } catch {
      // response body wasn't JSON; fall through with the empty detail
    }
    throw new Error(`${label} failed (${res.status})${detail ? `: ${detail}` : ""}`);
  }
  return res;
}

export async function postCollection(baseUrl: string, payload: CollectionPayload): Promise<void> {
  const res = await postWithRetry(
    authedUrl(baseUrl, "/collection"),
    { method: "POST", headers: { "Content-Type": "application/json", ...authHeaders() }, body: JSON.stringify(payload) },
    "postCollection"
  );
  await throwOnError(res, "postCollection");
}

export async function postIngest(baseUrl: string, payload: IngestPayload): Promise<{ document_id: string }> {
  const res = await postWithRetry(
    authedUrl(baseUrl, "/ingest"),
    { method: "POST", headers: { "Content-Type": "application/json", ...authHeaders() }, body: JSON.stringify(payload) },
    "postIngest"
  );
  await throwOnError(res, "postIngest");
  return (await res.json()) as { document_id: string };
}

export async function postMap(baseUrl: string, documentId: string, blob: Uint8Array): Promise<void> {
  const res = await postWithRetry(
    authedUrl(baseUrl, `/map?document_id=${encodeURIComponent(documentId)}`),
    { method: "POST", headers: authHeaders(), body: blob as BufferSource },
    "postMap"
  );
  await throwOnError(res, "postMap");
}
