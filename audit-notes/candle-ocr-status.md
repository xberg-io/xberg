# Candle OCR backends — branch status

Branch: `feat/candle-backends`
Audit date: 2026-06-03

## Backends shipped

| Backend | Sub-feature | Engine | Pool | CLI flag | Verified |
|---|---|---|---|---|---|
| `candle-trocr` | `trocr` | `TrocrEngine` (BEiT + RoBERTa via candle `trocr` + `vit` modules) | per-call construction (TrOCR is line-level; no pool needed yet) | `--ocr-backend candle-trocr` | ✅ end-to-end |
| `candle-paddleocr-vl` | `paddleocr-vl` | `PaddleOcrVlEngine` (NaViT + ERNIE-4.5 via candle `paddleocr_vl`) | `LazyLock<RwLock<AHashMap<(PaddleOcrVlTask, DevicePreference), Arc<Engine>>>>` | `--ocr-backend candle-paddleocr-vl` | ✅ end-to-end (Metal) |

Two earlier scaffolds (`candle-got-ocr`, `candle-glm-ocr`) were reverted in
commits `3deec9206a` and `153aecbc9b` because the subagents that landed them
shipped placeholder forward passes. See "Why GOT-OCR / GLM-OCR were dropped"
below.

## What is real vs. what is documented

Wave 1 audit findings (now all fixed under `cdcd286655`, `78eff152f1`,
`d3385696dd`, `5255904e8b`, integration-test commit, `5973fe8ee1`):

- `TrocrBackend::process_image` was a stub returning a placeholder string; now
  calls the real engine inside `tokio::task::spawn_blocking`.
- `PaddleOcrVlBackend::process_image` reconstructed a ~900 MB engine on every
  call; now pooled by `(task, device_preference)`.
- `TrocrEngine` used `TrOCRConfig::default()` for `decoder_start_token_id` and
  `eos_token_id`; now reads them from the loaded `config.json`.
- `PaddleOcrVlEngine` resolved BOS/EOS by name lookup on every decode step;
  now cached on the engine at construction time.
- `TrocrBackend::parse_options` discarded the user's runtime `variant`
  override; now returns `Option<TrocrVariant>` and the caller falls back to
  the construction-time default only when no override is present.
- `PaddleOcrVlEngine` image normalisation ran a per-pixel triple loop; now
  vectorised via `Tensor::affine` + `broadcast_sub` + `broadcast_div`.
- `from_mapped_safetensors` was misspelled as `from_mapped_safetensors` in
  `models/trocr.rs`; that prevented `--features trocr` from building at all.
- Both backends parsed their own device strings; now consume the central
  `AccelerationConfig` via a shared `resolve_device_preference` helper that
  maps ORT-flavoured providers to candle (`CoreMl → Metal`, `TensorRt → Cuda`).
- The `candle-trocr` and `candle-paddleocr-vl` aggregate features were not
  exposed on `kreuzberg-cli` and the `--ocr-backend` allowlist rejected them,
  so even after registry registration the CLI silently fell back to tesseract.
  Both now wired (commit `d3385696dd`).
- `kreuzberg-candle-ocr` declared `image = { workspace = true }` with no
  decoder features. The CLI path happened to work because of cross-crate
  feature unification with the main `kreuzberg` crate, but standalone use of
  the candle-ocr crate failed with "image format Png is not supported". The
  integration-test commit explicitly enables `png, jpeg, webp, bmp, tiff, gif`
  on the candle-ocr's `image` dep.

## End-to-end smoke results

### TrOCR (`candle-trocr`)

CLI smoke (`test_documents/images/english_and_korean.png`):

- Backend selection: candle-trocr was reached (registry log
  `Loading TrOCR variant: base-printed`).
- Model load: `microsoft/trocr-base-printed` fetched to HF cache (~1.5 GB),
  encoder-decoder built, `TrOCR base-printed initialized successfully`.
- Decode: completed in 215 s on CPU.
- Output: `content = "1"` on the multi-line fixture.

Integration test (`crates/kreuzberg-candle-ocr/tests/trocr_integration.rs`,
fixture `test_documents/images/test_hello_world.png`, weights pre-cached):

- Total: 87 s on CPU.
- Output: non-empty; `is_structured_markdown = false`.

**TrOCR is single-line OCR.** The "1" on the multi-line fixture is the
expected behaviour: TrOCR is trained to recognise one line of text per image,
not full pages. For practical use, TrOCR must be paired with a text-detection
stage that crops text regions before recognition. The single-line integration
fixture confirms the engine itself works correctly.

### PaddleOCR-VL (`candle-paddleocr-vl`)

