"use client";
import { useState } from "react";
import { useParams } from "next/navigation";
import { useEngine } from "@/providers/EngineProvider.js";
import { DocumentTable } from "@/components/DocumentTable.js";
import { Input } from "@/components/ui/input.js";

export function FolderPageClient({ collection: collectionParam }: { collection: string }) {
  // See DocumentPageClient: static export only bakes the placeholder shell's
  // param into props, so the real collection must be re-derived from the
  // actual browser URL once the client router has hydrated.
  const params = useParams<{ collection: string }>();
  const collection = params?.collection ?? collectionParam;
  const { ingestFile } = useEngine();
  const [passphrase, setPassphrase] = useState("");

  const onFiles = async (files: FileList | null) => {
    if (!files || !passphrase) return;
    for (const file of Array.from(files)) {
      try {
        await ingestFile(file, collection, passphrase);
      } catch {
        // Error is already tracked in EngineProvider's lastError state
        // Continue processing remaining files
      }
    }
  };

  return (
    <main className="p-6">
      <h1 className="mb-4 text-xl font-semibold">{collection}</h1>
      <div className="mb-4 space-y-2">
        <label htmlFor="passphrase" className="text-sm font-medium">
          Rehydration passphrase (never sent to the server in clear)
        </label>
        <Input id="passphrase" type="password" value={passphrase} onChange={(e) => setPassphrase(e.target.value)} />
        <input type="file" multiple disabled={!passphrase} aria-label="Upload documents" onChange={(e) => void onFiles(e.target.files)} />
      </div>
      <DocumentTable collection={collection} />
    </main>
  );
}
