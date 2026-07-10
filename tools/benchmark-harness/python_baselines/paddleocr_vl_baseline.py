#!/usr/bin/env python3
"""PaddleOCR-VL 1.5 reference baseline extraction.

Model: paddlepaddle/paddleocr-v4 (vision-language variant) or paddlex SDK
- Architecture: PP-VisionTransformer encoder + PP-Language (~600M params)
- Output: Structured JSON or direct markdown

Model size: ~700 MB - 2 GB (disk after download)
Hardware: CPU feasible, CUDA optional for speedup
Expected latency: GPU ~10-30s/page, CPU ~60-180s/page

Exit codes:
- 0: All fixtures processed successfully
- 1: Missing model or dependencies
- 2: Fixture processing errors; check stderr for per-file failures
"""

from __future__ import annotations

import argparse
import logging
import platform
import resource
import sys
import time
from pathlib import Path
from typing import Any

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


def _load_paddleocr_vl() -> Any:
    """Load PaddleOCR-VL model.

    Tries multiple import paths:
    1. paddlex (enterprise SDK, higher-level)
    2. paddleocr with VL support (open-source fork)
    """
    try:
        from paddlex import create_model

        log.info("Using PaddleX SDK")
        model = create_model("ocr_vl")
        return ("paddlex", model)
    except ImportError:
        pass

    try:
        from paddleocr import OCR

        log.info("Using PaddleOCR (open-source)")
        model = OCR(use_angle_cls=True, lang="en", version="v4")
        return ("paddleocr", model)
    except ImportError as exc:
        raise ImportError("Neither paddlex nor paddleocr available. Install: pip install paddlex paddlepaddle") from exc


def extract_sync_paddlex(
    file_path: str,
    model: Any,
) -> dict[str, Any]:
    """Extract using PaddleX SDK."""
    start = time.perf_counter()

    try:
        result = model.predict(img_file=file_path)
        content = _format_paddlex_result(result)
        duration_ms = (time.perf_counter() - start) * 1000.0

        return {
            "content": content,
            "metadata": {
                "framework": "paddleocr-vl",
                "backend": "paddlex",
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
                "framework": "paddleocr-vl",
                "backend": "paddlex",
                "error": str(e),
            },
            "_extraction_time_ms": duration_ms,
            "_peak_memory_bytes": _get_peak_memory_bytes(),
        }


def extract_sync_paddleocr(
    file_path: str,
    model: Any,
) -> dict[str, Any]:
    """Extract using PaddleOCR open-source."""
    start = time.perf_counter()

    try:
        result = model.ocr(file_path, cls=True)
        content = _format_paddleocr_result(result)
        duration_ms = (time.perf_counter() - start) * 1000.0

        return {
            "content": content,
            "metadata": {
                "framework": "paddleocr-vl",
                "backend": "paddleocr",
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
                "framework": "paddleocr-vl",
                "backend": "paddleocr",
                "error": str(e),
            },
            "_extraction_time_ms": duration_ms,
            "_peak_memory_bytes": _get_peak_memory_bytes(),
        }


def _format_paddlex_result(result: Any) -> str:
    """Convert PaddleX result to plaintext."""
    lines = []
    if hasattr(result, "text"):
        return str(result.text)
    if isinstance(result, dict) and "text" in result:
        return str(result["text"])
    if isinstance(result, list):
        for item in result:
            if isinstance(item, str):
                lines.append(item)
            elif isinstance(item, dict) and "text" in item:
                lines.append(item["text"])
    return "\n".join(lines)


def _format_paddleocr_result(result: Any) -> str:
    """Convert PaddleOCR result to plaintext."""
    lines = []
    if result is None:
        return ""

    for line_item in result:
        if isinstance(line_item, list):
            for item in line_item:
                if isinstance(item, (list, tuple)) and len(item) >= 2:
                    text_data = item[1]
                    text = text_data[0] if isinstance(text_data, tuple) else str(text_data)
                    if text.strip():
                        lines.append(text)
        elif isinstance(line_item, tuple) and len(line_item) >= 2:
            text_data = line_item[1]
            text = text_data[0] if isinstance(text_data, tuple) else str(text_data)
            if text.strip():
                lines.append(text)

    return "\n".join(lines)


def main() -> None:
    """CLI entry point."""
    parser = argparse.ArgumentParser(description="Generate PaddleOCR-VL reference baselines for image fixtures.")
    parser.add_argument(
        "--fixtures",
        type=Path,
        default=Path("../../fixtures"),
        help="Directory containing fixture images",
    )
    parser.add_argument(
        "--output",
        type=Path,
        default=Path("baselines/paddleocr_vl"),
        help="Directory to write baseline outputs",
    )
    parser.add_argument(
        "--device",
        choices=["cuda", "cpu"],
        default="cuda",
        help="Device to run model on (CUDA optional)",
    )

    args = parser.parse_args()

    args.output.mkdir(parents=True, exist_ok=True)

    log.info("=" * 70)
    log.info("PaddleOCR-VL Baseline Generation")
    log.info("=" * 70)
    log.info("Model: paddleocr-v4 (vision-language)")
    log.info("Size: ~700 MB - 2 GB (disk)")
    log.info(f"Device: {args.device}")
    log.info(f"Fixtures directory: {args.fixtures}")
    log.info(f"Output directory: {args.output}")
    log.info("=" * 70)

    if not args.fixtures.exists():
        log.error(f"Fixtures directory not found: {args.fixtures}")
        sys.exit(1)

    log.info("Loading PaddleOCR-VL model... (first run will download ~700MB-2GB)")
    try:
        backend, model = _load_paddleocr_vl()
        log.info(f"Model loaded successfully ({backend})")
    except ImportError as e:
        log.error(f"Failed to load model: {e}")
        log.error("Install with: pip install -r requirements.txt")
        sys.exit(1)

    image_files = sorted(f for f in args.fixtures.rglob("*") if _is_image_file(f))
    log.info(f"Found {len(image_files)} image files")

    if not image_files:
        log.warning("No image files found in fixtures directory")
        sys.exit(0)

    successful = 0
    failed = 0
    extract_fn = extract_sync_paddlex if backend == "paddlex" else extract_sync_paddleocr

    for i, img_path in enumerate(image_files, 1):
        log.info(f"[{i}/{len(image_files)}] Processing {img_path.name}...")

        result = extract_fn(str(img_path), model)

        fixture_stem = img_path.stem
        output_file = args.output / f"{fixture_stem}.paddleocr-vl.expected.txt"

        if "error" in result["metadata"]:
            failed += 1
            log.warning(f"  Failed: {result['metadata']['error']}")
        else:
            successful += 1
            output_file.write_text(result["content"], encoding="utf-8")
            timing_file = args.output / f"{fixture_stem}.paddleocr-vl.ms"
            timing_file.write_text(str(int(result["_extraction_time_ms"])), encoding="utf-8")
            log.info(f"  Saved: {output_file.name} ({result['_extraction_time_ms']:.0f} ms)")

    log.info("=" * 70)
    log.info("PaddleOCR-VL baseline generation complete")
    log.info(f"  Processed: {len(image_files)}")
    log.info(f"  Successful: {successful}")
    log.info(f"  Failed: {failed}")
    log.info(f"  Output: {args.output}")
    log.info("=" * 70)

    sys.exit(0 if failed == 0 else 2)


if __name__ == "__main__":
    main()
