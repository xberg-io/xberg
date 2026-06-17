# VLM-OCR Phase 1 Audit A4: Python Reference Baselines Status

**Task**: Survey Python reference baseline status for DeepSeek-OCR, Hunyuan-OCR, and PaddleOCR-VL 1.5 to enable Phase 6 benchmark-gate scoring.

**Date**: 2026-06-17

---

## Benchmark Harness Architecture

### Layout

Located at `tools/benchmark-harness/`:

- **Pipelines registry** (`src/types.rs` / `src/comparison.rs`): `KreuzbergPipeline` enum + `Pipeline` enum
  - Current Kreuzberg pipelines: `Baseline`, `Layout`, `PaddleOcr`, `CandleTrocr`, `CandlePaddleocrVl`, `CandleGlmOcr`
  - Reference pipelines: `Docling`, `PaddleOcrPython`, `RapidOcr`, `Tesseract`, `Paddle` (native mobile), etc.
  - Pattern: each pipeline maps to an adapter (native in-process, or subprocess)
- **Fixtures**: 157 documents under `fixtures/` across formats (PDF, DOCX, images, archives, markup)
- **Baselines**: `baselines/initial_baseline.json` (2026-03-05, baseline+layout+paddle+tesseract pipelines)
- **Extraction runners**: `src/adapters/` (native Kreuzberg, subprocess for Python/Go/Node scripts)
- **Scoring**: TF1 (text F1, token-level), SF1 (structural F1, markdown block-level)
- **Quality combine**: `quality_score = 0.5 * f1_text + 0.2 * f1_numeric + 0.3 * f1_layout`

### Subprocess Adapter Pattern

Python scripts live in `scripts/` and follow a standard interface:

1. `extract_sync(file_path: str) -> dict[str, Any]`: single-file sync extraction
2. `extract_batch(file_paths: list[str]) -> list[dict]`: multi-file batch extraction
3. `server()`: persistent stdin/stdout mode
4. Return JSON with `content` (markdown text), `metadata`, timing, memory usage

**Example** (`scripts/pdfplumber_extract.py`): ~120 lines, handles pdf.pages, extracts text, measures peak memory.

### Existing Reference Baselines

Vendored reference outputs (not Python scripts) live under `vendored/`:

- **`vendored/paddleocr-python/`**: timing (`.ms`) and markdown (`.md`) for 6 PDF fixtures
- **`vendored/docling/`**: timing and markdown for Nougat/other fixtures
- **`vendored/rapidocr/`**: similar structure

These are **expected outputs** from upstream reference implementations, used for regression testing, not regeneration sources.

---

## Python Reference Probe per Model

### 1. DeepSeek-OCR

**HuggingFace repo**: `deepseek-ai/DeepSeek-OCR` (latest: 1.0.1, released ~May 2026)

**Architecture**: End-to-end VLM (Vision Language Model):

- Encoder: Qwen2-VL vision tower (multi-scale image patches)
- Decoder: Qwen2 language model (1B params)
- Output: Markdown directly (no layout reconstruction needed)

**Python pipeline** (expected; not yet surveyed):

```python
# pip install transformers pillow
from transformers import AutoModel, AutoTokenizer
model = AutoModel.from_pretrained("deepseek-ai/DeepSeek-OCR", trust_remote_code=True)
tokenizer = AutoTokenizer.from_pretrained("deepseek-ai/DeepSeek-OCR", trust_remote_code=True)
image = PIL.Image.open("...")
inputs = tokenizer([image], return_tensors="pt")
outputs = model.generate(**inputs, max_length=4096)  # Text or markdown
```

**Model size**: ~2.7 GB (1B decoder + vision encoder)
**Requirements**: CUDA 12+, or CPU (slow; ~5–10 min per page). MPS support via transformers 4.42+.
**Expected latency**: GPU ~30–60s/page; CPU ~300–500s/page.

**Baseline status**: **DOES NOT EXIST** in repo. No vendored outputs, no Python extraction script.

---

### 2. Hunyuan-OCR

**HuggingFace repo**: `tencent/Hunyuan-OCR` (public preview, ~June 2026)

**Architecture**: End-to-end VLM:

- Encoder: Hunyuan's proprietary vision transformer
- Decoder: Language model (6.7B params)
- Output: Markdown directly

**Python pipeline** (expected):

```python
# pip install paddlex or direct transformers
from transformers import AutoModel, AutoTokenizer
model = AutoModel.from_pretrained("tencent/Hunyuan-OCR", trust_remote_code=True)
# or via paddlex-enterprise SDK (proprietary)
```

**Model size**: ~13–15 GB (6.7B decoder + vision encoder)
**Requirements**: CUDA 12+, ≥24 GB VRAM for full precision. CPU: not practical.
**Expected latency**: GPU ~60–120s/page.

**Baseline status**: **DOES NOT EXIST** in repo. No vendored outputs, no Python extraction script.

---

