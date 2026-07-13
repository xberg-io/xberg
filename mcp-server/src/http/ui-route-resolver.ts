import { existsSync } from "node:fs";
import { join } from "node:path";

// Next.js static export (`output: "export"`) can only emit files for the
// param combinations returned by `generateStaticParams` — one placeholder
// shell per dynamic route (see `document/[collection]/[id]/page.tsx` and
// `folder/[collection]/page.tsx`). Real collection/document ids are created
// at runtime and unknowable at build time, so a request for a real path has
// no matching file. Rewrite it to the placeholder shell instead of 404ing;
// the client re-derives the true params from the browser URL once
// `useParams()` hydrates (see DocumentPageClient/FolderPageClient).
//
// Kept in its own module (no `../engine.js` import) so it can be unit
// tested without pulling in the wasm `XbergEngine`, which isn't built in
// the lightweight MCP CI job (see vitest.config.ts's `wasmEngineTests`).
export function resolveUiSubPath(uiDistDir: string, subPath: string): string {
  const clean = subPath.split(/[?#]/)[0] ?? subPath;
  const segments = clean.split("/").filter(Boolean);
  if (segments[0] === "document" && segments.length >= 3) {
    if (existsSync(join(uiDistDir, "document", segments[1]!, segments[2]!, "index.html"))) return clean;
    return "/document/placeholder/placeholder/";
  }
  if (segments[0] === "folder" && segments.length >= 2) {
    if (existsSync(join(uiDistDir, "folder", segments[1]!, "index.html"))) return clean;
    return "/folder/placeholder/";
  }
  return clean;
}
