# kreuzberg-candle-ocr

Candle-based VLM OCR engines for Kreuzberg. Pure-Rust transformer OCR via [candle](https://github.com/huggingface/candle).

Supported models (per-model sub-features):

- **trocr** — Microsoft TrOCR (printed/handwritten, ~330M, MIT). Line-level
  only: TrOCR expects a single text line per image and produces poor output on
  full-page documents. Pair with a text-detection stage that crops text
  regions before recognition.
- **paddleocr-vl** — PaddleOCR-VL (0.9B, Apache-2.0, multi-task:
  OCR/tables/formulas/charts). Full-page vision-language model, emits markdown
  directly.

Device pass-through features mirror candle's own: `cuda`, `metal`, `mkl`, `accelerate`.

When depending on `kreuzberg`, the equivalent aggregate features are
`candle-cuda`, `candle-metal`, `candle-mkl`, `candle-accelerate`. Enable
alongside any candle-* backend — without one of them the candle build
remains CPU-only, and PaddleOCR-VL CPU decode is impractically slow.
