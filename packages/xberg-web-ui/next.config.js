import { fileURLToPath } from "node:url";

// The browser build uses wasm-pack's "web" target (built by
// `pnpm --filter @xberg-io/xberg-wasm build:wasm:web`). Its glue instantiates
// the .wasm at runtime via `new URL("xberg_wasm_bg.wasm", import.meta.url)`,
// so webpack emits the binary as a static asset and the worker fetches it —
// no build-time parse of the 95 MB binary (which is what broke the "bundler"
// target). The glue is patched by scripts/patch-web-env.mjs to provide the
// host libc shims (iswspace/strcmp/memchr/...) inline, so it no longer imports
// a separate "env" module. Node consumers (mcp-server) resolve the package
// normally and get pkg/nodejs, so this config never affects them.
const wasmWebPath = fileURLToPath(
  new URL("../../crates/xberg-wasm/pkg/web/xberg_wasm.js", import.meta.url)
);

/** @type {import('next').NextConfig} */
const nextConfig = {
  output: "export",
  basePath: "/ui",
  reactStrictMode: true,
  images: { unoptimized: true },
  // onnxruntime-web's default WASM backend needs `crossOriginIsolated` (a
  // multi-threaded, SharedArrayBuffer-backed worker pool) -- without these
  // headers it doesn't error, it just hangs forever inside `pipeline(...)`.
  // `output: "export"` means Next ignores `headers()` for the production
  // static export (a static host can't attach response headers); the real
  // serving path already sets the same two headers in
  // mcp-server/src/http/static-server.ts's CROSS_ORIGIN_ISOLATION_HEADERS.
  // This is dev-server-only parity so `next dev` doesn't hang the same way.
  async headers() {
    return [
      {
        source: "/:path*",
        headers: [
          { key: "Cross-Origin-Opener-Policy", value: "same-origin" },
          { key: "Cross-Origin-Embedder-Policy", value: "require-corp" },
        ],
      },
    ];
  },
  transpilePackages: ["onnxruntime-web", "@huggingface/transformers"],
  webpack(config, { isServer, webpack }) {
    // Source uses NodeNext-style `.js`-suffixed imports for `.ts`/`.tsx`
    // files (matches tsconfig's `moduleResolution: "bundler"`, and how
    // Vitest/esbuild already resolve it) — webpack needs an explicit
    // extension alias to resolve the same specifiers.
    config.resolve.extensionAlias = {
      ...(config.resolve.extensionAlias ?? {}),
      ".js": [".js", ".ts", ".tsx"],
    };

    // `xberg-wasm-runtime` (used by `EngineProvider`/`engine.worker.ts`)
    // statically imports these Node-only OCR/SQLite backends. They are only
    // *used* behind a runtime `typeof window` check that always resolves to
    // "unavailable" in a browser, but webpack still statically resolves the
    // imports to build a valid chunk — and these packages ship native
    // `onnxruntime-node`/`@napi-rs/canvas`/`better-sqlite3` bindings that
    // cannot be bundled for the browser at all. Stub them for BOTH the server
    // (SSR) and client compiles, since the server graph also pulls in
    // `xberg-wasm-runtime`. `createOcr`/`createVectorStore` already handle a
    // missing backend (return null / pick the browser store).
    config.resolve.alias = {
      ...(config.resolve.alias ?? {}),
      "ppu-paddle-ocr": false,
      "ppu-ocv": false,
      "onnxruntime-node": false,
      "@napi-rs/canvas": false,
      "better-sqlite3": false,
      "sqlite-vec": false,
      // `@huggingface/transformers`'s Node build (`transformers.node.mjs`)
      // statically imports `sharp`; the browser build never touches it, but
      // Next's server (SSR) compile resolves the `node` export condition and
      // trips over sharp's native submodules. Stub it for both compiles.
      "sharp": false,
    };
    if (!isServer) {
      // Browser builds load the wasm-pack "web" target instead of the
      // package "main" (pkg/nodejs). Node consumers (mcp-server) are
      // untouched: they resolve the package normally and get pkg/nodejs.
      config.resolve.alias["@xberg-io/xberg-wasm"] = wasmWebPath;
    }

    // onnxruntime-web's threaded WASM backend (pulled in by
    // @huggingface/transformers) ships a worker bootstrap file
    // (ort.bundle.min.mjs) that it references via
    // `new URL("ort.bundle.min.mjs", import.meta.url)`. Webpack statically
    // detects that pattern and copies the file verbatim into static/media/
    // as a raw asset -- it never passes through webpack's module/transpile
    // pipeline, so `transpilePackages` has no effect on it, and mutating
    // `optimization.minimizer[*].options.exclude` doesn't either (Next's
    // built-in minifier plugin doesn't honor it for asset-module output).
    // Next's production build still runs TerserPlugin over every emitted
    // .mjs file and chokes on this one's `import.meta`/`import`/`export`
    // syntax (parsed as a plain script, not a module). TerserPlugin skips
    // any asset already flagged `info.minimized` -- and this file already
    // ships pre-minified from onnxruntime-web ("ort.bundle.min") -- so mark
    // it as such via a processAssets hook that runs before minification.
    if (!isServer) {
      const ORT_BUNDLE_PATTERN = /ort\.bundle\.min[^/\\]*\.mjs$/;
      config.plugins.push({
        apply(compiler) {
          compiler.hooks.compilation.tap("SkipMinifyForOrtBundle", (compilation) => {
            compilation.hooks.processAssets.tap(
              {
                name: "SkipMinifyForOrtBundle",
                stage: webpack.Compilation.PROCESS_ASSETS_STAGE_ADDITIONAL,
              },
              (assets) => {
                for (const name of Object.keys(assets)) {
                  if (ORT_BUNDLE_PATTERN.test(name)) {
                    compilation.updateAsset(name, (source) => source, (info) => ({
                      ...info,
                      minimized: true,
                    }));
                  }
                }
              },
            );
          });
        },
      });
    }

    return config;
  },
};

export default nextConfig;
