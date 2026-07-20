#!/usr/bin/env python3
"""Calibrated import gate: score a candidate GT markdown against an independent PDF text oracle.

This is the gate every document passes before entering the corpus. It does NOT assume the GT text is
correct or its structure real. The rules are calibrated from a hand-adjudication of 15 flagged docs
(see corpus-rebuild memory): the naive v1 thresholds false-positived 10/15, so this version uses the
signals that actually discriminated:

- **Truncation (REJECT)** — GT is a clean *prefix* of the oracle that ends early: high token-recall on
  the oracle's leading portion, ~0 recall on its tail, with low overall coverage. (Raw low coverage
  alone false-fires on charts / intentionally-partial GT, so we require the prefix-drop shape.)
- **Table fabrication (REJECT)** — a GT table whose *header/label* tokens are absent from the oracle
  (docling's invented row/col labels scored 0). Value-token support and adjacency were NOT reliable
  (fabrication kept the real numbers; figures/forms false-fired), so we key on header tokens only.
- **Hallucination (REVIEW, escalate to REJECT on digit contradiction)** — on a native-text doc, GT
  vocabulary absent from the oracle is suspect, but merely-added logo/chart text is benign, so
  low precision routes to human REVIEW. A hard digit contradiction (GT standalone numbers largely
  absent from the oracle) escalates to REJECT (pdfa_019: "3.83"→"3D").
- **Scanned / image pages (NO_ORACLE / REVIEW)** — pages with ~0 extractable text make pdftotext an
  invalid reference; route to human review / a second OCR oracle, NEVER auto-reject.

pdftotext is triage, not truth. Scans and REVIEW docs need a second OCR oracle or human sign-off
before ACCEPT — those hooks are marked, not silently passed.

Usage:
    python gt_eval.py <fixture.json> ...           # score fixtures (fixture schema)
    from gt_eval import evaluate                    # evaluate(pdf, gt_md, source) -> Eval
"""

from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
from dataclasses import dataclass, field
from pathlib import Path

MIN_ORACLE_TOKENS = 25
EMPTY_PAGE_TOKENS = 5  # a page with < this many tokens is "empty" (image/scan) ~keep
SCAN_PAGE_FRACTION = 0.40  # >= this fraction empty => oracle unreliable => NO_ORACLE/REVIEW
TRUNC_COVERAGE = 0.55  # overall gt/oracle token ratio below this is a truncation candidate ~keep
TRUNC_HEAD_RECALL = 0.55  # leading-half oracle recall must be at least this... ~keep
TRUNC_TAIL_RECALL = 0.18  # ...and trailing-quarter recall at most this => clean prefix drop ~keep
TRUNC_EXTREME_COVERAGE = 0.25  # GT below this fraction of a multi-page native doc => truncated ~keep
TABLE_LABEL_SUPPORT = 0.40  # a table whose header tokens fall below this => fabricated ~keep
HALLUC_PRECISION = 0.70  # native-text precision below this => REVIEW
DIGIT_CONTRADICTION = 0.35  # >= this fraction of GT numbers absent from oracle => REJECT

_WORD = re.compile(r"[a-z0-9]+")
_NUM = re.compile(r"\d[\d.,/]*\d|\d")
# Math and code carry numbers pdftotext cannot faithfully extract (subscripts, equation glyphs), so a
# GT-vs-oracle digit comparison over them is noise. Strip them before extracting prose numbers. ~keep
_MATH_CODE = re.compile(r"\$\$.*?\$\$|\$[^$\n]*\$|```.*?```|`[^`\n]*`", re.DOTALL)
_TABLE_ROW = re.compile(r"^\s*\|.*\|\s*$")
_TABLE_SEP = re.compile(r"^\s*\|?[\s:|-]*-[\s:|-]*\|?\s*$")


def tokens(text: str) -> list[str]:
    return _WORD.findall(text.lower())


def numbers(text: str) -> list[str]:
    return [n.replace(",", "") for n in _NUM.findall(text)]


def oracle_pages(pdf_path: Path) -> tuple[list[list[str]], str]:
    r"""Independent extraction: (per-page token lists, raw text). pdftotext emits \f between pages.
    ([], "") => no extractable text layer.
    """
    try:
        out = subprocess.run(
            ["pdftotext", "-layout", "-enc", "UTF-8", str(pdf_path), "-"],
            capture_output=True,
            timeout=180,
        ).stdout.decode("utf-8", "replace")
    except (subprocess.SubprocessError, FileNotFoundError):
        return [], ""
    return [tokens(pg) for pg in out.split("\f")], out


_CITATION = re.compile(r"\[(\d{1,3})\]")


