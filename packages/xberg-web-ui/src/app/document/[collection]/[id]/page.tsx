"use client";
import { useEffect, useState } from "react";
import { useParams } from "next/navigation";
import { getHistoryEntry } from "@/lib/ingest-history.js";
import { DocumentViewer } from "@/components/DocumentViewer.js";
import type { IngestHistoryEntry } from "@/lib/types.js";

// Required by `output: "export"` for dynamic route segments: Next.js needs
// at least one static param set at build time to produce an HTML+JS shell.
// Collection/document ids are created at runtime and unknowable at build
// time; this page is 100% client-side (useParams + useEffect), so the shell
// just needs to exist — the client router hydrates it with the real URL's
// params.
export function generateStaticParams() {
  return [{ collection: "placeholder", id: "placeholder" }];
}

export default function DocumentPage() {
  const { collection, id } = useParams<{ collection: string; id: string }>();
  const [entry, setEntry] = useState<IngestHistoryEntry | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    void getHistoryEntry(collection, id)
      .then(setEntry)
      .catch((err) => {
        setEntry(null);
        setError(err instanceof Error ? err.message : "Failed to load document");
      })
      .finally(() => {
        setLoading(false);
      });
  }, [collection, id]);

  if (loading) return <main className="p-6">Loading…</main>;
  if (error) return <main className="p-6 text-red-600">Failed to load document: {error}</main>;
  if (!entry) return <main className="p-6">Document not found.</main>;
  return (
    <main className="p-6">
      <DocumentViewer entry={entry} />
    </main>
  );
}
