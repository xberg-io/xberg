"use client";
import { type ChangeEvent, useEffect, useRef, useState } from "react";
import { useParams } from "next/navigation";
import { getHistoryEntry } from "@/lib/ingest-history.js";
import { getAuthToken } from "@/lib/auth-client.js";
import { useEngine } from "@/providers/EngineProvider.js";
import { DocumentViewer } from "@/components/DocumentViewer.js";
import { DeleteDialog } from "@/components/DeleteDialog.js";
import { ReingestButton } from "@/components/ReingestButton.js";
import { Button } from "@/components/ui/button.js";
import type { IngestHistoryEntry, OcrLine } from "@/lib/types.js";

const MCP_BASE_URL = process.env.NEXT_PUBLIC_MCP_BASE_URL;

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
  const { ocrLayout } = useEngine();
  const [entry, setEntry] = useState<IngestHistoryEntry | null>(null);
  const [loading, setLoading] = useState(true);
  const [fileUrl, setFileUrl] = useState<string | undefined>(undefined);
  const [layoutLines, setLayoutLines] = useState<OcrLine[] | undefined>(undefined);
  const [viewerBusy, setViewerBusy] = useState(false);
  const fileInputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    void getHistoryEntry(collection, id)
      .then(setEntry)
      .catch(() => {
        setEntry(null);
      })
      .finally(() => {
        setLoading(false);
      });
  }, [collection, id]);

  const onLoadFile = async (event: ChangeEvent<HTMLInputElement>) => {
    const file = event.target.files?.[0];
    event.target.value = "";
    if (!file) return;
    setViewerBusy(true);
    try {
      setFileUrl(URL.createObjectURL(file));
      const bytes = new Uint8Array(await file.arrayBuffer());
      setLayoutLines(await ocrLayout(bytes));
    } catch {
      setLayoutLines(undefined);
    } finally {
      setViewerBusy(false);
    }
  };

  if (loading) return <main className="p-6">Loading…</main>;
  if (!entry) return <main className="p-6">Document not found.</main>;

  return (
    <main className="space-y-4 p-6">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <h1 className="text-xl font-semibold">{entry.filename}</h1>
        <div className="flex flex-wrap items-center gap-2">
          <input
            ref={fileInputRef}
            type="file"
            className="sr-only"
            onChange={onLoadFile}
          />
          <Button
            type="button"
            variant="outline"
            disabled={viewerBusy}
            aria-label="Load document file"
            onClick={() => fileInputRef.current?.click()}
          >
            {viewerBusy ? "Computing layout…" : "Load document file"}
          </Button>
          <DeleteDialog
            baseUrl={
              MCP_BASE_URL ??
              (typeof window !== "undefined"
                ? window.location.origin
                : "http://127.0.0.1:8080")
            }
            token={getAuthToken() ?? ""}
            collection={collection}
            externalIds={[id]}
          />
          <ReingestButton collection={collection} />
        </div>
      </div>

      <DocumentViewer
        fileUrl={fileUrl}
        mime={entry.mime}
        redactedText={entry.redactedText}
        counts={entry.piiCategoryCounts}
        layoutLines={layoutLines}
      />
    </main>
  );
}
