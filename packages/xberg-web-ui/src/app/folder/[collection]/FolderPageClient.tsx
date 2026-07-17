"use client";
import { type ChangeEvent, useState } from "react";
import { useEngine } from "@/providers/EngineProvider.js";
import { DocumentTable } from "@/components/DocumentTable.js";
import { Input } from "@/components/ui/input.js";
import { collectionFromPathname } from "@/lib/route-params.js";

export function FolderPageClient({ collection: collectionParam }: { collection: string }) {
  // Static export (`output: "export"`) only ever generates ONE file for this
  // dynamic route -- `folder/placeholder.html` -- and mcp-server's static
  // file server (ui-route-resolver.ts) serves that same file for every real
  // `/folder/<name>` request. Its embedded router state says the route is
  // `/folder/placeholder`, so Next's own `useParams()` returns
  // `{collection: "placeholder"}` regardless of the actual address bar URL
  // (confirmed live: `window.location.pathname` correctly reports the real
  // path while `useParams()` does not) -- there is no client-side re-sync
  // for a hard navigation to a route Next itself didn't generate. Parse the
  // real segment out of `window.location.pathname` directly instead.
  const collection = collectionFromPathname("folder") ?? collectionParam;
  const { ingestFile } = useEngine();
  const [passphrase, setPassphrase] = useState("");

  const onFiles = async (event: ChangeEvent<HTMLInputElement>) => {
    const files = event.target.files;
    event.target.value = "";
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
        <input type="file" multiple disabled={!passphrase} aria-label="Upload documents" onChange={(e) => void onFiles(e)} />
      </div>
      <DocumentTable collection={collection} />
    </main>
  );
}
