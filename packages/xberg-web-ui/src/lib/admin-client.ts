export type AdminPayload =
  | { op: "drop_collection"; collection: string }
  | { op: "delete_documents"; collection: string; external_ids: string[] }
  | { op: "stats"; collection: string };

export type AdminResult = {
  dropped?: boolean;
  deleted?: number;
  documents?: number;
  chunks?: number;
  last_ingested_at?: number;
};

const MAX_RETRIES = 3;
const BACKOFF_MS = 400;

async function postWithRetry(url: string, init: RequestInit): Promise<Response> {
  let last: Response | undefined;
  for (let i = 0; i <= MAX_RETRIES; i++) {
    const res = await fetch(url, init);
    last = res;
    if (res.status < 500) return res;
    if (i < MAX_RETRIES) {
      await new Promise((r) => setTimeout(r, BACKOFF_MS * 2 ** i));
    }
  }
  return last!;
}

export async function postAdmin(
  baseUrl: string,
  token: string,
  payload: AdminPayload
): Promise<AdminResult> {
  const url = `${baseUrl.replace(/\/$/, "")}/admin?token=${encodeURIComponent(token)}`;
  const res = await postWithRetry(url, {
    method: "POST",
    headers: { "Content-Type": "application/json", Authorization: `Bearer ${token}` },
    body: JSON.stringify(payload),
  });
  if (!res.ok) throw new Error(`admin failed (${res.status})`);
  return (await res.json()) as AdminResult;
}
