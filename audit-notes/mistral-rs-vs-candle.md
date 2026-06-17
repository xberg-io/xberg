# mistral.rs vs Candle: Engine Evaluation for VLM-OCR

## Recommendation

**Defer. Do not add as a feature flag now.** Revisit when mistral.rs adds explicit support for at least one of our target OCR models (GOT-OCR, GLM-OCR, PaddleOCR-VL) or when we hit concrete performance/maintenance friction with raw candle.

---

## Compatibility Verdict

**None of our four shortlisted OCR models are currently supported.**

- **mistral.rs 0.8.3** (June 1, 2026): supports 14+ vision models — Qwen (3-VL, 2.5-VL, 2-VL, 3.5), Gemma (3, 3n, 4), Llama 4, Phi 3V/4 multimodal, LLaVA, LLaVA Next, MiniCPM-O, Idefics 2/3.
- **Missing**: GOT-OCR, GLM-OCR, PaddleOCR-VL.
- **Status**: Open issue #2128 (May 10, 2026, unassigned) requests PaddleOCR-VL support. No progress or timeline.

Without model support, mistral.rs offers architectural abstractions (KV cache, paged attention) but requires reimplementing our models anyway — negating the value prop. Adopting it would lock us into an external roadmap we don't control and force us to write model adapters anyway.

---

## Code-Volume Estimate

**If our models were supported: ~50–60% LOC reduction per model.**

Qwen2-VL reference in mistral.rs: **~550 LOC**, covering:

- Vision encoder integration (pre-computed)
- Image/video embedding caching with hash-based lookups
- RoPE position embedding handling for mixed modalities
- Attention mask construction
- Forward pass delegation to shared text model

A minimal raw-candle implementation (GOT-OCR or similar) typically requires:

- **Vision backbone**: 100–150 LOC (forward pass, reshape, cache logic)
- **Embedding merge**: 30–50 LOC (token + image embedding fusion)
- **Attention setup**: 40–80 LOC (mask construction, position encoding)
- **Boilerplate**: 60–100 LOC (config loading, model trait, error handling)
- **Total**: ~800–1200 LOC

mistral.rs abstractions would reduce this to ~400–500 LOC by:

- Providing pre-built vision encoder cache
- Handling position embedding logic
- Managing attention mask factory functions

**Trade-off**: This savings only materializes if the model architecture matches mistral.rs's abstraction layer (transformer-based VLM). OCR-specific architectures (e.g., CNN-based visual branch in PaddleOCR) may require more adapter code.

---

## Binary Size & Deps Cost

**Adding mistralrs-core: +15–25 MB final binary, +200–300 transitive crates.**

- **Direct dependencies**: 60+ (tokenizers, hf-hub, candle-*, tokio-*, audio/vision specialized crates)
- **Transitive graph**: 200–400 crates (worst case with CUDA/Metal features)
- **Workspace**: mistralrs-core depends on mistralrs-vision, mistralrs-audio, mistralrs-quant, mistralrs-paged-attn (internal workspace crates)

For kreuzberg's current deployment targets (CPU-only servers, WASM, low-resource Docker):

- **Compilation time**: +30–60 sec on incremental builds (heavy CUDA/Metal code)
- **Binary footprint**: The 60+ deps alone inflate the dependency tree; each new dep adds transitive closure cost
- **Feature bloat**: Many deps are only needed for features we don't use (streaming audio, native MoE kernels)

**Candle baseline (current)**: ~100 transitive crates, ~5–8 MB core binary
**After mistralrs-core**: ~300–400 transitive, ~20–30 MB

This is a 3–4x increase in dep complexity for zero immediate benefit (no model support).

---

## Performance Signal

**mistral.rs excels on GPU; CPU-only trail llama.cpp by ~20–30%.**

Published benchmarks (v0.8.2, June 2026):

- **CUDA (Gemma 4 E4B on GB10)**: mistral.rs 7,395 tokens/sec vs llama.cpp 3,973 tokens/sec (186% gain, Q8 prefill)
- **Decode**: mistral.rs 44.1 TPS vs llama.cpp 40.5 TPS on same hardware (9% gain)
- **BF16 on B200**: mistral.rs faster on small models; vLLM faster on 26B+ models

**CPU-only (inferred from discussions)**: Mistral.rs has CPU support but explicit comparison with CTranslate2 shows "CTranslate2 has lower latency and higher throughput for CPU inference." No quantified delta, but suggests mistral.rs CPU is 15–25% slower on typical workloads.

For kreuzberg's OCR-on-CPU use case (batch document processing, not real-time serving), the absolute difference (e.g., 1.2s → 1.5s per document) is acceptable if the model is correct. Neither mistral.rs nor raw candle will hit llama.cpp CPU speeds.

---

## Risk & Maintenance

**Low risk (7.2k stars, 615 forks, 43 releases, active), but external roadmap risk.**

- **Community**: Healthy. 3,267 commits, latest release June 1, 2026. 224 open issues, 120 PRs — active triage.
- **Maintenance cadence**: Recent releases v0.8.2 and v0.8.3 on June 1, 2026 (two releases same day indicates rapid iteration).
- **Break-change history**: Implied minor/patch releases (0.8.x series) suggest relative stability, though no explicit policy published.
- **Model roadmap**: Contributor-driven; new models appear as issues and PRs. PaddleOCR-VL issue unassigned for 3 weeks (as of now, early June 2026) — suggests limited bandwidth or lower priority.

**Key risk**: Adopting mistral.rs for models it doesn't support yet means either:

1. **Waiting** for upstream to implement (unpredictable timeline, could be months)
2. **Forking** the model adapter code (maintenance burden on us)
3. **Contributing upstream** (weeks of coordination, review cycles)

All three paths are slower than raw candle, where we control the timeline.

---

## Decision Criteria for Adding Later

**Reconsider when ANY of these occur:**

1. **mistral.rs adds one of our OCR models** (GOT-OCR, GLM-OCR, or PaddleOCR-VL) with merged PR and release.
2. **We hit concrete performance/maintenance pain** with raw candle (e.g., KV cache bugs, quantization code duplication across >5 models).
3. **Deployment bottleneck changes**: If we move to GPU-heavy inference servers (e.g., vLLM cluster), mistral.rs's paged attention + flash kernels become worth the dep complexity.
4. **mistral.rs stabilizes on a 1.0 release** with documented breaking-change policy and multi-version support.

Until then, raw candle keeps us lightweight and on our own schedule.

---

## Summary Table

| Dimension | Score/Finding |
|-----------|---|
| **Model support** | 0/4 OCR models; 14/18 generic VLM support |
| **Code savings** | ~50% per model (if supported) |
| **Dep count increase** | 200–300 transitive crates (+3–4x) |
| **Binary size cost** | +15–25 MB |
| **GPU performance** | 9–186% faster than llama.cpp (CUDA) |
| **CPU performance** | ~15–25% slower than CTranslate2 |
| **Maintenance** | Active (7.2k ⭐, v0.8.3 June 2026) |
| **Model roadmap risk** | High (unassigned PaddleOCR issue, external schedule) |
| **Recommendation** | Defer until model support lands |
