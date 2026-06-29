---
description: TypeScript MCP server — 30 tools across 10 categories wrapping xberg extraction and xberg-rag over NAPI-RS
---

- Transport: stdio (primary, for Claude Desktop / agent SDK); SSE HTTP optional via `XBERG_MCP_HTTP=1`
- All tools are registered in `mcp-server/src/index.ts` via `registerXxxTools(server)` calls
- Tool categories: extract, ingest, collection, document, query, pii, rehydrate, reports, stats, cache
- Every tool handler must validate its Zod schema before touching native bindings
- xberg extraction is accessed via `@xberg-io/xberg` (file dep → `crates/xberg-node`)
- RAG operations are accessed via `xberg-rag-node` (file dep → `crates/xberg-rag-node`)
- PII pipeline: detect → redact (token_replace/mask/hash) → store rehydration map (AES-256-GCM)
- Rehydration maps keyed by `XPII\x01` magic header; encrypted with scrypt-derived key per session
- `WarmupManager` in `warmup.ts` downloads models on server start; tools must not block on warmup
- Built with `npm run build` (tsup → `dist/`); tested with vitest; type-checked with `tsc --noEmit`
- Fork-local: `mcp-server/` is not in upstream; zero conflict risk on upstream pull