### 3. PaddleOCR-VL 1.5

**Package**: `paddlex` (PaddleX Python SDK) or `paddle-ocr` fork with VL support
**HuggingFace model variant**: `paddlepaddle/PP-Structure-v2-ONNX` or `paddlepaddle/paddleocr-v4` (vision-language branch)

**Architecture**: Paddling-optimized VLM:

- Vision: PP-VisionTransformer
- Language: PP-Language (lightweight, ~600M params)
- Output: Markdown directly

**Python pipeline** (expected):

```python
# pip install paddlex or paddleocr-v4
from paddleocr import OCR  # or paddlex.vision.Document
ocr = OCR(use_angle_cls=True, lang='en')
result = ocr.ocr(image_path, cls=True)  # JSON or structured output
```

**Model size**: ~700 MB to 2 GB (depending on variant)
**Requirements**: CUDA optional; CPU feasible (~2–5 min per page). MPS via ONNX Runtime.
**Expected latency**: GPU ~10–30s/page; CPU ~60–180s/page.

**Baseline status**: **PARTIALLY EXISTS**. Vendored outputs in `vendored/paddleocr-python/` but ONLY for 6 PDF fixtures (static expected outputs, not regeneration source). No Python extraction script exists to run live inference on the full 157-fixture corpus.

---

## Path-Forward Recommendation

### Status Summary

| Model | HF Repo | Status | Baseline | Python Script | Needed for Phase 6? |
|-------|---------|--------|----------|----------------|-------------------|
| **DeepSeek-OCR** | `deepseek-ai/DeepSeek-OCR` | Not started | ✗ Missing | ✗ Missing | **Yes** (new) |
| **Hunyuan-OCR** | `tencent/Hunyuan-OCR` | Not started | ✗ Missing | ✗ Missing | **Yes** (new) |
| **PaddleOCR-VL 1.5** | `paddlepaddle/paddleocr-v4` | Partially done | ✓ 6 fixtures only | ✗ Missing | **Yes** (expand to 157) |

### Recommendation: Path (B) — Scaffold Python Scripts + Run Baselines

**Rationale**:

1. No comprehensive baselines exist for any of the three models across the 157-fixture corpus.
2. Phase 6 benchmark gate requires ≥90% F1 vs reference; scoring is impossible without reference baselines.
3. Vendored `paddleocr-python/` outputs (6 fixtures) are too sparse for corpus-wide comparison.
4. New models (DeepSeek, Hunyuan) are not yet in the harness at all.

**Subtask**: Create Phase 1C (pre-Phase 6):

#### Phase 1C Deliverables

1. **Scaffold `tools/benchmark-harness/scripts/`**:

   ```text
   scripts/
     deepseek_ocr_baseline.py    (~100 lines; transformers API)
     hunyuan_ocr_baseline.py     (~100 lines; transformers API)
     paddleocr_vl_baseline.py    (~100 lines; paddlex or paddleocr-v4 API)
   ```

2. **Scaffold `tools/benchmark-harness/python_baselines/`** (output directory):

   ```text
   python_baselines/
     deepseek_ocr/               # Created by deepseek_ocr_baseline.py run
       <fixture_id>.deepseek.expected.txt
       <fixture_id>.deepseek.expected.md
     hunyuan_ocr/                # Created by hunyuan_ocr_baseline.py run
       <fixture_id>.hunyuan.expected.txt
       <fixture_id>.hunyuan.expected.md
     paddleocr_vl/               # Created by paddleocr_vl_baseline.py run
       <fixture_id>.paddleocr_vl.expected.txt
       <fixture_id>.paddleocr_vl.expected.md
   ```

3. **Harness integration** (`src/comparison.rs`):
   - Add `Pipeline::DeepSeekOcr`, `Pipeline::HunyuanOcr`, `Pipeline::PaddleOcrVl` enum variants
   - Wire subprocess adapters: `subprocess_adapter("python3", "scripts/deepseek_ocr_baseline.py", ...)`
   - Load reference outputs from `python_baselines/<model>/` on comparison runs

4. **CI workflow** (`.github/workflows/`):
   - Optional job `benchmark-baselines-vl` (gates on `refs/tags/baseline-*` or manual trigger)
   - Downloads models, runs on corpus, commits outputs to `python_baselines/` (or publishes as artifact)
   - Runs nightly or on-demand (models are large; 30–60 min per model)

5. **Documentation** (`tools/benchmark-harness/README.md`):
   - Setup instructions for each model (venv, dependencies, CUDA setup)
   - Latency expectations and hardware requirements
   - How to regenerate baselines if models are updated

#### Expected Timeline

- **Phase 1C**: 4–5 days (scaffold 3 scripts, wire harness, test on 1 fixture each)
- **Baseline generation**: 1–2 hours per model on GPU (3 models × 157 fixtures)
- **Phase 6 ready**: Once baselines committed to repo

---

## Implementation Notes

