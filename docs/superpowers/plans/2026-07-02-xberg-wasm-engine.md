# Xberg Shared WASM Engine Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build one `wasm32` binary (`XbergEngine`) that runs extract, OCR, NER, anonymization, and RAG in-browser and under Node, reaching ML/storage through JSPI-injected host interfaces.

**Architecture:** Extend the `xberg-wasm` crate with a stateful `XbergEngine` handle. Pure-Rust capability (extract, chunk, keywords, PII, redaction+AES-GCM rehydration, Tesseract OCR, Candle NER) runs in-binary; embeddings, vector store, and the PaddleOCR/GLiNER fast-paths are injected from JS and bridged via JSPI. Hybrid dispatch: injected fast-path when present, in-binary fallback otherwise.

**Tech Stack:** Rust 2024, `wasm-bindgen`/`wasm-bindgen-futures`, `xberg-rag` (`vector-store` + `pipeline`), `xberg-gliner-candle`, RustCrypto (`aes-gcm`, `scrypt`), `wasm-bindgen-test`.

**Spec:** [2026-07-02-xberg-wasm-engine-design.md](../specs/2026-07-02-xberg-wasm-engine-design.md)

> **Mechanism note:** Task titles say "JSPI bridge" — the implementation is **standard async `wasm-bindgen`** (`async fn` exports + `JsFuture` over the injected JS Promises), NOT `WebAssembly.Suspending`/`promising`. It is browser-portable (works beyond Chrome/Edge); JSPI is an optional future optimization. Read every "JSPI" in this plan as "async wasm-bindgen bridge".

## Global Constraints

- Rust 2024 edition; `cargo clippy -D warnings`; zero warnings.
- No `.unwrap()`/`panic!` in library code — `Result<T, E>` with `thiserror`; every engine method returns `Result<T, JsValue>`.
- Every `unsafe` block (none expected here) carries a `// SAFETY:` comment.
- `xberg-wasm` is Alef-generated — **its `Cargo.toml` is regenerated, not hand-edited as source**. Feature additions go through `alef.toml`; run `task alef:generate` and commit generator input + output together. Rust `src/` files under `xberg-wasm` that are NOT Alef-managed may be added directly (verify against `alef.toml` before editing).
- WASM build target: `wasm32-unknown-unknown`; verify with `cargo build -p xberg-wasm --target wasm32-unknown-unknown`.
- Cargo target dir is `E:/cargo-target` (Windows dev, per repo config).
- Rehydration container format is frozen: `XPII\x01 | salt(16) | iv(12) | tag(16) | ciphertext`; scrypt `N=32768, r=8, p=1`, 32-byte key. Must stay byte-compatible with `mcp-server/src/redaction/rehydration.ts`.
- Commit messages: conventional commits, imperative, <72 chars; **no AI attribution** (repo `no-ai-signatures` rule).
- Run `prek run --all-files` before each commit; re-stage if hooks rewrite.

---

### Task 1: Consume `ner-candle-wasm` (delivered by plan A)

**Prerequisite:** [plan A — ner-candle-wasm](2026-07-02-ner-candle-wasm.md) must be complete. Plan A makes `xberg-gliner`'s tokenizer surface ORT-free, adds `Gliner2Candle::from_bytes`, and ships the `ner-candle-wasm` feature on `xberg` (in `wasm-target`) plus the `WasmCandleNer` adapter. This task only confirms the engine build picks it up. (Do NOT "gate xberg-gliner out" — plan A keeps its tokenizer/encoder; that earlier instruction was wrong.)

**Files:**
- Verify only: `crates/xberg-wasm/Cargo.toml` lists `ner-candle-wasm` via `wasm-target`.
- Test: build command (compile-validation).

**Interfaces:**
- Consumes: `xberg::text::ner_candle_wasm::WasmCandleNer` (from plan A Task 3), used later by Task 6's in-binary NER fallback.

- [ ] **Step 1: Confirm the feature is wired**

Run: `task alef:generate && grep -n "ner-candle-wasm" crates/xberg-wasm/Cargo.toml`
Expected: `ner-candle-wasm` appears under `wasm-target`. If absent, plan A is not complete — stop and finish A first.

- [ ] **Step 2: Build the wasm engine with the full target feature set**

Run: `cargo build -p xberg-wasm --target wasm32-unknown-unknown --no-default-features --features wasm-target 2>&1 | tail -40`
Expected: SUCCESS. If plan A recorded `tokenizers`/`candle` as wasm-infeasible, `ner-candle-wasm` is excluded from `wasm-target` there — this build still succeeds and Task 6's in-binary NER fallback tests become `#[ignore]`, injected ORT-Web NER remaining primary.

- [ ] **Step 3: Commit (only if `alef:generate` changed generated output)**

