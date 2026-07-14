"use client";
import { useEffect, useState } from "react";
import init, { XbergEngine } from "@xberg-io/xberg-wasm";
import { createXbergRuntimeFactory } from "xberg-wasm-runtime";

// Self-test route (dev/verification only): drives the FULL browser stack —
// wasm engine (init + extract) AND the xberg-wasm-runtime model injection
// (embedder/OCR/NER via transformers.js + onnxruntime-web, with a real model
// download from HuggingFace) — so we can confirm OCR/redaction prerequisites
// work end-to-end in a real browser.
export default function WasmSelfTestPage() {
  const [status, setStatus] = useState("pending");

  useEffect(() => {
    (async () => {
      try {
        setStatus("stage:init-start");
        await init();
        setStatus("stage:inited");

        setStatus("stage:factory");
        // wasmPaths: ORT's runtime .mjs/.wasm must be served same-origin
        // (public/ort/, populated by scripts/copy-ort-dist.mjs). The CDN
        // default hangs forever on crossOriginIsolated pages: ORT's threaded
        // runtime can't spawn its pthread workers from a cross-origin URL.
        const injection = await createXbergRuntimeFactory({
          forceWasmBackend: true,
          wasmPaths: "/ui/ort/",
        });
        setStatus("stage:factory-done");

        // wasm build has no tokio-runtime, so the timeout field must be null.
        const engine = new XbergEngine({ extraction_timeout_secs: null }, injection);
        setStatus("stage:engine");

        const sample = "Contact Alice at alice@example.com about the Q3 contract.";
        const bytes = new TextEncoder().encode(sample);
        const out = (await engine.extract(
          { kind: "bytes", bytes, filename: "sample.txt" },
          undefined
        )) as { results?: Array<{ content: string }> };
        const content = out.results?.[0]?.content ?? "";
        setStatus("stage:extracted");

        // Exercise the real in-browser embedder (bge-m3, 1024-dim via ORT).
        const vectors = await injection.embedder.embed([sample]);
        const dim = vectors[0]?.length ?? 0;
        if (dim !== 1024) {
          setStatus(`ERR embedder dim=${dim} expected 1024`);
          return;
        }
        setStatus(`OK extract_len=${content.length} embed_dim=${dim}`);
      } catch (e) {
        setStatus("ERR " + (e instanceof Error ? e.stack || e.message : String(e)));
      }
    })();
  }, []);

  return (
    <main className="p-6">
      <h1 className="text-xl font-semibold">wasm self-test</h1>
      <pre id="result" data-testid="result">{status}</pre>
    </main>
  );
}
