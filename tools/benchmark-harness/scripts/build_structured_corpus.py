#!/usr/bin/env python3
"""Reproducible builder for the ParseBench STRUCTURED-EXTRACTION ground-truth subset.

This is the VLM / structured axis of the xberg benchmark corpus (companion to the PDF->Markdown
corpus produced by ``build_corpus.py``). It stages ParseBench's four *structured* GT splits — which
score a parser with per-rule assertions rather than whole-document Markdown similarity — plus their
source PDFs, into the layout the harness expects.

    chart            chart_data_point rules (read a value off a chart)
    layout           layout / order rules (block bounding boxes + reading order)
    text_content     content-faithfulness rules (missing/unexpected words & sentences, order, ...)
    text_formatting  inline-formatting rules (bold/italic/title/latex/... spans)

Licensing (matches ``test_documents/LICENSES.md``):

- The four ``*.jsonl`` GT files are ParseBench's OWN annotations -> Apache-2.0 -> **VENDOR**
  (committed) under ``test_documents/ground_truth/structured/parsebench/<split>.jsonl``.
- The source PDFs are the SAME third-party enterprise PDFs as ParseBench's table split ->
  **REFERENCE** (not committed). They land in the gitignored
  ``test_documents/.corpus-cache/structured/pdf/`` and are referenced from the manifest by path +
  sha256 for provenance only.

The builder does NOT wire scoring: the existing ``json_quality.rs`` / ``field_quality.rs`` do not fit
these per-rule assertion formats. Its job is to SAVE the subset faithfully for a future rule-runner.

Usage:
    python build_structured_corpus.py                 # build from /tmp/parsebench into test_documents/
    python build_structured_corpus.py --source DIR    # override the ParseBench staging dir
    python build_structured_corpus.py --dry-run       # plan only, no writes

Reproducible + idempotent: sorted iteration, content-hash-guarded copies (a file is only rewritten
when its bytes change), and a pinned upstream revision recorded in the manifest.
"""

from __future__ import annotations

import argparse
import collections
import hashlib
import json
import os
import shutil
import sys
from pathlib import Path

# --- pinned provenance -------------------------------------------------------------------------
REPO = "llamaindex/ParseBench"
REVISION = "2805a1d940f95a203e0ae4b88be9934f7765b3fc"
LICENSE = "Apache-2.0"
URL = "https://huggingface.co/datasets/llamaindex/ParseBench"

MANIFEST_SCHEMA = 1

# Split -> (xberg capability it exercises, the cohort/capability tag surfaced for querying).
# Deterministic order also decides which subdir a byte-identical PDF is sourced from on collisions. ~keep
SPLITS = ("chart", "layout", "text_content", "text_formatting")
SPLIT_META = {
    "chart": {"capability": "chart-extraction", "cohort": "chart"},
    "layout": {"capability": "layout-detection", "cohort": "layout"},
    "text_content": {"capability": "content-faithfulness", "cohort": "faithfulness"},
    "text_formatting": {"capability": "inline-formatting", "cohort": "formatting"},
}

# --- repo-relative destinations ----------------------------------------------------------------
REPO_ROOT = Path(__file__).resolve().parents[3]
TEST_DOCS = REPO_ROOT / "test_documents"
GT_OUT_DIR = TEST_DOCS / "ground_truth" / "structured" / "parsebench"
# Gitignored on-demand cache for `reference`-class content (see test_documents/.gitignore). ~keep
CACHE_PDF_DIR = TEST_DOCS / ".corpus-cache" / "structured" / "pdf"
MANIFEST = TEST_DOCS / "ground_truth" / "structured_manifest.json"

DEFAULT_SOURCE = Path("/tmp/parsebench")  # noqa: S108  ParseBench staging (jsonl + docs/<split>/*.pdf)


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with open(path, "rb") as handle:
        for chunk in iter(lambda: handle.read(65536), b""):
            digest.update(chunk)
    return digest.hexdigest()


def doc_id(pdf_rel: str) -> str:
    """`docs/chart/Some File_p1.pdf` -> `pb_Some_File_p1` (spaces -> underscores)."""
    stem = os.path.splitext(os.path.basename(pdf_rel))[0]
    return "pb_" + stem.replace(" ", "_")


def copy_if_changed(src: Path, dst: Path, dry_run: bool) -> None:
    """Copy only when the destination is absent or its bytes differ — keeps re-runs idempotent."""
    if dst.exists() and sha256_file(dst) == sha256_file(src):
        return
    if dry_run:
        return
    dst.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(src, dst)


def load_split(source: Path, split: str):
    """Yield parsed records for one split, tolerating a trailing blank line."""
    with open(source / f"{split}.jsonl", encoding="utf-8") as handle:
        for line in handle:
            line = line.strip()
            if line:
                yield json.loads(line)