```bash
prek run --all-files
git add crates/xberg-wasm/Cargo.toml
git commit -m "chore(wasm): confirm ner-candle-wasm in engine target set"
```

---

### Task 2: Port rehydration crypto to Rust (`anon` module)

Move the AES-256-GCM encrypted-map crypto from TypeScript (`mcp-server/src/redaction/rehydration.ts`) into pure Rust so it runs in-wasm and in Node identically. This is the most self-contained task; do it early.

**Files:**
- Create: `crates/xberg/src/text/anon_crypto.rs`
- Modify: `crates/xberg/src/text/mod.rs` (add `pub mod anon_crypto;` under the `redaction` feature)
- Modify: `crates/xberg/Cargo.toml` (add `aes-gcm`, `scrypt` under the `redaction` feature — via `alef.toml`/workspace deps as appropriate)
- Test: `crates/xberg/src/text/anon_crypto.rs` (`#[cfg(test)]` module)

**Interfaces:**
- Produces:
  - `pub fn encrypt_map(map: &BTreeMap<String, String>, passphrase: &str) -> Result<Vec<u8>, AnonError>`
  - `pub fn decrypt_map(bytes: &[u8], passphrase: &str) -> Result<BTreeMap<String, String>, AnonError>`
  - `pub enum AnonError { BadMagic, Truncated, Decrypt, Serde(String) }` (impl `std::error::Error`)

- [ ] **Step 1: Add crypto dependencies**

In `crates/xberg/Cargo.toml` `[dependencies]` (or `alef.toml` source + `[workspace.dependencies]`):

```toml
aes-gcm = { version = "0.10", optional = true }
scrypt = { version = "0.11", default-features = false, optional = true }
```

Add both to the `redaction` feature list: `redaction = [..., "dep:aes-gcm", "dep:scrypt"]`.

- [ ] **Step 2: Write the failing round-trip test**

Create `crates/xberg/src/text/anon_crypto.rs` with only the test first:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn encrypt_then_decrypt_roundtrips() {
        let mut map = BTreeMap::new();
        map.insert("[EMAIL_1]".to_string(), "jane@doe.com".to_string());
        map.insert("[PHONE_1]".to_string(), "+15551234567".to_string());

        let blob = encrypt_map(&map, "correct horse battery staple").unwrap();
        assert_eq!(&blob[0..5], b"XPII\x01");

        let back = decrypt_map(&blob, "correct horse battery staple").unwrap();
        assert_eq!(back, map);
    }

    #[test]
    fn wrong_passphrase_fails_auth() {
        let mut map = BTreeMap::new();
        map.insert("[EMAIL_1]".to_string(), "jane@doe.com".to_string());
        let blob = encrypt_map(&map, "right").unwrap();
        assert!(matches!(decrypt_map(&blob, "wrong"), Err(AnonError::Decrypt)));
    }

    #[test]
    fn rejects_bad_magic() {
        assert!(matches!(decrypt_map(b"NOPE............", "x"), Err(AnonError::BadMagic)));
    }
}
```

- [ ] **Step 3: Run test to verify it fails**

Run: `cargo test -p xberg --features redaction anon_crypto 2>&1 | tail -20`
Expected: FAIL — `encrypt_map`/`decrypt_map`/`AnonError` not found.

- [ ] **Step 4: Implement the module**

Prepend to `crates/xberg/src/text/anon_crypto.rs` (mirrors `rehydration.ts` byte layout exactly):

```rust
//! AES-256-GCM encrypted rehydration maps. Byte-compatible with the historic
//! TypeScript `XPII\x01 | salt(16) | iv(12) | tag(16) | ciphertext` container.

use std::collections::BTreeMap;

use aes_gcm::aead::{Aead, KeyInit, Payload};
use aes_gcm::{Aes256Gcm, Key, Nonce};
use scrypt::{scrypt, Params};

const MAGIC: &[u8; 5] = b"XPII\x01";
const SALT_LEN: usize = 16;
const IV_LEN: usize = 12;
const TAG_LEN: usize = 16;
const KEY_LEN: usize = 32;

#[derive(Debug)]
pub enum AnonError {
    BadMagic,
    Truncated,
    Decrypt,
    Serde(String),
}

impl std::fmt::Display for AnonError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AnonError::BadMagic => write!(f, "not an XPII map file"),
            AnonError::Truncated => write!(f, "map file truncated"),
            AnonError::Decrypt => write!(f, "decryption failed (wrong passphrase or corrupt data)"),
            AnonError::Serde(e) => write!(f, "map (de)serialization failed: {e}"),
        }
    }
}
impl std::error::Error for AnonError {}

