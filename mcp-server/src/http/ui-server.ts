import type { IncomingMessage, ServerResponse } from "node:http";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";
import { generateAuthToken, extractToken, isValidToken } from "./auth.js";
import { serveStaticFile } from "./static-server.js";
import { createIngestHandler } from "./ingest-route.js";
import { createMapUploadHandler } from "./map-route.js";
import { createCollectionHandler } from "./collection-route.js";
import { createAdminHandler } from "./admin-route.js";
import { getRuntime } from "../engine.js";
import { getCacheDir } from "../paths.js";
import { resolveUiSubPath } from "./ui-route-resolver.js";

export { resolveUiSubPath } from "./ui-route-resolver.js";

// This file lives at `src/http/ui-server.ts` in dev (`tsx`) and
// `dist/http/ui-server.js` after `tsc` — both are two directories below the
// package root, so `../../` resolves to the package root in either case.
const PACKAGE_ROOT = join(dirname(fileURLToPath(import.meta.url)), "..", "..");

export interface UiRoutes {
  /** The token clients must present via `Authorization: Bearer` or `?token=`. */
  token: string;
  /** Returns `true` if this request matched a UI/ingest/map route (handled or rejected), `false` to fall through. */
  handleRequest(req: IncomingMessage, res: ServerResponse, url: URL): Promise<boolean>;
}

export function createUiRoutes(): UiRoutes {
  const token = process.env["XBERG_MCP_UI_TOKEN"] ?? generateAuthToken();
  const uiDistDir = process.env["XBERG_UI_DIST_DIR"] ?? join(PACKAGE_ROOT, "ui-dist");
  const rehydrationDir = (): string => join(getCacheDir(), "rehydration");

  const ingestHandler = createIngestHandler(() => getRuntime().store);
  const mapHandler = createMapUploadHandler(rehydrationDir);
  const collectionHandler = createCollectionHandler(() => getRuntime().store);
  const adminHandler = createAdminHandler(() => getRuntime().store);

  return {
    token,
    async handleRequest(req, res, url) {
      const isUi = url.pathname === "/ui" || url.pathname.startsWith("/ui/");
      const isIngest = req.method === "POST" && url.pathname === "/ingest";
      const isMap = req.method === "POST" && url.pathname === "/map";
      const isCollection = req.method === "POST" && url.pathname === "/collection";
      const isAdmin = req.method === "POST" && url.pathname === "/admin";
      if (!isUi && !isIngest && !isMap && !isCollection && !isAdmin) return false;

      // `/ui/*` (GET) serves the compiled static app shell -- no secrets
      // live in it, and it cannot practically be gated behind `?token=`:
      // Next.js's client-side router fetches its own hashed JS/CSS bundles
      // (`/ui/_next/static/...`), ORT's `.wasm`/`.mjs` runtime
      // (`/ui/ort/...`, see engine.worker.ts's `wasmPaths: "/ui/ort/"`),
      // and RSC payloads for client-side navigation
      // (`/ui/folder/<name>.txt?_rsc=...`) via plain relative fetches that
      // never carry the page URL's query string -- there is no way for any
      // of them to "inherit" the token. Gating `/ui` at all left every one
      // of these 401ing right after a successful, authenticated top-level
      // navigation, so the app could load the shell but never actually
      // render a folder, run OCR/NER models, or navigate client-side. The
      // real security boundary is the data-mutating API routes below
      // (ingest/map/collection/admin), which remain fully token-gated.
      if (!isUi) {
        const candidate = extractToken(req, url);
        if (!isValidToken(candidate, token)) {
          res.writeHead(401, { "Content-Type": "application/json" }).end(JSON.stringify({ error: "unauthorized" }));
          return true;
        }
      }

      if (isIngest) {
        await ingestHandler(req, res);
        return true;
      }
      if (isMap) {
        await mapHandler(req, res, url);
        return true;
      }
      if (isCollection) {
        await collectionHandler(req, res);
        return true;
      }
      if (isAdmin) {
        await adminHandler(req, res, url);
        return true;
      }
      const subPath = url.pathname === "/ui" ? "/" : url.pathname.slice("/ui".length);
      serveStaticFile(uiDistDir, resolveUiSubPath(uiDistDir, subPath), res);
      return true;
    },
  };
}
