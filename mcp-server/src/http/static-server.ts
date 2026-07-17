import { createReadStream, existsSync, realpathSync, statSync } from "node:fs";
import { extname, join, resolve, sep } from "node:path";
import type { ServerResponse } from "node:http";

const CONTENT_TYPES: Record<string, string> = {
  ".html": "text/html; charset=utf-8",
  ".js": "text/javascript; charset=utf-8",
  ".mjs": "text/javascript; charset=utf-8",
  ".css": "text/css; charset=utf-8",
  ".json": "application/json; charset=utf-8",
  ".wasm": "application/wasm",
  ".svg": "image/svg+xml",
  ".png": "image/png",
  ".ico": "image/x-icon",
};

const CROSS_ORIGIN_ISOLATION_HEADERS = {
  "Cross-Origin-Opener-Policy": "same-origin",
  "Cross-Origin-Embedder-Policy": "require-corp",
};

/**
 * Resolve `requestPath` against `rootDir`, decoding percent-escapes and
 * normalizing `..` segments first, then verifying the final absolute path is
 * still inside `rootDir`. Returns `null` if the request would escape the
 * root (path traversal). The containment check at the end is the actual
 * guarantee — normalization alone is not trusted.
 */
export function resolveSafePath(rootDir: string, requestPath: string): string | null {
  let decoded: string;
  try {
    decoded = decodeURIComponent(requestPath);
  } catch {
    return null;
  }
  const root = resolve(rootDir);
  // Treat the request as relative to rootDir by stripping any leading
  // separators, so `resolve` does not treat it as drive/root-anchored. The
  // containment check below is the real guarantee — `..` segments are only
  // safe once we have verified the resolved path stays inside rootDir.
  const relative = decoded.replace(/^[/\\]+/, "");
  const target = resolve(root, `.${sep}${relative}`);
  if (target !== root && !target.startsWith(root + sep)) return null;
  return target;
}

/** Serve a single file from `rootDir` for `requestPath`, or a 403/404. */
export function serveStaticFile(rootDir: string, requestPath: string, res: ServerResponse): void {
  let filePath = resolveSafePath(rootDir, requestPath === "/" ? "/index.html" : requestPath);
  if (filePath === null) {
    res.writeHead(403).end("Forbidden");
    return;
  }
  if (existsSync(filePath) && statSync(filePath).isDirectory()) {
    filePath = join(filePath, "index.html");
  }
  if (!existsSync(filePath)) {
    // Next.js's static export (`output: "export"`) names most pages as a
    // flat `<route>.html` file rather than `<route>/index.html` (e.g.
    // `/folder/placeholder` -> `folder/placeholder.html`) -- try that
    // convention before giving up, mirroring the standard static-site
    // `try_files $uri $uri.html` fallback.
    const htmlPath = `${filePath}.html`;
    if (existsSync(htmlPath) && statSync(htmlPath).isFile()) {
      filePath = htmlPath;
    } else {
      res.writeHead(404).end("Not Found");
      return;
    }
  }
  // Canonicalize both root and target so a symlink inside rootDir pointing
  // outside it cannot escape the containment boundary established above.
  try {
    const rootCanonical = realpathSync(rootDir);
    const fileCanonical = realpathSync(filePath);
    if (fileCanonical !== rootCanonical && !fileCanonical.startsWith(rootCanonical + sep)) {
      res.writeHead(403).end("Forbidden");
      return;
    }
  } catch {
    res.writeHead(404).end("Not Found");
    return;
  }
  const contentType = CONTENT_TYPES[extname(filePath)] ?? "application/octet-stream";
  res.writeHead(200, { "Content-Type": contentType, ...CROSS_ORIGIN_ISOLATION_HEADERS });
  res.on("error", () => {
    // Swallow client-disconnect (e.g. EPIPE) errors so they don't crash the process.
  });
  const stream = createReadStream(filePath);
  stream.on("error", () => {
    if (!res.headersSent) res.writeHead(500).end("Internal Server Error");
    else res.end();
  });
  stream.pipe(res);
}