fn derive_key(passphrase: &str, salt: &[u8]) -> [u8; KEY_LEN] {
    // scrypt N=2^14 (16384), r=8, p=1 → matches Node `scryptSync(pw, salt, 32)`
    // DEFAULTS as used by rehydration.ts (no options passed). NOT 2^15: the
    // XPII container does not store cost params, so this MUST equal Node's
    // default cost or existing map files fail to decrypt. (The CLAUDE.md
    // `pii-pipeline` note says N=32768, but the shipped TS code uses the
    // default 16384 — the code is authoritative for on-disk compatibility.)
    let params = Params::new(14, 8, 1, KEY_LEN).expect("valid scrypt params");
    let mut key = [0u8; KEY_LEN];
    scrypt(passphrase.as_bytes(), salt, &params, &mut key).expect("scrypt into 32 bytes");
    key
}

fn random_bytes<const N: usize>() -> [u8; N] {
    let mut buf = [0u8; N];
    getrandom::fill(&mut buf).expect("getrandom");
    buf
}

pub fn encrypt_map(map: &BTreeMap<String, String>, passphrase: &str) -> Result<Vec<u8>, AnonError> {
    let plain = serde_json::to_vec(map).map_err(|e| AnonError::Serde(e.to_string()))?;
    let salt = random_bytes::<SALT_LEN>();
    let iv = random_bytes::<IV_LEN>();
    let key = derive_key(passphrase, &salt);
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&key));
    // aes-gcm appends the 16-byte tag to the ciphertext; the TS layout stores tag
    // BEFORE ciphertext, so split it back out.
    let mut ct_and_tag = cipher
        .encrypt(Nonce::from_slice(&iv), Payload { msg: &plain, aad: &[] })
        .map_err(|_| AnonError::Decrypt)?;
    let tag = ct_and_tag.split_off(ct_and_tag.len() - TAG_LEN);

    let mut out = Vec::with_capacity(MAGIC.len() + SALT_LEN + IV_LEN + TAG_LEN + ct_and_tag.len());
    out.extend_from_slice(MAGIC);
    out.extend_from_slice(&salt);
    out.extend_from_slice(&iv);
    out.extend_from_slice(&tag);
    out.extend_from_slice(&ct_and_tag);
    Ok(out)
}

pub fn decrypt_map(bytes: &[u8], passphrase: &str) -> Result<BTreeMap<String, String>, AnonError> {
    let header = MAGIC.len() + SALT_LEN + IV_LEN + TAG_LEN;
    if bytes.len() < header {
        return if bytes.len() >= MAGIC.len() && &bytes[..MAGIC.len()] != MAGIC {
            Err(AnonError::BadMagic)
        } else {
            Err(AnonError::Truncated)
        };
    }
    if &bytes[..MAGIC.len()] != MAGIC {
        return Err(AnonError::BadMagic);
    }
    let mut off = MAGIC.len();
    let salt = &bytes[off..off + SALT_LEN];
    off += SALT_LEN;
    let iv = &bytes[off..off + IV_LEN];
    off += IV_LEN;
    let tag = &bytes[off..off + TAG_LEN];
    off += TAG_LEN;
    let ct = &bytes[off..];

    let key = derive_key(passphrase, salt);
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&key));
    // Re-append tag to match aes-gcm's expected [ciphertext || tag] layout.
    let mut ct_and_tag = Vec::with_capacity(ct.len() + TAG_LEN);
    ct_and_tag.extend_from_slice(ct);
    ct_and_tag.extend_from_slice(tag);
    let plain = cipher
        .decrypt(Nonce::from_slice(iv), Payload { msg: &ct_and_tag, aad: &[] })
        .map_err(|_| AnonError::Decrypt)?;
    serde_json::from_slice(&plain).map_err(|e| AnonError::Serde(e.to_string()))
}
```

Add `pub mod anon_crypto;` to `crates/xberg/src/text/mod.rs` gated with `#[cfg(feature = "redaction")]`. Ensure `getrandom` is a dependency of `xberg` (it is transitively; add an explicit `getrandom = "0.3"` with `features=["wasm_js"]` under the wasm target if the build complains).

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test -p xberg --features redaction anon_crypto 2>&1 | tail -20`
Expected: PASS (3 tests).

- [ ] **Step 6: Add the TS-compatibility cross-check fixture**

Generate a map file with the existing TS code and assert Rust decrypts it. Add:

```rust
#[test]
fn decrypts_map_produced_by_typescript() {
    // Fixture bytes produced by mcp-server encryptMapFile(map, "pw") with
    // map = {"[EMAIL_1]":"a@b.com"}. Committed as a hex constant to prove
    // cross-language format compatibility.
    let hex = include_str!("testdata/ts_map_email.hex");
    let bytes = hex_decode(hex.trim());
    let back = decrypt_map(&bytes, "pw").unwrap();
    assert_eq!(back.get("[EMAIL_1]").map(String::as_str), Some("a@b.com"));
}
```

Generate the fixture (build the TS server first — `dist/` may not exist): `cd mcp-server && npm run build && node -e 'import("./dist/redaction/rehydration.js").then(m=>{const fs=require("fs");m.encryptMapFile("/tmp/x.xpii",{"[EMAIL_1]":"a@b.com"},"pw");console.log(fs.readFileSync("/tmp/x.xpii").toString("hex"))})'` → save stdout to `crates/xberg/src/text/testdata/ts_map_email.hex`. Add a tiny `hex_decode` test helper.

Run: `cargo test -p xberg --features redaction anon_crypto`
Expected: PASS (4 tests). If the cross-check fails, the scrypt cost params differ — reconcile `Params` with the Node `scryptSync` cost actually used, then re-run.

- [ ] **Step 7: Commit**

```bash
prek run --all-files
git add crates/xberg/src/text/anon_crypto.rs crates/xberg/src/text/mod.rs crates/xberg/src/text/testdata/ crates/xberg/Cargo.toml alef.toml
git commit -m "feat(redaction): pure-Rust AES-GCM rehydration map crypto"
```

---

### Task 3: wasm-conditional `Embedder` + synchronous wasm ingest path

The engine cannot use `xberg_rag::ingest_document` on wasm (`spawn_blocking` + `Send` embedder). Add a wasm-safe `Embedder` bound and a `spawn_blocking`-free ingest.

**Files:**
- Modify: `crates/xberg-rag/src/pipeline.rs:37-41` (trait attr) and `:115-130` (ingest)
- Test: `crates/xberg-rag/src/pipeline.rs` (`#[cfg(test)]`)

