/** @type {import('next').NextConfig} */
const nextConfig = {
  output: "export",
  basePath: "/ui",
  reactStrictMode: true,
  images: { unoptimized: true },
  webpack(config, { isServer }) {
    // Source uses NodeNext-style `.js`-suffixed imports for `.ts`/`.tsx`
    // files (matches tsconfig's `moduleResolution: "bundler"`, and how
    // Vitest/esbuild already resolve it) — webpack needs an explicit
    // extension alias to resolve the same specifiers.
    config.resolve.extensionAlias = {
      ...(config.resolve.extensionAlias ?? {}),
      ".js": [".js", ".ts", ".tsx"],
    };

    if (!isServer) {
      // `xberg-wasm-runtime` (used by `EngineProvider`/`engine.worker.ts`)
      // only reaches these Node-only OCR/SQLite backends behind a runtime
      // `typeof window` check that always resolves to "unavailable" in a
      // browser — but webpack still statically resolves every import to
      // build a valid worker chunk, and these packages ship native
      // `onnxruntime-node`/`@napi-rs/canvas`/`better-sqlite3` bindings that
      // cannot be bundled for the browser at all. Stub them to an empty
      // module for the client/worker compilation only; the Node-side
      // build (mcp-server) is untouched since it never runs through this
      // webpack config. `createOcr`/`createVectorStore` already handle a
      // missing backend (return null / pick the browser store).
      config.resolve.alias = {
        ...(config.resolve.alias ?? {}),
        "ppu-paddle-ocr": false,
        "ppu-ocv": false,
        "onnxruntime-node": false,
        "@napi-rs/canvas": false,
        "better-sqlite3": false,
        "sqlite-vec": false,
      };
    }
    return config;
  },
};

export default nextConfig;