def max_citation(text: str) -> int:
    """Highest in-text bracket citation [N] — a cheap arXiv version-drift signal (GT derived from a
    later source version cites references the rendered PDF's bibliography does not contain).
    """
    nums = [int(m.group(1)) for m in _CITATION.finditer(text)]
    return max(nums) if nums else 0


def table_header_tokens(md: str) -> list[list[str]]:
    """First (header) row token list per GFM pipe table — the labels a fabricator invents."""
    headers: list[list[str]] = []
    prev: list[list[str]] | None = None
    expecting = False
    for line in md.splitlines():
        if _TABLE_ROW.match(line) and not _TABLE_SEP.match(line):
            if not expecting:
                prev = [tokens(c) for c in line.strip().strip("|").split("|")]
                expecting = True
        elif _TABLE_SEP.match(line) and expecting and prev is not None:
            headers.append([t for cell in prev for t in cell])
            expecting = False
        else:
            expecting = False
    return [h for h in headers if h]


@dataclass
class Eval:
    """Gate outcome for one document: verdict, reasons, execution cohorts, and oracle scores."""

    verdict: str = "PENDING"
    reasons: list = field(default_factory=list)
    cohorts: list = field(default_factory=list)
    pages: int = 0
    empty_pages: int = 0
    oracle_tokens: int = 0
    gt_tokens: int = 0
    recall: float = 0.0
    precision: float = 0.0
    coverage: float = 0.0
    head_recall: float = 0.0
    tail_recall: float = 0.0
    worst_table_label_support: float = 1.0
    digit_contradiction: float = 0.0


def _segment_recall(oracle_flat: list[str], gt_set: set[str], lo: float, hi: float) -> float:
    a, b = int(len(oracle_flat) * lo), int(len(oracle_flat) * hi)
    seg = oracle_flat[a:b]
    if not seg:
        return 1.0
    return len([t for t in seg if t in gt_set]) / len(seg)