**Interfaces:**
- Consumes: `VectorStore` (existing), `IngestRequest`, `RagPipelineConfig`.
- Produces: `ingest_document` compiles on wasm; on wasm the `Embedder` bound is `async_trait(?Send)`; new `ingest_document_local` (no `spawn_blocking`) usable on both targets.

- [ ] **Step 1: Make the `Embedder` trait wasm-conditional**

Replace the `#[async_trait]` attribute on `Embedder` (pipeline.rs:37) with:

```rust
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
pub trait Embedder: 'static {
    async fn embed(&self, texts: Vec<String>) -> RagResult<Vec<Vec<f32>>>;
}
```

(Drop the unconditional `Send + Sync` supertrait; keep it only off-wasm via a separate `#[cfg(not(target_arch="wasm32"))]` bound where `Arc<dyn Embedder>` crosses threads, if any caller needs it.)

- [ ] **Step 2: Write the failing test for the sync ingest path**

Add to the pipeline test module (uses the existing `StubEmbedder`/in-memory store test scaffolding at pipeline.rs:282+):

```rust
#[tokio::test]
async fn ingest_document_local_upserts_without_spawn_blocking() {
    let store = std::sync::Arc::new(crate::backends::memory::InMemoryStore::new("t"));
    store.ensure_collection(&test_spec(DIM)).await.unwrap();
    let embedder = StubEmbedder { dim: DIM as usize };
    let req = IngestRequest { full_text: "hello world. second sentence.".into(), ..Default::default() };
    let cfg = RagPipelineConfig { chunking: &xberg::ChunkingConfig::default() };
    let id = ingest_document_local(store.clone(), "docs", req, &cfg, &embedder).await.unwrap();
    assert!(!id.0.is_empty());
}
```

- [ ] **Step 3: Run to verify it fails**

Run: `cargo test -p xberg-rag --features pipeline,in-memory ingest_document_local 2>&1 | tail`
Expected: FAIL — `ingest_document_local` not found.

- [ ] **Step 4: Implement `ingest_document_local`**

Add next to `ingest_document`. Identical except chunking runs inline (no `spawn_blocking`):

```rust
/// Like [`ingest_document`] but chunks inline (no `tokio::task::spawn_blocking`),
/// so it compiles and runs on `wasm32` where the multi-thread runtime is absent.
pub async fn ingest_document_local(
    store: Arc<dyn VectorStore>,
    collection: &str,
    request: IngestRequest,
    config: &RagPipelineConfig<'_>,
    embedder: &dyn Embedder,
) -> RagResult<DocumentId> {
    let chunks = xberg::chunking::chunk_for_rag(&request.full_text, config.chunking)
        .map_err(RagError::Core)?
        .chunks;
    let texts: Vec<String> = chunks.iter().map(|c| c.content.clone()).collect();
    let embeddings = embedder.embed(texts).await?;
    let records: Vec<ChunkRecord> = chunks
        .into_iter()
        .zip(embeddings)
        .enumerate()
        .map(|(i, (c, e))| chunk_to_record(c, i as u32, e))
        .collect();
    let doc = DocumentRecord::from(&request);
    store.upsert_document(collection, &doc, &records).await
}
```

(If `DocumentRecord::from(&IngestRequest)` does not exist, build the `DocumentRecord` inline from `request` fields — check `types.rs` for the constructor and match it exactly.)

