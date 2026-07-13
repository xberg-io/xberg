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

export function DocumentPageClient({
  collection: collectionParam,
  id: idParam,
}: {
  collection: string;
  id: string;
}) {
  // Static export can only ever bake the placeholder shell's params into
  // this component's props. For a real collection/id, the server falls
  // back to serving that same shell (see ui-server.ts), so the true
  // segments must be re-derived from the actual browser URL on the client
  // instead of trusted from props.
  const params = useParams<{ collection: string; id: string }>();
  const collection = params?.collection ?? collectionParam;
  const id = params?.id ?? idParam;
  const { ocrLayout } = useEngine();
  const [entry, setEntry] = useState<IngestHistoryEntry | null>(null);
  const [loading, setLoading] = useState(true);
  const [fileUrl, setFileUrl] = useState<string | undefined>(undefined);
  const [layoutLines, setLayoutLines] = useState<OcrLine[] | undefined>(undefined);
  const [viewerBusy, setViewerBusy] = useState(false);
  const fileInputRef = useRef<HTMLInputElement>(null);
  const fileUrlRef = useRef<string | undefined>(undefined);

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

  useEffect(() => {
    return () => {
      if (fileUrlRef.current) URL.revokeObjectURL(fileUrlRef.current);
    };
  }, []);

  const onLoadFile = async (event: ChangeEvent<HTMLInputElement>) => {
    const file = event.target.files?.[0];
    event.target.value = "";
    if (!file) return;
    setViewerBusy(true);
    try {
      const nextUrl = URL.createObjectURL(file);
      if (fileUrlRef.current) URL.revokeObjectURL(fileUrlRef.current);
      fileUrlRef.current = nextUrl;
      setFileUrl(nextUrl);
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
          <ReingestButton collection={collection} expectedExternalId={id} />
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
