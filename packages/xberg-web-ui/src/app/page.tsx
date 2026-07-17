"use client";
import { useEffect, useState } from "react";
import Link from "next/link";
import { listFolders } from "@/lib/ingest-history.js";
import { CreateFolderDialog } from "@/components/CreateFolderDialog.js";
import { resolveMcpBaseUrl } from "@/lib/mcp-base-url.js";

export default function HomePage() {
  const [folders, setFolders] = useState<string[]>([]);
  // This page previously hardcoded `window.location.origin` unconditionally,
  // ignoring NEXT_PUBLIC_MCP_BASE_URL entirely (unlike layout.tsx/
  // DocumentPageClient.tsx, which already use this same shared helper).
  // That's correct only when web-ui is served from the same origin as
  // mcp-server (its own static /ui route) -- a separate dev server on
  // another port needs the explicit override to reach mcp-server's API at all.
  const baseUrl = resolveMcpBaseUrl();

  useEffect(() => {
    void listFolders()
      .then(setFolders)
      .catch((err) => {
        console.error("Failed to load folders:", err instanceof Error ? err.message : String(err));
      });
  }, []);

  return (
    <main className="p-6">
      <div className="mb-4 flex items-center justify-between">
        <h1 className="text-xl font-semibold">Folders</h1>
        <CreateFolderDialog baseUrl={baseUrl} onCreated={(name) => setFolders((f) => Array.from(new Set([...f, name])))} />
      </div>
      {folders.length === 0 ? (
        <p className="text-sm text-slate-500">No folders yet — create one to start uploading.</p>
      ) : (
        <ul className="space-y-1">
          {folders.map((f) => (
            <li key={f}>
              <Link className="text-slate-900 underline" href={`/folder/${f}`}>
                {f}
              </Link>
            </li>
          ))}
        </ul>
      )}
    </main>
  );
}