**Gate the `spawn_blocking` path off wasm.** The existing `ingest_document` calls `tokio::task::spawn_blocking` (pipeline.rs:125), which does not exist on `wasm32` and would break the wasm build even though the engine never calls it. Annotate it:

```rust
#[cfg(not(target_arch = "wasm32"))]
pub async fn ingest_document(/* ...unchanged signature... */) -> RagResult<DocumentId> {
    // ...existing body unchanged...
}
```

Then make `tokio` a non-wasm dependency of `xberg-rag`. In `crates/xberg-rag/Cargo.toml`, move `tokio` out of `[dependencies]` for the `pipeline` feature into a target block, OR make the `pipeline` feature not pull `tokio` on wasm. Concretely, change the `pipeline` feature to drop `dep:tokio` and add tokio only where the gated code needs it:

```toml
pipeline = ["vector-store", "dep:xberg", "xberg/chunking", "dep:futures"]

[target.'cfg(not(target_arch = "wasm32"))'.dependencies]
tokio = { workspace = true, optional = true }
```

and gate any other `tokio::` use in the pipeline module behind `#[cfg(not(target_arch = "wasm32"))]`.

- [ ] **Step 5: Run to verify pass (native) and wasm compiles**

Run: `cargo test -p xberg-rag --features pipeline,in-memory ingest_document_local 2>&1 | tail`
Expected: PASS.
Run: `cargo build -p xberg-rag --target wasm32-unknown-unknown --no-default-features --features vector-store,pipeline 2>&1 | tail`
Expected: SUCCESS (no `spawn_blocking`/`Send` errors).

- [ ] **Step 6: Commit**

```bash
prek run --all-files
git add crates/xberg-rag/src/pipeline.rs
git commit -m "feat(rag): wasm-safe Embedder bound and inline ingest path"
```

---

### Task 4: JSPI `Embedder` bridge in `xberg-wasm`

Implement `xberg_rag::Embedder` over an injected JS object, suspending on its async `embed()` via JSPI.

**Files:**
- Create: `crates/xberg-wasm/src/bridge/mod.rs`, `crates/xberg-wasm/src/bridge/embedder.rs`
- Modify: `crates/xberg-wasm/src/lib.rs` (add `mod bridge;` — verify lib.rs is not fully Alef-generated; if it is, add a non-generated `src/bridge/` module referenced from a hand-owned include point per `alef.toml`)
- Test: `crates/xberg-wasm/tests/embedder_bridge.rs` (`wasm-bindgen-test`)

**Interfaces:**
- Consumes: injected JS `{ embed(texts: string[]): Promise<Float32Array[]> }`.
- Produces: `pub struct JsEmbedder` implementing `xberg_rag::Embedder`; `JsEmbedder::new(js_obj: js_sys::Object) -> JsEmbedder`.

- [ ] **Step 0a: Add `rlib` to `xberg-wasm` crate-type (required for integration tests)**

`xberg-wasm` is currently `crate-type = ["cdylib"]` only (Cargo.toml:29). Integration tests under `tests/` link against the crate as an `rlib` and will FAIL to link without it (the `wasm-constraints` doc mandates `["cdylib", "rlib"]`). This manifest is Alef-generated — edit `alef.toml` so the generated `[lib]` emits both, then `task alef:generate`:

```toml
[lib]
crate-type = ["cdylib", "rlib"]
```

Verify: `grep -A1 '\[lib\]' crates/xberg-wasm/Cargo.toml` shows `["cdylib", "rlib"]`.

- [ ] **Step 0b: Add the `xberg-rag` dependency to `xberg-wasm`**

The bridges use `xberg_rag::{Embedder, VectorStore, RagError, ...}`; `xberg-wasm` has no `xberg-rag` dependency yet. Add it via `alef.toml` (generated manifest), ORT-free features only:

```toml
xberg-rag = { path = "../xberg-rag", default-features = false, features = ["vector-store", "pipeline"] }
```

Run `task alef:generate`; verify `xberg-rag` appears under `[dependencies]` in `crates/xberg-wasm/Cargo.toml`. Also add `async-trait` and `serde-wasm-bindgen` if not already present (serde-wasm-bindgen is already a dep; async-trait is already a dep).

- [ ] **Step 1: Write the failing wasm test**

Create `crates/xberg-wasm/tests/embedder_bridge.rs`:

```rust
#![cfg(target_arch = "wasm32")]
use wasm_bindgen_test::*;
wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
async fn js_embedder_returns_vectors() {
    // A JS stub embedder returning a fixed 2-dim vector per input.
    let stub = js_sys::eval(
        "({ embed: async (t) => t.map(() => new Float32Array([0.1, 0.2])) })"
    ).unwrap().dyn_into::<js_sys::Object>().unwrap();
    let emb = xberg_wasm::bridge::embedder::JsEmbedder::new(stub);
    let out = emb.embed(vec!["a".into(), "b".into()]).await.unwrap();
    assert_eq!(out.len(), 2);
    assert_eq!(out[0], vec![0.1_f32, 0.2]);
}
```