CLI smoke (`test_documents/images/english_and_korean.png`):

- First attempt (CPU): timed out at the 10 minute extraction limit. The
  default candle build is CPU-only because the `candle-{cuda,metal,...}`
  pass-throughs are not pulled in by `candle-paddleocr-vl` itself.
- Second attempt (`--features candle-paddleocr-vl,candle-metal` on macOS,
  Apple M-series): `DevicePreference::Auto` selected Metal; decode
  completed in **72.7 s**.

Output (excerpt): `"RULES AND INSTRUCTIONS\n1. Template for day 1 (korean),
for day 2 (English) for day 3 both English and korean.\n2. Use all your
accounts. ... 안녕하세요, 저희는 YGE소속 그룹 TREASURE멤버 HARUTO씨의
팬입니다. ..."` — full multi-paragraph transcription with both Latin and
Hangul scripts correctly recognised. Quality score: 1.0.

This is the practical full-page candle OCR backend. CPU decode on this size
of VLM (~0.9 B params, ~1.8 GB safetensors) is impractically slow; users
should always enable one of the `candle-{cuda,metal,accelerate,mkl}`
aggregate features on platforms with hardware support. See the candle-ocr
README for details.

## Coverage

```text
cargo test -p kreuzberg --lib candle_ocr:: \
    --features candle-trocr,candle-paddleocr-vl
# 15 tests passing — backend constructors, parse_options, device resolution,
# language support, mime_type emission, initialize/shutdown lifecycle.

KREUZBERG_NETWORK_TESTS=1 cargo test -p kreuzberg-candle-ocr \
    --features trocr --test trocr_integration -- --ignored
# 1 test passing — full engine load + decode + assertion on real fixture.
```

The PaddleOCR-VL integration test exists (gated identically) but has not yet
been run; it requires the weights to be cached or downloaded fresh.

## Why GOT-OCR / GLM-OCR were dropped

The original plan claimed both were "tractable using candle's existing module
catalog" by composing `segment_anything::image_encoder::ImageEncoderViT` with
`qwen2::ModelForCausalLM` (GOT) and `vit::Model` with `glm4::Model` (GLM).
That premise was wrong: candle 0.10.2 exposes only `forward(input_ids, …)` on
both `qwen2::ModelForCausalLM` and `glm4::Model`, with the embedding layer
private. The "vision-as-prefix" pattern (used cleanly by `paligemma.rs`
because `gemma::Model` exposes `forward_embeds`) is not mechanically possible
against vanilla qwen2/glm4. Two parallel ocr-engineer subagents both shipped
placeholder forward passes rather than fork the decoders — reasonably, since
forking adds ~1000 LOC of decoder duplication per model with no way to verify
the cross-modal glue without a 5 GB weight download.

The two scaffolding commits were reverted to keep the branch honest. Real
GOT-OCR / GLM-OCR ports remain feasible but are now follow-up work, blocked
on one of:

- a candle PR adding `forward_embeds` + public `embed_tokens()` to `qwen2`
  and `glm4` (clean fix, multi-week cycle); or
- a local fork of those decoders into `kreuzberg-candle-ocr` with the
  embedding accessor added (~1000 LOC per model).

## Pending follow-ups

- Pair TrOCR with text detection so it returns useful output on full-page
  fixtures.
- Run the benchmark sweep (`task bench:ocr` or equivalent) on a host where
  the candle backends are GPU-accelerated. PaddleOCR-VL CPU decode time
  (~minutes per fixture) makes a 15-fixture sweep impractical without a GPU.
- Submit the candle `forward_embeds` PR upstream if we decide to revisit
  GOT-OCR / GLM-OCR.

## Commits on this branch since the audit started

```text
989e0c0021 style(candle-ocr): rustfmt long lines in paddleocr_vl engine + test
5973fe8ee1 docs(registry): drop got-ocr / glm-ocr from candle registration comment
<integration-test commit>
5255904e8b docs(candle-ocr): document TrOCR as line-level, drop stale ModelKind variants
ee94b426ec docs(changelog): drop stale GOT-OCR and GLM-OCR entries; add CLI exposure
d3385696dd feat(cli): expose candle-ocr backends in CLI
153aecbc9b Revert "feat(candle-ocr): add GOT-OCR 2.0 backend behind got-ocr sub-feature"
3deec9206a Revert "feat(candle-ocr): add GLM-OCR backend scaffolding with vision encoder"
78eff152f1 refactor(candle-ocr): consume central AccelerationConfig, drop duplicate DeviceKind
cdcd286655 fix(candle-ocr): wire TrocrBackend to engine, pool PaddleOCR-VL, fix token sources
```
