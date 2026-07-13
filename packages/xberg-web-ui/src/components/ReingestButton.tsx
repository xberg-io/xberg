"use client";

import { type ChangeEvent, useRef, useState } from "react";
import { Button } from "@/components/ui/button.js";
import { Input } from "@/components/ui/input.js";
import { useEngine } from "@/providers/EngineProvider.js";
import { sanitizeExternalId } from "@/lib/sanitize-id.js";

export interface ReingestButtonProps {
  collection: string;
  /**
   * The `external_id` of the document this button re-ingests. The picked
   * file's sanitized filename MUST match this, or `upsertDocument` would
   * silently create a new, unrelated document instead of replacing the one
   * being re-ingested (external_id is derived from the filename — see
   * `engine.worker.ts`'s `sanitizeExternalId(filename)`).
   */
  expectedExternalId: string;
}

export function ReingestButton({
  collection,
  expectedExternalId,
}: ReingestButtonProps) {
  const { ingestFile } = useEngine();
  const inputRef = useRef<HTMLInputElement>(null);
  const [passphrase, setPassphrase] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const onPick = async (event: ChangeEvent<HTMLInputElement>) => {
    const file = event.target.files?.[0];
    event.target.value = "";
    if (!file || !passphrase) return;
    setError(null);

    const pickedExternalId = sanitizeExternalId(file.name);
    if (pickedExternalId !== expectedExternalId) {
      setError(
        `"${file.name}" would ingest as a new document ("${pickedExternalId}"), not replace this one ("${expectedExternalId}"). Pick the original file (same name) to re-ingest.`,
      );
      return;
    }

    setBusy(true);
    try {
      await ingestFile(file, collection, passphrase);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="flex flex-col gap-2 border bg-muted rounded p-3">
      <div className="flex flex-wrap items-center gap-2">
        <Input
          type="password"
          aria-label="Rehydration passphrase"
          placeholder="Rehydration passphrase"
          value={passphrase}
          onChange={(e) => setPassphrase(e.target.value)}
          className="max-w-xs"
        />
        <input
          ref={inputRef}
          type="file"
          className="sr-only"
          tabIndex={-1}
          onChange={onPick}
        />
        <Button
          type="button"
          variant="outline"
          disabled={busy || !passphrase}
          aria-label="Re-ingest document"
          onClick={() => inputRef.current?.click()}
        >
          {busy ? "Re-ingesting…" : "Re-ingest"}
        </Button>
      </div>
      {error ? (
        <p role="alert" className="text-sm text-destructive">
          {error}
        </p>
      ) : null}
    </div>
  );
}