- [ ] **Step 2: Run to verify it fails**

Run: `wasm-pack test --headless --chrome crates/xberg-wasm -- --test embedder_bridge 2>&1 | tail -30`
Expected: FAIL — `JsEmbedder` not found.

- [ ] **Step 3: Implement the bridge**

`crates/xberg-wasm/src/bridge/mod.rs`:

```rust
pub mod embedder;
```

`crates/xberg-wasm/src/bridge/embedder.rs`:

```rust
//! JSPI bridge implementing `xberg_rag::Embedder` over an injected JS embedder.

use async_trait::async_trait;
use js_sys::{Array, Float32Array, Object, Reflect};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use xberg_rag::pipeline::Embedder;
use xberg_rag::error::{RagError, RagResult};

pub struct JsEmbedder {
    inner: Object,
}

impl JsEmbedder {
    pub fn new(inner: Object) -> Self {
        Self { inner }
    }
}

#[async_trait(?Send)]
impl Embedder for JsEmbedder {
    async fn embed(&self, texts: Vec<String>) -> RagResult<Vec<Vec<f32>>> {
        let js_texts = Array::new();
        for t in &texts {
            js_texts.push(&JsValue::from_str(t));
        }
        let f = Reflect::get(&self.inner, &JsValue::from_str("embed"))
            .map_err(js_err)?
            .dyn_into::<js_sys::Function>()
            .map_err(|_| RagError::Backend("injected embedder has no embed()".into()))?;
        let promise = f.call1(&self.inner, &js_texts).map_err(js_err)?;
        let result = JsFuture::from(js_sys::Promise::from(promise)).await.map_err(js_err)?;
        let arr: Array = result.dyn_into().map_err(|_| RagError::Backend("embed() did not resolve to an array".into()))?;
        let mut out = Vec::with_capacity(arr.length() as usize);
        for v in arr.iter() {
            let f32arr: Float32Array = v.dyn_into().map_err(|_| RagError::Backend("embed() row is not Float32Array".into()))?;
            out.push(f32arr.to_vec());
        }
        Ok(out)
    }
}

fn js_err(v: JsValue) -> RagError {
    RagError::Backend(format!("JS embedder error: {v:?}").into())
}
```

