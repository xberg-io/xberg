"use client";
import { useEffect, useState } from "react";
import Link from "next/link";
import { listFolders } from "@/lib/ingest-history.js";
import { CreateFolderDialog } from "@/components/CreateFolderDialog.js";

export default function HomePage() {
  const [folders, setFolders] = useState<string[]>([]);
  const baseUrl = typeof window !== "undefined" ? window.location.origin : "http://127.0.0.1:8080";

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
