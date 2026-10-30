---
title: "Architecture Decisions"
---

xberg records its lasting decisions as MADR architecture decision records in
[`adrs/`](https://github.com/xberg-io/xberg/tree/main/adrs), checked by
[adrlint](https://github.com/Goldziher/adrlint). An accepted record is not edited: a later decision supersedes or
amends it. Organization-wide decisions live in the
[org decision log](https://github.com/xberg-io/agent-conventions/blob/main/docs/adr-index.md) and are cited, not
repeated, here. This index sits outside `adrs/` because adrlint lints every Markdown file in the log; add a row in the
same change as a new record.

| ADR | Title | Status | Date |
|-----|-------|--------|------|
| [0000](https://github.com/xberg-io/xberg/blob/main/adrs/0000-record-architecture-decisions.md) | Record architecture decisions | accepted | 2026-10-30 |
| [0001](https://github.com/xberg-io/xberg/blob/main/adrs/0001-synchronous-extraction-paths-on-wasm.md) | Synchronous extraction paths on WASM | accepted | 2025-12-13 |
| [0002](https://github.com/xberg-io/xberg/blob/main/adrs/0002-pure-rust-native-pdf-engine-by-default.md) | Pure-Rust native PDF engine by default | accepted | 2026-08-19 |
| [0003](https://github.com/xberg-io/xberg/blob/main/adrs/0003-java-on-panama-ffm-jni-only-for-kotlin-android.md) | Java on Panama FFM, JNI only for Kotlin Android | accepted | 2026-05-15 |
| [0004](https://github.com/xberg-io/xberg/blob/main/adrs/0004-rust-only-engine-seams-for-the-enterprise-boundary.md) | Rust-only engine seams for the enterprise boundary | accepted | 2026-06-29 |
| [0005](https://github.com/xberg-io/xberg/blob/main/adrs/0005-engine-neutral-inference-seam-over-ort-and-tract.md) | Engine-neutral inference seam over ORT and tract | accepted | 2026-07-19 |