(Match `RagError::Backend`'s actual constructor signature from `error.rs` — adjust `.into()` accordingly. Add `mod bridge;` to `lib.rs` at the hand-owned insertion point.)

- [ ] **Step 4: Run to verify it passes**

Run: `wasm-pack test --headless --chrome crates/xberg-wasm -- --test embedder_bridge 2>&1 | tail -20`
Expected: PASS. (Requires Chrome; JSPI suspension is exercised via `JsFuture`.)

- [ ] **Step 5: Commit**

```bash
prek run --all-files
git add crates/xberg-wasm/src/bridge/ crates/xberg-wasm/src/lib.rs crates/xberg-wasm/tests/embedder_bridge.rs
git commit -m "feat(wasm): JSPI Embedder bridge over injected JS"
```

---

### Task 5: JSPI `VectorStore` bridge

Implement the wasm-aware `VectorStore` trait over an injected JS store object.

**Files:**
- Create: `crates/xberg-wasm/src/bridge/store.rs`
- Modify: `crates/xberg-wasm/src/bridge/mod.rs` (add `pub mod store;`)
- Test: `crates/xberg-wasm/tests/store_bridge.rs`

**Interfaces:**
- Consumes: injected JS `{ upsertDocument, query, ensureCollection, dropCollection, getCollection, deleteDocuments, ... }` returning Promises; JSON-serialized args.
- Produces: `pub struct JsVectorStore` implementing `xberg_rag::VectorStore`; `JsVectorStore::new(Object) -> Self`.

- [ ] **Step 1: Write the failing wasm test** — ensure_collection + upsert + query round-trip against a JS in-memory stub (eval a small object holding a Map). Assert `query` returns the upserted chunk. (Full stub JS written inline in the test, mirroring the six methods used.)

- [ ] **Step 2: Run to verify it fails** — `wasm-pack test --headless --chrome crates/xberg-wasm -- --test store_bridge`; Expected: FAIL (`JsVectorStore` not found).

- [ ] **Step 3: Implement `JsVectorStore`** — one method per `VectorStore` trait fn (signatures from [store.rs:38-90+](../../../crates/xberg-rag/src/store.rs)). Each serializes args with `serde_wasm_bindgen::to_value`, calls the JS method, `JsFuture::from(promise).await`, deserializes the result with `serde_wasm_bindgen::from_value`. Use `#[async_trait(?Send)]`. `name()` returns a stored `String`; `capabilities()` returns `Capabilities` describing vector+filter support.

- [ ] **Step 4: Run to verify it passes** — same command; Expected: PASS.

- [ ] **Step 5: Commit**

```bash
prek run --all-files
git add crates/xberg-wasm/src/bridge/store.rs crates/xberg-wasm/src/bridge/mod.rs crates/xberg-wasm/tests/store_bridge.rs
git commit -m "feat(wasm): JSPI VectorStore bridge over injected JS"
```

---

### Task 6: NER + OCR hybrid bridges with in-binary fallback

Optional injected `ner()`/`ocr()`; fall back to in-binary Candle / Tesseract when absent.

**Files:**
- Create: `crates/xberg-wasm/src/bridge/ner.rs`, `crates/xberg-wasm/src/bridge/ocr.rs`
- Modify: `crates/xberg-wasm/src/bridge/mod.rs`
- Test: `crates/xberg-wasm/tests/hybrid_dispatch.rs`

**Interfaces:**
- Consumes: optional injected JS `{ ner(text, opts): Promise<Entity[]> }`, `{ ocr(bytes, opts): Promise<OcrResult> }`.
- Produces: `pub async fn resolve_ner(injected: Option<Object>, text: &str, opts: &NerOpts) -> Result<Vec<Entity>, JsValue>` and `resolve_ocr(...)`, each dispatching injected-first, fallback-second.

- [ ] **Step 1: Write two failing tests** — (a) with an injected stub `ner`, `resolve_ner` returns the stub's entities; (b) with `None`, `resolve_ner` returns the in-binary Candle result on a fixture (skip/`#[ignore]` if Task 1 recorded Candle-wasm as deferred — leave the test asserting the "NER unavailable offline" typed error instead). Mirror for OCR (injected stub vs in-binary Tesseract on a tiny PNG fixture).

- [ ] **Step 2: Run to verify they fail** — `wasm-pack test --headless --chrome crates/xberg-wasm -- --test hybrid_dispatch`; Expected: FAIL.

- [ ] **Step 3: Implement `resolve_ner`/`resolve_ocr`** — `match injected { Some(obj) => call JS via JSFuture, None => call in-binary path }`. In-binary NER calls `xberg`'s Candle NER entrypoint under `#[cfg(feature = "ner-candle-wasm")]`, else returns `Err(JsValue::from_str("NER unavailable: no injected backend and ner-candle-wasm disabled"))`. In-binary OCR calls the existing `ocr-wasm` Tesseract path already reachable from `xberg`.

- [ ] **Step 4: Run to verify they pass** — same command; Expected: PASS (fallback tests may be `#[ignore]` per Task 1 outcome).

- [ ] **Step 5: Commit**

```bash
prek run --all-files
git add crates/xberg-wasm/src/bridge/ner.rs crates/xberg-wasm/src/bridge/ocr.rs crates/xberg-wasm/src/bridge/mod.rs crates/xberg-wasm/tests/hybrid_dispatch.rs
git commit -m "feat(wasm): hybrid NER/OCR dispatch with in-binary fallback"
```

---

### Task 7: `XbergEngine` handle

Wire the public stateful API over the bridges + in-binary capability.

**Files:**
- Create: `crates/xberg-wasm/src/engine.rs`, `crates/xberg-wasm/src/anon.rs`
- Modify: `crates/xberg-wasm/src/lib.rs` (`mod engine; mod anon;`)
- Test: `crates/xberg-wasm/tests/engine.rs`

**Interfaces:**
- Consumes: `JsEmbedder`, `JsVectorStore`, `resolve_ner`, `resolve_ocr`, `xberg::text::anon_crypto`, `ingest_document_local`.
- Produces (wasm-bindgen):
  - `#[wasm_bindgen] impl XbergEngine { #[wasm_bindgen(constructor)] pub fn new(config: JsValue, injection: JsValue) -> Result<XbergEngine, JsValue> }`
  - `async fn extract(&self, input, config) -> Result<JsValue, JsValue>`
  - `async fn ocr(&self, bytes: Vec<u8>, opts: JsValue) -> Result<JsValue, JsValue>`
  - `fn detect_pii(&self, text: String) -> Result<JsValue, JsValue>`
  - `fn redact(&self, text: String, strategy: JsValue) -> Result<JsValue, JsValue>`
  - `fn rehydrate(&self, doc: String, map_bytes: Vec<u8>, passphrase: String) -> Result<String, JsValue>`
  - `async fn ner(&self, text: String, opts: JsValue) -> Result<JsValue, JsValue>`
  - `async fn ingest(&self, doc: JsValue, collection: String) -> Result<JsValue, JsValue>`
  - `async fn query(&self, q: String, collection: String, k: u32) -> Result<JsValue, JsValue>`

- [ ] **Step 1: Write the failing integration test** — construct `XbergEngine` with JS-stub embedder+store (from Tasks 4–5 patterns), no ner/ocr; call `ingest` on a doc then `query`, assert a chunk returns; call `redact(text, "token_replace")` then `rehydrate` round-trips. Full stub JS + assertions inline.

- [ ] **Step 2: Run to verify it fails** — `wasm-pack test --headless --chrome crates/xberg-wasm -- --test engine`; Expected: FAIL.

- [ ] **Step 3: Implement `XbergEngine`** — `new` parses `injection` (`Reflect::get` for `embedder`,`store`,`ner?`,`ocr?`), builds `JsEmbedder`/`JsVectorStore`, stores `Option<Object>` for ner/ocr. `ingest` builds `IngestRequest` (extract→PII detect→redact per config→) then `ingest_document_local`. `query` embeds via `JsEmbedder` then `store.query`. `rehydrate` calls `anon::rehydrate` which uses `anon_crypto::decrypt_map` + token substitution. `redact`/`detect_pii` delegate to core `redaction`. Each method maps core errors to `JsValue` preserving message+code.

- [ ] **Step 4: Run to verify it passes** — same command; Expected: PASS.

- [ ] **Step 5: Build the release wasm and check it loads** — `wasm-pack build crates/xberg-wasm --target web --out-dir pkg 2>&1 | tail`; Expected: SUCCESS, `pkg/xberg_wasm_bg.wasm` emitted.

- [ ] **Step 6: Commit**

```bash
prek run --all-files
git add crates/xberg-wasm/src/engine.rs crates/xberg-wasm/src/anon.rs crates/xberg-wasm/src/lib.rs crates/xberg-wasm/tests/engine.rs
git commit -m "feat(wasm): XbergEngine handle over injected bridges"
```

---

### Task 8: Freshness + parity guardrails

**Files:**
- Modify: `crates/xberg-wasm/tests/parity.rs` (create)
- Test: parity of PII/redaction Rust-vs-TS on a shared fixture

- [ ] **Step 1: Write parity test** — same input text → assert `detect_pii` categories/counts match a committed expected JSON (derived from the current TS `detect.ts` output on the same fixture).

- [ ] **Step 2: Run** — `wasm-pack test --headless --chrome crates/xberg-wasm -- --test parity`; Expected: PASS.

- [ ] **Step 3: Verify Alef freshness** — `task alef:generate && git diff --exit-code crates/xberg-wasm/Cargo.toml`; Expected: no diff (generated manifest matches `alef.toml`).

- [ ] **Step 4: Commit**

```bash
prek run --all-files
git add crates/xberg-wasm/tests/parity.rs
git commit -m "test(wasm): PII/redaction parity guardrail"
```

---

## Self-Review Notes

- **Spec coverage:** §2 crate structure → Tasks 1,4,5,6,7. §3 API contract → Task 7. §4 injection seam/JSPI → Tasks 4,5,6. §5 capability placement → Tasks 6,7. §6 anon crypto → Task 2. §7 data flow → Task 7. §9 testing → Tasks 2–8. §A ner-candle-wasm → Task 1.
- **Open risk carried from spec:** Task 1 is a genuine build-validation gate; if Candle-wasm cannot compile, in-binary NER defers and Task 6's fallback tests become `#[ignore]` with the injected path remaining primary. This is explicit, not a placeholder.
- **Type consistency:** `JsEmbedder`/`JsVectorStore`/`resolve_ner`/`resolve_ocr`/`ingest_document_local`/`encrypt_map`/`decrypt_map` used consistently across tasks. Bridge error mapping targets `RagError::Backend(Box<dyn Error + Send + Sync>)` (verified error.rs:113) — `String.into()` works.

- **Review corrections applied (2026-07-02):**
  - **H1** scrypt cost fixed to `Params::new(14, ...)` (N=16384) to match Node `scryptSync` defaults used by `rehydration.ts` — the earlier N=32768 (2^15) would never decrypt existing maps. (Project `pii-pipeline` rule says 32768; the shipped TS uses the default 16384 — code is authoritative for on-disk compat. If the team wants 32768, that's a separate re-encryption migration + TS change.)
  - **H2** Task 4 Step 0a adds `crate-type = ["cdylib", "rlib"]` via `alef.toml` — integration tests need the `rlib`.
  - **H3** Task 4 Step 0b adds the `xberg-rag` dependency to `xberg-wasm` (was missing).
  - **M1** Task 3 now gates `ingest_document` (spawn_blocking) and `tokio` off wasm.
  - **M4** async mechanism is standard `wasm-bindgen` (`JsFuture`), not JSPI — see the Mechanism note at the top; browser-portable, Chrome/Edge is a product (WebGPU/OPFS/COEP) choice only.
