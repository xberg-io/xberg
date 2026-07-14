// Copy onnxruntime-web's runtime files (shipped inside
// @huggingface/transformers/dist) into public/ort/ so they are served
// SAME-ORIGIN.
//
// Why this is required, not an optimization: transformers.js defaults
// `env.backends.onnx.wasm.wasmPaths` to the jsdelivr CDN. Fetching the
// .mjs/.wasm cross-origin works (jsdelivr sends CORS + CORP headers), but on
// a crossOriginIsolated page (our COOP/COEP headers, required for the
// SharedArrayBuffer-backed threaded backend and OPFS) ORT selects its
// THREADED runtime, whose Emscripten bootstrap spawns a pthread worker pool
// via `new Worker(new URL(import.meta.url))`. With CDN wasmPaths,
// import.meta.url is the CDN origin -> cross-origin Worker -> SecurityError
// ("Script at 'https://cdn.jsdelivr.net/...' cannot be accessed from origin
// ...") which Emscripten's pool bootstrap never surfaces -- pipeline() hangs
// forever with zero console output and zero network activity. Serving these
// files same-origin (and pointing wasmPaths at them) is the only fix that
// keeps threading enabled.
import { copyFileSync, existsSync, mkdirSync, readdirSync, statSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
// @huggingface/transformers is a dependency of xberg-wasm-runtime (the
// package that actually calls pipeline()), not of xberg-web-ui itself, so
// read its dist straight out of that package's node_modules (pnpm's strict
// layout means web-ui cannot require.resolve it, and the package's exports
// map blocks resolving ./package.json anyway).
const distDir = join(
  here, "..", "..", "xberg-wasm-runtime", "node_modules", "@huggingface", "transformers", "dist",
);
if (!existsSync(distDir)) {
  console.error(`[copy-ort-dist] transformers dist not found at ${distDir} -- run pnpm install first`);
  process.exit(1);
}
const outDir = join(here, "..", "public", "ort");

mkdirSync(outDir, { recursive: true });

const ortFiles = readdirSync(distDir).filter(
  (f) => f.startsWith("ort-") && (f.endsWith(".mjs") || f.endsWith(".wasm")),
);
if (ortFiles.length === 0) {
  console.error(`[copy-ort-dist] no ort-*.mjs/.wasm files found in ${distDir}`);
  process.exit(1);
}
for (const f of ortFiles) {
  const src = join(distDir, f);
  const dest = join(outDir, f);
  copyFileSync(src, dest);
  console.log(`[copy-ort-dist] ${f} (${(statSync(src).size / 1024 / 1024).toFixed(1)} MB) -> public/ort/`);
}
