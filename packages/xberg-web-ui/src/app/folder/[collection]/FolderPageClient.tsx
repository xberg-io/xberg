"use client";
import { type ChangeEvent, useEffect, useState } from "react";
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
  // for a hard navigation to a route Next itself didn't generate.
  //
  // The first client render must still match the server-rendered HTML
  // (baked with `collectionParam`, e.g. "placeholder"), or React's
  // hydration bails out and force-remounts the whole tree -- which was
  // silently discarding user input like the typed passphrase. So render
  // `collectionParam` on mount, then correct to the real value from
  // `window.location.pathname` in an effect (runs client-only, after
  // hydration completes).
  const [collection, setCollection] = useState(collectionParam);
  useEffect(() => {
    const real = collectionFromPathname("folder");
    if (real && real !== collectionParam) setCollection(real);
  }, [collectionParam]);
  const { ingestFile, lastError } = useEngine();
  const [passphrase, setPassphrase] = useState("");

  const onFiles = async (event: ChangeEvent<HTMLInputElement>) => {
    // `input.files` is a live FileList tied to the element -- resetting
    // `event.target.value` (done immediately below, so a re-selecting the
    // same file still fires `change`) also empties that FileList in place.
    // Snapshot to a plain array FIRST, or every upload silently no-ops.
    const files = Array.from(event.target.files ?? []);
    event.target.value = "";
    if (files.length === 0 || !passphrase) return;
    for (const file of files) {
      try {
        await ingestFile(file, collection, passphrase);
      } catch {
        // Error is already tracked in EngineProvider's lastError state
        // and rendered below. Continue processing remaining files.
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
        {lastError ? (
          <p role="alert" className="text-sm text-destructive">
            {lastError}
          </p>
        ) : null}
      </div>
      <DocumentTable collection={collection} />
    </main>
  );
}