def build(source: Path, dry_run: bool) -> dict:
    if not source.exists():
        sys.exit(f"ParseBench source dir not found: {source}")

    if not dry_run:
        GT_OUT_DIR.mkdir(parents=True, exist_ok=True)
        CACHE_PDF_DIR.mkdir(parents=True, exist_ok=True)

    # per-doc accumulation keyed by pb id
    docs: dict[str, dict] = {}
    # deterministic source PDF selection: first split (in SPLITS order) that references the doc wins;
    # collisions across split subdirs must be byte-identical or we abort. ~keep
    pdf_source: dict[str, Path] = {}
    split_summary: dict[str, dict] = {}

    for split in SPLITS:
        src_jsonl = source / f"{split}.jsonl"
        if not src_jsonl.exists():
            sys.exit(f"missing split file: {src_jsonl}")

        per_doc = collections.defaultdict(
            lambda: {
                "n_rules": 0,
                "rule_types": collections.Counter(),
                "tags": collections.Counter(),
                "upstream": None,
                "pdf_rel": None,
            }
        )
        total_rules = 0
        type_counter: collections.Counter = collections.Counter()

        for record in load_split(source, split):
            total_rules += 1
            type_counter[record["type"]] += 1
            did = doc_id(record["pdf"])
            entry = per_doc[did]
            entry["n_rules"] += 1
            entry["rule_types"][record["type"]] += 1
            entry["upstream"] = os.path.splitext(os.path.basename(record["pdf"]))[0]
            entry["pdf_rel"] = record["pdf"]
            for tag in record.get("tags") or []:
                entry["tags"][tag] += 1

        # vendor the split GT verbatim (committed, Apache-2.0), hash the copy we ship ~keep
        vendored = GT_OUT_DIR / f"{split}.jsonl"
        copy_if_changed(src_jsonl, vendored, dry_run)
        gt_hash = sha256_file(vendored if vendored.exists() else src_jsonl)

        split_summary[split] = {
            "capability": SPLIT_META[split]["capability"],
            "cohort": SPLIT_META[split]["cohort"],
            "n_docs": len(per_doc),
            "n_rules": total_rules,
            "rule_types": dict(sorted(type_counter.items())),
            "gt_file": f"ground_truth/structured/parsebench/{split}.jsonl",
            "gt_sha256": gt_hash,
        }

        for did in sorted(per_doc):
            acc = per_doc[did]
            src_pdf = source / acc["pdf_rel"]
            # register / validate the reference PDF source for this id
            if did in pdf_source:
                if sha256_file(pdf_source[did]) != sha256_file(src_pdf):
                    sys.exit(f"pb id collision with differing bytes for {did}: {pdf_source[did]} vs {src_pdf}")
            else:
                pdf_source[did] = src_pdf

            doc = docs.setdefault(
                did,
                {
                    "id": did,
                    "source_dataset": "parsebench",
                    "upstream_id": acc["upstream"],
                    "pdf": None,
                    "pdf_sha256": None,
                    "source_url": URL,
                    "license": LICENSE,
                    "source_revision": REVISION,
                    "redistribute": "reference",
                    "splits": {},
                    "_cohorts": set(),
                },
            )
            doc["splits"][split] = {
                "n_rules": acc["n_rules"],
                "rule_types": dict(sorted(acc["rule_types"].items())),
                "tags": dict(sorted(acc["tags"].items())),
            }
            doc["_cohorts"].add(SPLIT_META[split]["cohort"])
            for tag in acc["tags"]:
                doc["_cohorts"].add(tag)

    # materialize reference PDFs into the gitignored cache + finalize per-doc entries
    pdf_present = 0
    for did in sorted(docs):
        doc = docs[did]
        src_pdf = pdf_source[did]
        dst_pdf = CACHE_PDF_DIR / f"{did}.pdf"
        if src_pdf.exists():
            copy_if_changed(src_pdf, dst_pdf, dry_run)
            doc["pdf"] = f".corpus-cache/structured/pdf/{did}.pdf"
            doc["pdf_sha256"] = sha256_file(dst_pdf if dst_pdf.exists() else src_pdf)
            pdf_present += 1
        else:
            doc["pdf"] = None
            doc["pdf_sha256"] = None
            doc["pdf_missing"] = True
        doc["cohorts"] = sorted(doc.pop("_cohorts"))

    manifest = {
        "schema": MANIFEST_SCHEMA,
        "kind": "structured_manifest",
        "description": (
            "ParseBench structured-extraction ground truth (chart / layout / text_content / "
            "text_formatting splits) staged for the xberg benchmark corpus. GT is vendored "
            "(Apache-2.0); source PDFs are reference-only and live in the gitignored corpus cache."
        ),
        "source": {"repo": REPO, "revision": REVISION, "license": LICENSE, "url": URL},
        "splits": split_summary,
        "counts": {
            "distinct_docs": len(docs),
            "docs_with_pdf": pdf_present,
            "docs_missing_pdf": len(docs) - pdf_present,
            "total_rules": sum(s["n_rules"] for s in split_summary.values()),
        },
        "documents": [docs[did] for did in sorted(docs)],
    }

    if not dry_run:
        MANIFEST.parent.mkdir(parents=True, exist_ok=True)
        with open(MANIFEST, "w", encoding="utf-8") as handle:
            json.dump(manifest, handle, indent=2, ensure_ascii=False)
            handle.write("\n")

    return manifest


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source", type=Path, default=DEFAULT_SOURCE, help="ParseBench staging dir")
    parser.add_argument("--dry-run", action="store_true", help="plan only, no writes")
    args = parser.parse_args()

    manifest = build(args.source, args.dry_run)

    mode = "DRY-RUN (no writes)" if args.dry_run else "WROTE"
    print(f"structured corpus build [{mode}]")
    print(f"  source     : {args.source}")
    print(f"  vendor GT  : {GT_OUT_DIR}")
    print(f"  cache PDFs : {CACHE_PDF_DIR}")
    print(f"  manifest   : {MANIFEST}")
    print("SPLIT SUMMARY")
    for split, summary in manifest["splits"].items():
        print(f"  {split:16s} docs={summary['n_docs']:4d} rules={summary['n_rules']:6d} cap={summary['capability']}")
    counts = manifest["counts"]
    print(
        f"distinct docs (union): {counts['distinct_docs']}  "
        f"with_pdf={counts['docs_with_pdf']}  missing_pdf={counts['docs_missing_pdf']}  "
        f"total_rules={counts['total_rules']}"
    )


if __name__ == "__main__":
    main()