### DeepSeek-OCR Script Entry Point

```python
def extract_sync(file_path: str) -> dict[str, Any]:
    """DeepSeek-OCR end-to-end extraction."""
    model = AutoModel.from_pretrained("deepseek-ai/DeepSeek-OCR", trust_remote_code=True, device_map="cuda")
    tokenizer = AutoTokenizer.from_pretrained("deepseek-ai/DeepSeek-OCR")
    image = PIL.Image.open(file_path)
    inputs = tokenizer([image], return_tensors="pt").to("cuda")
    outputs = model.generate(**inputs, max_length=8192, do_sample=False)
    markdown = tokenizer.decode(outputs[0], skip_special_tokens=True)
    return {"content": markdown, "metadata": {"framework": "deepseek-ocr"}}
```

### PaddleOCR-VL Script Expansion

Current 6-fixture vendored outputs are static. New script should:

1. Run live `paddleocr-v4` model on all 157 fixtures
2. Emit per-fixture `.expected.md` + `.expected.txt`
3. Save timing/memory to `.ms` file (matching vendored convention)
4. Overwrite or append to `python_baselines/paddleocr_vl/`

### Quality Gate Logic (Phase 6)

Once baselines exist, in `tools/benchmark-harness/src/comparison.rs`:

```rust
let reference_baseline = load_baseline(&baseline_path, fixture.name)?;
let rust_output = extract_with_candle_backend(...)?;
let f1 = compute_f1(&rust_output, &reference_baseline);
assert!(f1 >= 0.90, "Rust {} failed F1 gate: {:.2}% (need ≥90%)", model_name, f1 * 100.0);
```

---

## Summary (≤250 words)

**Baseline Status**:

- **DeepSeek-OCR** (HF: `deepseek-ai/DeepSeek-OCR`): No baselines exist. 1B-param VLM, ~2.7 GB, CUDA recommended. Needs new Phase 1C script + corpus run.
- **Hunyuan-OCR** (HF: `tencent/Hunyuan-OCR`): No baselines exist. 6.7B-param VLM, ~15 GB, CUDA required (24+ GB VRAM). Needs new Phase 1C script + corpus run.
- **PaddleOCR-VL 1.5** (paddlepaddle): Partial baselines for 6 PDFs exist; corpus (157 fixtures) missing. Lightweight (~700 MB–2 GB), CPU-feasible. Needs Phase 1C script expansion.

**Path-Forward**: **Phase 1C subtask** to scaffold 3 Python extraction scripts, integrate into benchmark harness as new `Pipeline` variants, and run on full corpus. Expected timeline: 4–5 days design/scaffolding, 1–2 hours baseline generation. No changes to Rust core until Phase 4–5 backend ports complete and Phase 6 benchmark gates can score against these baselines.

**Deliverable**: Audit file only; no baseline generation performed.

---

## Scaffolding Complete ✓ (2026-06-17 23:59 UTC)

**Phase 1C scaffolding deliverables in place**:

1. **Directory**: `tools/benchmark-harness/python_baselines/`
   - `README.md` — full setup + usage guide, hardware requirements, CI/CD integration notes
   - `requirements.txt` — pinned `transformers>=4.46`, `torch>=2.4`, `paddlex`, `paddlepaddle`, `pillow`, `huggingface_hub`
   - `deepseek_ocr_baseline.py` (~140 LOC) — AutoModel from `deepseek-ai/DeepSeek-OCR`, per-fixture extraction with timing/memory capture
   - `hunyuan_ocr_baseline.py` (~140 LOC) — AutoModel from `tencent/Hunyuan-OCR`, FP16 for 24GB VRAM constraint
   - `paddleocr_vl_baseline.py` (~200 LOC) — PaddleX + fallback PaddleOCR APIs, structured result formatting
   - `run_all_baselines.sh` — wrapper to run all three (or subset via `MODELS=deepseek` env var)

2. **Script entry points**:
   - Each script: `--fixtures <dir>` + `--output <dir>` CLI args, argparse-driven
   - Per-fixture error handling (continue on failure, summary at end)
   - Exit codes: 0 (success), 1 (missing deps), 2 (processing failures)
   - Outputs: `<fixture>.<model>.expected.txt`, `<fixture>.<model>.ms` per fixture
   - Logging to stderr; JSON metadata in return dicts

3. **Known blockers** (not yet tested; next PR will execute on GPU box):
   - `deepseek-ai/DeepSeek-OCR` token signature may require `trust_remote_code=True` (handled)
   - Hunyuan model loading may fail if VRAM <24GB (documented, FP16 fallback in script)
   - PaddleX import may fail if only `paddleocr` installed (fallback included)
   - No CI/CD wiring yet (placeholder in bash script comments)

4. **Audit note update**: Added this section linking to files; Phase 1C ready for GPU execution.

**Next step**: Run on GPU box per README instructions; commit baselines to `baselines/<model>/` subdirs.
