"use client";
import { type ChangeEvent, useEffect, useRef, useState } from "react";
import { documentParamsFromPathname } from "@/lib/route-params.js";
import { getHistoryEntry } from "@/lib/ingest-history.js";
import { getAuthToken } from "@/lib/auth-client.js";
import { resolveMcpBaseUrl } from "@/lib/mcp-base-url.js";
import { useEngine } from "@/providers/EngineProvider.js";
import { DocumentViewer } from "@/components/DocumentViewer.js";
import { DeleteDialog } from "@/components/DeleteDialog.js";
import { ReingestButton } from "@/components/ReingestButton.js";
import { Button } from "@/components/ui/button.js";
import type { IngestHistoryEntry, OcrLine } from "@/lib/types.js";

export function DocumentPageClient({
  collection: collectionParam,
  id: idParam,
}: {
  collection: string;
  id: string;
}) {
  // Static export can only ever bake the placeholder shell's params into
  // this component's props, and mcp-server's static file server falls back
  // to serving that same shell for every real collection/id (see
  // ui-route-resolver.ts) -- its embedded router state says the route is
  // `/document/placeholder/placeholder`, so `useParams()` returns that
  // fixed value regardless of the actual address bar URL (confirmed live:
  // `window.location.pathname` reports the real path while `useParams()`
  // does not). Parse the real segments out of the pathname directly instead.
  const { collection: realCollection, id: realId } = documentParamsFromPathname();
  const collection = realCollection ?? collectionParam;
  const id = realId ?? idParam;
  const { ocrLayout } = useEngine();
  const [entry, setEntry] = useState<IngestHistoryEntry | null>(null);
  const [loading, setLoading] = useState(true);
  const [fileUrl, setFileUrl] = useState<string | undefined>(undefined);
  const [layoutLines, setLayoutLines] = useState<OcrLine[] | undefined>(undefined);
  const [viewerBusy, setViewerBusy] = useState(false);
  const fileInputRef = useRef<HTMLInputElement>(null);
  const fileUrlRef = useRef<string | undefined>(undefined);

  useEffect(() => {
    let active = true;
    setLoading(true);

    void getHistoryEntry(collection, id)
      .then((res) => {
        if (active) setEntry(res);
      })
      .catch(() => {
        if (active) setEntry(null);
      })
      .finally(() => {
        if (active) setLoading(false);
      });

    return () => {
      active = false;
    };
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
      setLayoutLines(undefined);
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
            baseUrl={resolveMcpBaseUrl()}
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
