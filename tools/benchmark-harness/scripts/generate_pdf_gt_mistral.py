#!/usr/bin/env python3
"""Generate PDF markdown ground truth using Mistral's pixtral vision model.

Usage:
    # Generate GT for all PDFs missing MD GT:
    python generate_pdf_gt_mistral.py

    # Generate GT for a specific fixture:
    python generate_pdf_gt_mistral.py tools/benchmark-harness/fixtures/pdf/2203.01017v2.json

    # Dry run (show what would be generated):
    python generate_pdf_gt_mistral.py --dry-run

    # Pilot batch (first N):
    python generate_pdf_gt_mistral.py --limit 10
"""

import argparse
import base64
import json
import os
import sys
import time
from pathlib import Path

MISTRAL_API_KEY = os.environ.get("MISTRAL_API_KEY", "")
MISTRAL_MODEL = "mistral-ocr-latest"
MISTRAL_API_URL = "https://api.mistral.ai/v1/ocr"

PROMPT = (
    "Convert this PDF to clean GFM (GitHub Flavored Markdown). "
    "Preserve the document structure: headings, paragraphs, tables, lists, "
    "code blocks, and formulas. Use proper heading hierarchy (# for title, ## for sections). "
    "Render tables as GFM pipe tables. Do not add commentary or explanations."
)


def load_env():
    """Load MISTRAL_API_KEY from ../liter-llm/.env if not in environment."""
    global MISTRAL_API_KEY
    if MISTRAL_API_KEY:
        return
    env_path = Path(__file__).resolve().parents[3] / ".." / "liter-llm" / ".env"
    if env_path.exists():
        for line in env_path.read_text().splitlines():
            if line.startswith("MISTRAL_API_KEY="):
                MISTRAL_API_KEY = line.split("=", 1)[1].strip()
                return
    print("ERROR: MISTRAL_API_KEY not found", file=sys.stderr)
    sys.exit(1)


def call_mistral_ocr(pdf_path: str) -> str:
    """Send a PDF to Mistral OCR and return markdown."""
    import httpx

    pdf_data = Path(pdf_path).read_bytes()
    b64 = base64.standard_b64encode(pdf_data).decode("ascii")

    payload = {
        "model": MISTRAL_MODEL,
        "document": {
            "type": "document_url",
            "document_url": f"data:application/pdf;base64,{b64}",
        },
    }

    resp = httpx.post(
        MISTRAL_API_URL,
        json=payload,
        headers={
            "Authorization": f"Bearer {MISTRAL_API_KEY}",
            "Content-Type": "application/json",
        },
        timeout=120.0,
    )
    resp.raise_for_status()
    data = resp.json()

    # Extract markdown from pages
    pages = data.get("pages", [])
    if not pages:
        return ""
    return "\n\n".join(p.get("markdown", "") for p in pages)


def find_fixtures_needing_gt() -> list[tuple[str, str, str]]:
    """Find PDF fixtures that don't have markdown GT.
    Returns list of (fixture_path, pdf_path, gt_md_path).
    """
    fixtures_dir = Path("tools/benchmark-harness/fixtures/pdf")
    results = []

    for f in sorted(fixtures_dir.glob("*.json")):
        data = json.loads(f.read_text())
        gt = data.get("ground_truth")
        if gt is None:
            continue
        if gt.get("markdown_file"):
            continue  # Already has MD GT

        doc_path = data.get("document", "")
        pdf_path = str((f.parent / doc_path).resolve())
        if not Path(pdf_path).exists():
            continue

        # Determine GT output path
        text_file = gt.get("text_file", "")
        if text_file:
            gt_md = text_file.rsplit(".", 1)[0] + ".md"
        else:
            name = Path(doc_path).stem
            gt_md = f"../../../../test_documents/ground_truth/pdf/{name}.md"

        gt_md_path = str((f.parent / gt_md).resolve())
        results.append((str(f), pdf_path, gt_md_path))

    return results


def process_fixture(fixture_path: str, pdf_path: str, gt_md_path: str, dry_run: bool = False) -> bool:
    """Process a single fixture. Returns True if successful."""
    name = Path(pdf_path).stem
    size_mb = Path(pdf_path).stat().st_size / (1024 * 1024)

    if dry_run:
        print(f"  [dry-run] {name} ({size_mb:.1f}MB) → {gt_md_path}")
        return True

    print(f"  Processing {name} ({size_mb:.1f}MB)...", end=" ", flush=True)

    try:
        markdown = call_mistral_ocr(pdf_path)
        if not markdown.strip():
            print("EMPTY")
            return False

        # Sanitize
        from sanitize_pandoc_gt import sanitize

        markdown = sanitize(markdown)

        # Write GT file
        Path(gt_md_path).parent.mkdir(parents=True, exist_ok=True)
        Path(gt_md_path).write_text(markdown)

        # Update fixture JSON
        data = json.loads(Path(fixture_path).read_text())
        gt = data["ground_truth"]
        # Compute relative path from fixture to GT
        rel_path = os.path.relpath(gt_md_path, Path(fixture_path).parent)
        gt["markdown_file"] = rel_path
        gt["source"] = "mistral-pixtral"
        Path(fixture_path).write_text(json.dumps(data, indent=2) + "\n")

        print(f"OK ({len(markdown)} bytes)")
        return True

    except Exception as e:
        print(f"ERROR: {e}")
        return False


def main():
    parser = argparse.ArgumentParser(description="Generate PDF GT with Mistral OCR")
    parser.add_argument("fixture", nargs="?", help="Specific fixture JSON to process")
    parser.add_argument("--dry-run", action="store_true", help="Show what would be done")
    parser.add_argument("--limit", type=int, default=0, help="Process only first N fixtures")
    parser.add_argument("--delay", type=float, default=1.0, help="Delay between API calls (seconds)")
    args = parser.parse_args()

    load_env()

    if args.fixture:
        # Process single fixture
        data = json.loads(Path(args.fixture).read_text())
        doc_path = data.get("document", "")
        pdf_path = str((Path(args.fixture).parent / doc_path).resolve())
        gt = data.get("ground_truth", {})
        text_file = gt.get("text_file", "")
        if text_file:
            gt_md = text_file.rsplit(".", 1)[0] + ".md"
        else:
            gt_md = f"../../../../test_documents/ground_truth/pdf/{Path(doc_path).stem}.md"
        gt_md_path = str((Path(args.fixture).parent / gt_md).resolve())
        process_fixture(args.fixture, pdf_path, gt_md_path, dry_run=args.dry_run)
        return

    # Process all fixtures needing GT
    fixtures = find_fixtures_needing_gt()
    print(f"Found {len(fixtures)} PDF fixtures needing markdown GT")

    if args.limit > 0:
        fixtures = fixtures[: args.limit]
        print(f"Processing first {args.limit}")

    success = 0
    failed = 0
    for fixture_path, pdf_path, gt_md_path in fixtures:
        ok = process_fixture(fixture_path, pdf_path, gt_md_path, dry_run=args.dry_run)
        if ok:
            success += 1
        else:
            failed += 1
        if not args.dry_run and args.delay > 0:
            time.sleep(args.delay)

    print(f"\nDone: {success} generated, {failed} failed")


if __name__ == "__main__":
    main()
