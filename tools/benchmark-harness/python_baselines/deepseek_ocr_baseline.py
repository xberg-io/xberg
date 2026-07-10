#!/usr/bin/env python3
"""DeepSeek-OCR reference baseline extraction.

Model: deepseek-ai/DeepSeek-OCR (1B-param end-to-end VLM)
- Encoder: Qwen2-VL vision tower (multi-scale patches)
- Decoder: Qwen2 language model (1B params)
- Output: Direct markdown OCR from image

Model size: ~2.7 GB (disk after download)
Hardware: CUDA 12+ recommended; CPU fallback slow (~5-10 min/page)
Expected latency: GPU ~30-60s/page, CPU ~300-500s/page

Exit codes:
- 0: All fixtures processed successfully
- 1: Missing model or dependencies
- 2: Fixture processing errors; check stderr for per-file failures
"""

from __future__ import annotations

import argparse
import logging
import os
import platform
import resource
import sys
import time
from pathlib import Path
from typing import Any

import PIL.Image
from transformers import AutoModel, AutoTokenizer

logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s [%(levelname)s] %(message)s",
)
log = logging.getLogger(__name__)


def _get_peak_memory_bytes() -> int:
    """Get peak memory usage in bytes."""
    usage = resource.getrusage(resource.RUSAGE_SELF)
    if platform.system() == "Linux":
        return usage.ru_maxrss * 1024
    return usage.ru_maxrss


def _is_image_file(path: Path) -> bool:
    """Check if file is a supported image."""
    return path.suffix.lower() in {".png", ".jpg", ".jpeg", ".tif", ".tiff", ".webp"}


def extract_sync(
    file_path: str,
    model: Any,
    tokenizer: Any,
    device: str = "cuda",
) -> dict[str, Any]:
    """Extract text from single image using DeepSeek-OCR.

    Args:
        file_path: Path to image file
        model: Loaded DeepSeek model
        tokenizer: Loaded tokenizer
        device: Device to run on ("cuda" or "cpu")

    Returns:
        dict with 'content' (text), 'metadata', timing, and memory info
    """
    start = time.perf_counter()

    try:
        image = PIL.Image.open(file_path).convert("RGB")

        inputs = tokenizer([image], return_tensors="pt")
        inputs = {k: v.to(device) for k, v in inputs.items()}

        with __import__("torch").no_grad():
            outputs = model.generate(
                **inputs,
                max_length=8192,
                do_sample=False,
                temperature=1.0,
            )

        content = tokenizer.decode(outputs[0], skip_special_tokens=True)
        duration_ms = (time.perf_counter() - start) * 1000.0

        return {
            "content": content,
            "metadata": {
                "framework": "deepseek-ocr",
                "model": "deepseek-ai/DeepSeek-OCR",
                "device": device,
            },
            "_extraction_time_ms": duration_ms,
            "_peak_memory_bytes": _get_peak_memory_bytes(),
        }

    except Exception as e:
        duration_ms = (time.perf_counter() - start) * 1000.0
        log.error(f"Extraction failed for {file_path}: {e}")
        return {
            "content": "",
            "metadata": {
                "framework": "deepseek-ocr",
                "error": str(e),
            },
            "_extraction_time_ms": duration_ms,
            "_peak_memory_bytes": _get_peak_memory_bytes(),
        }


def main() -> None:
    """CLI entry point."""
    parser = argparse.ArgumentParser(description="Generate DeepSeek-OCR reference baselines for image fixtures.")
    parser.add_argument(
        "--fixtures",
        type=Path,
        default=Path("../../fixtures"),
        help="Directory containing fixture images",
    )
    parser.add_argument(
        "--output",
        type=Path,
        default=Path("baselines/deepseek_ocr"),
        help="Directory to write baseline outputs",
    )
    parser.add_argument(
        "--device",
        choices=["cuda", "cpu"],
        default="cuda",
        help="Device to run model on",
    )
    parser.add_argument(
        "--hf-token",
        default=os.environ.get("HF_TOKEN"),
        help="HuggingFace token (for gated model access)",
    )

    args = parser.parse_args()

    args.output.mkdir(parents=True, exist_ok=True)

    log.info("=" * 70)
    log.info("DeepSeek-OCR Baseline Generation")
    log.info("=" * 70)
    log.info("Model: deepseek-ai/DeepSeek-OCR")
    log.info("Size: ~2.7 GB (disk)")
    log.info(f"Device: {args.device}")
    log.info(f"Fixtures directory: {args.fixtures}")
    log.info(f"Output directory: {args.output}")
    log.info("=" * 70)

    if not args.fixtures.exists():
        log.error(f"Fixtures directory not found: {args.fixtures}")
        sys.exit(1)

    log.info("Loading DeepSeek-OCR model... (first run will download ~2.7 GB)")
    try:
        model = AutoModel.from_pretrained(
            "deepseek-ai/DeepSeek-OCR",
            trust_remote_code=True,
            device_map=args.device,
            torch_dtype=__import__("torch").float32,
            token=args.hf_token,
        )
        tokenizer = AutoTokenizer.from_pretrained(
            "deepseek-ai/DeepSeek-OCR",
            trust_remote_code=True,
            token=args.hf_token,
        )
        log.info("Model loaded successfully")
    except Exception as e:
        log.error(f"Failed to load model: {e}")
        sys.exit(1)

    image_files = sorted(f for f in args.fixtures.rglob("*") if _is_image_file(f))
    log.info(f"Found {len(image_files)} image files")

    if not image_files:
        log.warning("No image files found in fixtures directory")
        sys.exit(0)

    successful = 0
    failed = 0

    for i, img_path in enumerate(image_files, 1):
        log.info(f"[{i}/{len(image_files)}] Processing {img_path.name}...")

        result = extract_sync(str(img_path), model, tokenizer, device=args.device)

        fixture_stem = img_path.stem
        output_file = args.output / f"{fixture_stem}.deepseek-ocr.expected.txt"

        if "error" in result["metadata"]:
            failed += 1
            log.warning(f"  Failed: {result['metadata']['error']}")
        else:
            successful += 1
            output_file.write_text(result["content"], encoding="utf-8")
            timing_file = args.output / f"{fixture_stem}.deepseek-ocr.ms"
            timing_file.write_text(str(int(result["_extraction_time_ms"])), encoding="utf-8")
            log.info(f"  Saved: {output_file.name} ({result['_extraction_time_ms']:.0f} ms)")

    log.info("=" * 70)
    log.info("DeepSeek-OCR baseline generation complete")
    log.info(f"  Processed: {len(image_files)}")
    log.info(f"  Successful: {successful}")
    log.info(f"  Failed: {failed}")
    log.info(f"  Output: {args.output}")
    log.info("=" * 70)

    sys.exit(0 if failed == 0 else 2)


if __name__ == "__main__":
    main()
