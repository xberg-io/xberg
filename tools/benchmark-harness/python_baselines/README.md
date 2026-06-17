# VLM-OCR Python Reference Baselines

This directory scaffolds Python reference baseline pipelines for three VLM-OCR models used in Phase 6 benchmark-gate scoring.

## Purpose

Phase 6 gates require comparing Rust candle backend output to Python reference implementations. The 90% character-level F1 threshold measures extraction quality against established baselines. This directory provides:

1. **Generation scripts** — Run live inference on all 157 fixture documents to produce `<fixture>.<model>.expected.txt` reference outputs.
2. **Baseline storage** — Reference outputs saved under `baselines/<model>/` for consumption by the Rust benchmark harness.
3. **Reproducibility** — Pinned dependencies (requirements.txt) for consistent baseline generation across runs.

## Models Covered

| Model | HF ID | Size | Requirements | Status |
|-------|-------|------|--------------|--------|
| **DeepSeek-OCR** | `deepseek-ai/DeepSeek-OCR` | ~2.7 GB | CUDA 12+, 8 GB VRAM | Scaffolded, not yet run |
| **Hunyuan-OCR** | `tencent/Hunyuan-OCR` | ~13–15 GB | CUDA 12+, 24 GB VRAM | Scaffolded, not yet run |
| **PaddleOCR-VL 1.5** | `paddlepaddle/paddleocr-v4` or `paddlex` | ~700 MB–2 GB | CPU OK, CUDA optional | Scaffolded, not yet run |

## Running the Baseline Scripts

### Setup

```bash
cd tools/benchmark-harness/python_baselines
python -m venv venv
source venv/bin/activate  # or venv\Scripts\activate on Windows
pip install -r requirements.txt
```

### Generate All Baselines

```bash
# Runs all three models against fixtures/ directory
bash run_all_baselines.sh

# Or selectively (DeepSeek only):
MODELS="deepseek" bash run_all_baselines.sh

# Custom fixture directory:
python deepseek_ocr_baseline.py \
  --fixtures /path/to/fixtures \
  --output baselines/deepseek_ocr
```

### Individual Model Runs

Each script accepts `--fixtures` and `--output` arguments:

```bash
# DeepSeek: ~2.7 GB download, 30–60s/page on V100 GPU
python deepseek_ocr_baseline.py \
  --fixtures ../../fixtures \
  --output baselines/deepseek_ocr

# Hunyuan: ~15 GB download, 60–120s/page on V100, requires 24+ GB VRAM
python hunyuan_ocr_baseline.py \
  --fixtures ../../fixtures \
  --output baselines/hunyuan_ocr

# PaddleOCR-VL: Lightweight, CPU feasible (2–5 min/page), CUDA optional
python paddleocr_vl_baseline.py \
  --fixtures ../../fixtures \
  --output baselines/paddleocr_vl
```

## Output Format

Each script writes:

- **`<fixture-stem>.<model-name>.expected.txt`** — Plain text extracted content (used for F1 scoring)
- **`<fixture-stem>.<model-name>.expected.md`** — Markdown output (if model supports it)
- **`<fixture-stem>.<model-name>.ms`** — Timing file (milliseconds)

Example:

```text
baselines/deepseek_ocr/
  document_001.deepseek-ocr.expected.txt
  document_001.deepseek-ocr.expected.md
  document_001.deepseek-ocr.ms
  document_002.deepseek-ocr.expected.txt
  ...
```

## Harness Integration

The Rust benchmark harness (`tools/benchmark-harness/src/`) loads these baselines:

```rust
// Load reference baseline
let reference = load_baseline(&baseline_path, fixture.name)?;

// Compare Rust output vs reference
let f1 = compute_character_f1(&rust_output, &reference);

// Gate: must be ≥90%
assert!(f1 >= 0.90, "Baseline F1 failed: {:.1}% (need ≥90%)", f1 * 100.0);
```

## Troubleshooting

### Model Download Failures

- Requires HuggingFace token for gated models (set `HF_TOKEN` env var).
- Ensure sufficient disk space (30–50 GB recommended for all three).

### CUDA/Device Issues

- **DeepSeek/Hunyuan** require CUDA 12+; fall back to CPU (very slow) by removing `device_map="cuda"`.
- **PaddleOCR-VL** works on CPU; CUDA optional.

### Per-Fixture Errors

- Scripts catch exceptions per fixture and continue (summary at end).
- Check stderr for failed fixtures; common causes: corrupted image, unsupported format, timeout.

### Memory Issues

- Reduce batch size or run one fixture at a time if VRAM exhausted.
- Monitor with `nvidia-smi` (GPU) or `top` (CPU memory).

## CI/CD Integration (Future)

Once baselines are generated and committed:

1. CI workflow `benchmark-baselines-vl` (optional job, triggered on-demand or tags):
   - Runs script, commits outputs to `baselines/` or uploads as artifact.
   - Runs nightly or on manual trigger (models are large; 2–4 hours per full run).

2. Phase 6 tests gate on baseline presence:

   ```rust
   let baseline_path = "tools/benchmark-harness/baselines/deepseek_ocr/document.expected.txt";
   assert!(baseline_path.exists(), "Baseline not found; run python_baselines scripts first");
   ```

## Estimated Timeline

- **Scaffolding** (this PR): Done ✓
- **Baseline generation** (next PR, GPU box):
  - DeepSeek: ~90 min (157 fixtures × 30–60s + overhead)
  - Hunyuan: ~3–4 hours (larger model, longer latency)
  - PaddleOCR-VL: ~30–60 min (lightweight, CPU feasible)
- **Total**: ~5–6 hours on a single V100/A100 GPU

## Notes

- Scripts are scaffolded but **not yet validated** on live models. First run on GPU box may reveal dependency or API issues.
- PaddleOCR-VL 1.5 is the latest; older `paddle-ocr` package may not support vision-language variants.
- Baseline outputs are reproducible but may differ slightly between CUDA versions and GPU architectures due to floating-point precision.