def evaluate(pdf_path: Path, gt_md: str, source: str = "") -> Eval:
    ev = Eval()
    g_tok = tokens(gt_md)
    ev.gt_tokens = len(g_tok)
    # Empty / near-empty GT (e.g. a failed html_to_gfm produced "") is always bad, oracle-independent. ~keep
    if len(g_tok) < 3:
        ev.verdict, ev.reasons = "REJECT", [f"empty/near-empty GT ({len(g_tok)} tokens)"]
        return ev

    pages, oracle_raw = oracle_pages(pdf_path)
    ev.pages = len(pages)
    ev.empty_pages = sum(1 for p in pages if len(p) < EMPTY_PAGE_TOKENS)
    oracle_flat = [t for p in pages for t in p]
    o_set, g_set = set(oracle_flat), set(g_tok)
    ev.oracle_tokens = len(oracle_flat)

    # No / near-empty oracle => cannot validate here.
    if len(oracle_flat) < MIN_ORACLE_TOKENS:
        ev.verdict = "NO_ORACLE" if not oracle_flat else "TRIVIAL"
        ev.reasons = [
            "no extractable text layer (scanned/image)" if not oracle_flat else f"oracle only {len(oracle_flat)} tokens"
        ]
        ev.cohorts = ["forced-OCR"] if not oracle_flat else []
        return ev

    inter = len(g_set & o_set)
    ev.recall = inter / len(o_set)
    ev.precision = inter / max(1, len(g_set))
    ev.coverage = len(g_tok) / max(1, len(oracle_flat))
    ev.head_recall = _segment_recall(oracle_flat, g_set, 0.0, 0.5)
    ev.tail_recall = _segment_recall(oracle_flat, g_set, 0.75, 1.0)

    # A page with no text layer only matters when it HIDES GT content — i.e. coverage is low. A blank
    # trailing page (common in per-page crops; GT fully covered by the text page) is not a scan. ~keep
    frac_empty = ev.empty_pages / max(1, ev.pages)
    scan_suspect = frac_empty >= SCAN_PAGE_FRACTION and ev.coverage < TRUNC_COVERAGE
    if scan_suspect:
        ev.cohorts.append("forced-OCR" if frac_empty > 0.8 else "selective-OCR")
    else:
        ev.cohorts.append("native-clean")

    reasons = []
    # Truncation, shape A: clean prefix whose recall drops off in the oracle tail. ~keep
    if ev.coverage < TRUNC_COVERAGE and ev.head_recall >= TRUNC_HEAD_RECALL and ev.tail_recall <= TRUNC_TAIL_RECALL:
        reasons.append(
            f"truncated: covers {ev.coverage:.2f}, head-recall {ev.head_recall:.2f} vs "
            f"tail-recall {ev.tail_recall:.2f} (clean prefix ends early)"
        )
    # Truncation, shape B: GT is a tiny fraction of a native doc (single- or multi-page). Catches the
    # prefix-drop miss when repeated boilerplate keeps tail-recall high (federal-register: cov 0.10) and
    # the single-page case the prefix rule cannot see (a 1-page PDF whose GT covers a sliver of it). ~keep
    elif "native-clean" in ev.cohorts and ev.coverage < TRUNC_EXTREME_COVERAGE:
        reasons.append(f"truncated: GT is {ev.coverage:.2f} of a {ev.pages}-page native doc (extreme length gap)")

    # Table fabrication: header/label tokens absent from oracle. Skip whenever a meaningful fraction of
    # pages have no text layer — the table's labels may legitimately live on an image page (else a real ~keep
    # table on a partly-scanned doc false-REJECTs). Such docs fall through to the scan/precision REVIEW.
    if frac_empty < SCAN_PAGE_FRACTION:
        for hdr in table_header_tokens(gt_md):
            sup = len([t for t in hdr if t in o_set]) / max(1, len(hdr))
            ev.worst_table_label_support = min(ev.worst_table_label_support, sup)
        if ev.worst_table_label_support < TABLE_LABEL_SUPPORT:
            reasons.append(
                f"fabricated table: header-token support {ev.worst_table_label_support:.2f} < {TABLE_LABEL_SUPPORT}"
            )

    # NOTE: an exact-string GT-vs-oracle digit-contradiction check was tried and removed — it
    # false-rejected 30% of authoritative source-derived academic GT (pdftotext mangles subscript/
    # equation/citation number glyphs), while its only real win (prose hallucination like pdfa_019)
    # is already routed to REVIEW via the scanned-page path. Truncation + table-label + precision
    # cover the real defects without that noise. `numbers()`/`_MATH_CODE` kept for the sidecar work. ~keep

    if reasons:
        ev.verdict, ev.reasons = "REJECT", reasons
        return ev

    # arXiv version drift: ReaDoc arXiv GT is derived from the LaTeX SOURCE, which may be a different
    # version than the rendered PDF. The token gate can't see it (body matches), but the GT then cites ~keep
    # references absent from the PDF bibliography. High-signal, cheap; routes to human REVIEW.
    gt_cite, oracle_cite = max_citation(gt_md), max_citation(oracle_raw)
    version_drift = "readoc" in source and gt_cite > oracle_cite + 15 and gt_cite > 1.3 * max(1, oracle_cite)

    # Hallucination / scan / drift => REVIEW (needs 2nd OCR or human), else ACCEPT.
    if scan_suspect:
        ev.verdict = "REVIEW"
        ev.reasons = [f"{ev.empty_pages}/{ev.pages} pages have no text layer + low coverage — needs 2nd OCR oracle"]
    elif version_drift:
        ev.verdict = "REVIEW"
        ev.reasons = [
            f"possible arXiv version drift: GT cites [{gt_cite}] but PDF bibliography tops out near [{oracle_cite}]"
        ]
    elif ev.precision < HALLUC_PRECISION and 0.6 <= ev.coverage <= 1.6:
        ev.verdict = "REVIEW"
        ev.reasons = [f"precision {ev.precision:.2f} on native-text doc — possible hallucination"]
    else:
        ev.verdict = "ACCEPT"
    return ev


def _resolve(fixture: Path):
    d = json.loads(fixture.read_text())
    gt = d.get("ground_truth") or {}
    if not gt.get("markdown_file"):
        return None
    return (
        (fixture.parent / d["document"]).resolve(),
        (fixture.parent / gt["markdown_file"]).resolve(),
        gt.get("source", ""),
    )


def main() -> int:
    ap = argparse.ArgumentParser(description="Calibrated GT import gate")
    ap.add_argument("fixtures", nargs="+")
    args = ap.parse_args()
    for f in args.fixtures:
        r = _resolve(Path(f))
        if not r:
            continue
        pdf, gt_md, source = r
        ev = evaluate(pdf, gt_md.read_text("utf-8", "replace"), source)
        print(
            f"{ev.verdict:9} cov={ev.coverage:.2f} head={ev.head_recall:.2f} tail={ev.tail_recall:.2f} "
            f"prec={ev.precision:.2f} tbl={ev.worst_table_label_support:.2f} "
            f"[{','.join(ev.cohorts)}] {Path(f).stem}" + (f" — {'; '.join(ev.reasons)}" if ev.reasons else "")
        )
    return 0


if __name__ == "__main__":
    sys.exit(main())
