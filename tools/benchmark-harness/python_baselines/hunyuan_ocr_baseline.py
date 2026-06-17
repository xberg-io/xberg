#!/usr/bin/env python3
"""Hunyuan-OCR reference baseline extraction.

Model: tencent/Hunyuan-OCR (6.7B-param end-to-end VLM)
- Encoder: Hunyuan proprietary vision transformer
- Decoder: Language model (6.7B params)
- Output: Direct markdown OCR from image

Model size: ~13–15 GB (disk after download)
Hardware: CUDA 12+ required, ≥24 GB VRAM for full precision
Expected latency: GPU ~60–120s/page

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

# Logging setup
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
    """Extract text from single image using Hunyuan-OCR.

    Args:
        file_path: Path to image file
        model: Loaded Hunyuan model
        tokenizer: Loaded tokenizer
        device: Device to run on ("cuda" only; CPU not practical)

    Returns:
        dict with 'content' (text), 'metadata', timing, and memory info
    """
    start = time.perf_counter()

    try:
        # Load and preprocess image
        image = PIL.Image.open(file_path).convert("RGB")

        # Tokenize (Hunyuan API: expects image in processor or tokenizer)
        # Note: Hunyuan may use a custom processor; check model card
        inputs = tokenizer([image], return_tensors="pt")
        inputs = {k: v.to(device) for k, v in inputs.items()}

        # Generate markdown
        with __import__("torch").no_grad():
            outputs = model.generate(
                **inputs,
                max_length=8192,
                do_sample=False,
                temperature=1.0,
            )

        # Decode to text
        content = tokenizer.decode(outputs[0], skip_special_tokens=True)
        duration_ms = (time.perf_counter() - start) * 1000.0

        return {
            "content": content,
            "metadata": {
                "framework": "hunyuan-ocr",
                "model": "tencent/Hunyuan-OCR",
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
                "framework": "hunyuan-ocr",
                "error": str(e),
            },
            "_extraction_time_ms": duration_ms,
            "_peak_memory_bytes": _get_peak_memory_bytes(),
        }


def main():
    """CLI entry point."""
    parser = argparse.ArgumentParser(description="Generate Hunyuan-OCR reference baselines for image fixtures.")
    parser.add_argument(
        "--fixtures",
        type=Path,
        default=Path("../../fixtures"),
        help="Directory containing fixture images",
    )
    parser.add_argument(
        "--output",
        type=Path,
        default=Path("baselines/hunyuan_ocr"),
        help="Directory to write baseline outputs",
    )
    parser.add_argument(
        "--hf-token",
        default=os.environ.get("HF_TOKEN"),
        help="HuggingFace token (for gated model access)",
    )

    args = parser.parse_args()

    # Create output directory
    args.output.mkdir(parents=True, exist_ok=True)

    log.info("=" * 70)
    log.info("Hunyuan-OCR Baseline Generation")
    log.info("=" * 70)
    log.info("Model: tencent/Hunyuan-OCR")
    log.info("Size: ~13–15 GB (disk)")
    log.info("WARNING: Requires CUDA 12+, ≥24 GB VRAM")
    log.info(f"Fixtures directory: {args.fixtures}")
    log.info(f"Output directory: {args.output}")
    log.info("=" * 70)

    # Check fixture directory
    if not args.fixtures.exists():
        log.error(f"Fixtures directory not found: {args.fixtures}")
        sys.exit(1)

    # Load model and tokenizer
    log.info("Loading Hunyuan-OCR model... (first run will download ~13–15 GB)")
    try:
        model = AutoModel.from_pretrained(
            "tencent/Hunyuan-OCR",
            trust_remote_code=True,
            device_map="cuda",
            torch_dtype=__import__("torch").float16,  # Use FP16 to fit in 24 GB VRAM
            token=args.hf_token,
        )
        tokenizer = AutoTokenizer.from_pretrained(
            "tencent/Hunyuan-OCR",
            trust_remote_code=True,
            token=args.hf_token,
        )
        log.info("Model loaded successfully")
    except Exception as e:
        log.error(f"Failed to load model: {e}")
        log.error("Ensure CUDA 12+ and ≥24 GB VRAM available")
        sys.exit(1)

    # Find image files
    image_files = sorted(f for f in args.fixtures.rglob("*") if _is_image_file(f))
    log.info(f"Found {len(image_files)} image files")

    if not image_files:
        log.warning("No image files found in fixtures directory")
        sys.exit(0)

    # Process fixtures
    successful = 0
    failed = 0

    for i, img_path in enumerate(image_files, 1):
        log.info(f"[{i}/{len(image_files)}] Processing {img_path.name}...")

        result = extract_sync(str(img_path), model, tokenizer, device="cuda")

        # Save output
        fixture_stem = img_path.stem
        output_file = args.output / f"{fixture_stem}.hunyuan-ocr.expected.txt"

        if "error" in result["metadata"]:
            failed += 1
            log.warning(f"  Failed: {result['metadata']['error']}")
        else:
            successful += 1
            output_file.write_text(result["content"], encoding="utf-8")
            timing_file = args.output / f"{fixture_stem}.hunyuan-ocr.ms"
            timing_file.write_text(str(int(result["_extraction_time_ms"])), encoding="utf-8")
            log.info(f"  Saved: {output_file.name} ({result['_extraction_time_ms']:.0f} ms)")

    # Summary
    log.info("=" * 70)
    log.info("Hunyuan-OCR baseline generation complete")
    log.info(f"  Processed: {len(image_files)}")
    log.info(f"  Successful: {successful}")
    log.info(f"  Failed: {failed}")
    log.info(f"  Output: {args.output}")
    log.info("=" * 70)

    sys.exit(0 if failed == 0 else 2)


if __name__ == "__main__":
    main()
