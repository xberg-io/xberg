#!/usr/bin/env python3
"""Emit typed structural sidecars (`<id>.structural.json`) next to ground-truth `.md`.

The sidecar schema is the single source of truth shared with the Rust
`structural_sidecar` module (`tools/benchmark-harness/src/structural_sidecar.rs`).
The Rust `StructuralSidecar::from_markdown` is the *reference* parser; this script
mirrors its deterministic GFM rules so the Python-emitted JSON deserializes into
the same Rust type and scores identically.

Two sources are supported per document:

* **Markdown** (`<id>.md`) — GFM. Pipe tables cannot express row/column spans, so
  every cell is emitted with ``rowspan == colspan == 1`` and
  ``spans_recoverable == false``.
* **HTML** (`<id>.html`, when present alongside the `.md`) — real ``<table>``
  markup is read for genuine ``rowspan``/``colspan``; those tables are emitted
  with ``spans_recoverable == true``.

Node schema (internal ``kind`` tag, matching the Rust ``#[serde(tag = "kind")]``):

    {"kind": "heading",   "level": u8, "parent": int|null, "path": [str], "text": str}
    {"kind": "list_item", "depth": int, "ordered": bool, "parent_item": int|null, "text": str}
    {"kind": "table", "n_rows": int, "n_cols": int, "header_rows": int,
                      "cells": [{"row","col","rowspan","colspan","is_header","text"}],
                      "spans_recoverable": bool}
    {"kind": "figure",   "caption": int|null, "text": str}
    {"kind": "caption",  "binds_to": int|null, "text": str}
    {"kind": "footnote", "binds_to": int|null, "text": str}
    {"kind": "formula",  "display": bool, "text": str}
    {"kind": "image",    "alt": str}
    {"kind": "paragraph","text": str}

The document object is ``{"nodes": [...], "reading_order": [int]}``.

Usage:
    build_structural_sidecar.py <gt_dir> [<gt_dir> ...]
    build_structural_sidecar.py --file path/to/doc.md
"""

from __future__ import annotations

import argparse
import json
import re
import sys
from dataclasses import dataclass, field
from html.parser import HTMLParser
from pathlib import Path

CAPTION_PREFIXES = ("figure", "fig.", "table", "chart", "diagram", "scheme", "plate")
_FORMULA_CMDS = ("\\frac", "\\sum", "\\int", "\\begin{")


# --------------------------------------------------------------------------- #
# Markdown parsing (line-oriented, deterministic, mirrors the Rust rules)
# --------------------------------------------------------------------------- #


@dataclass
class Doc:
    """Accumulates typed structural nodes for one document as they are parsed."""

    nodes: list[dict] = field(default_factory=list)
    reading_order: list[int] = field(default_factory=list)

    def finish(self) -> dict:
        """Freeze reading order to parse order and return the sidecar dict."""
        self.reading_order = list(range(len(self.nodes)))
        return {"nodes": self.nodes, "reading_order": self.reading_order}


def _is_caption(text: str) -> bool:
    low = text.lstrip().lower()
    for pfx in CAPTION_PREFIXES:
        if low.startswith(pfx):
            rest = low[len(pfx) :].lstrip()
            if rest[:1].isdigit() or rest[:1] == ":":
                return True
    return False


def _is_footnote(text: str) -> bool:
    t = text.lstrip()
    return t.startswith("[^") and "]:" in t


def _is_formula(text: str) -> bool:
    t = text.lstrip()
    return t.startswith("\\") or any(cmd in text for cmd in _FORMULA_CMDS)


_HEADING_RE = re.compile(r"^(#{1,6})\s+(.*\S)\s*$")
_ULIST_RE = re.compile(r"^(\s*)[-*+]\s+(.*)$")
_OLIST_RE = re.compile(r"^(\s*)\d+[.)]\s+(.*)$")
_IMAGE_RE = re.compile(r"!\[([^\]]*)\]\([^)]*\)")
_TABLE_SEP_RE = re.compile(r"^\s*\|?\s*:?-{2,}:?\s*(\|\s*:?-{2,}:?\s*)*\|?\s*$")


def _split_row(line: str) -> list[str]:
    s = line.strip()
    s = s.removeprefix("|")
    s = s.removesuffix("|")
    return [c.strip() for c in s.split("|")]


def parse_markdown(md: str) -> dict:
    lines = md.splitlines()
    doc = Doc()
    heading_stack: list[tuple[int, int, str]] = []  # (level, node_index, text) ~keep
    para: list[str] = []

    def flush_para() -> None:
        if not para:
            return
        text = " ".join(" ".join(para).split()).strip()
        para.clear()
        if not text:
            return
        if _is_formula(text):
            doc.nodes.append({"kind": "formula", "display": True, "text": text})
        else:
            doc.nodes.append({"kind": "paragraph", "text": text})

    i = 0
    n = len(lines)
    while i < n:
        line = lines[i]
        stripped = line.strip()

        # blank line -> paragraph boundary
        if not stripped:
            flush_para()
            i += 1
            continue

        # standalone image line
        img = _IMAGE_RE.fullmatch(stripped)
        if img:
            flush_para()
            doc.nodes.append({"kind": "image", "alt": img.group(1).strip()})
            i += 1
            continue

        # heading
        h = _HEADING_RE.match(line)
        if h:
            flush_para()
            level = len(h.group(1))
            text = h.group(2).strip()
            while heading_stack and heading_stack[-1][0] >= level:
                heading_stack.pop()
            parent = heading_stack[-1][1] if heading_stack else None
            path = [t for (_, _, t) in heading_stack]
            idx = len(doc.nodes)
            doc.nodes.append({"kind": "heading", "level": level, "parent": parent, "path": path, "text": text})
            heading_stack.append((level, idx, text))
            i += 1
            continue

        # pipe table: header row followed by a separator row
        if "|" in line and i + 1 < n and _TABLE_SEP_RE.match(lines[i + 1]):
            flush_para()
            i = _parse_table(lines, i, doc)
            continue

        # list block
        if _ULIST_RE.match(line) or _OLIST_RE.match(line):
            flush_para()
            i = _parse_list_block(lines, i, doc)
            continue

        # otherwise: paragraph text
        para.append(stripped)
        i += 1

    flush_para()
    _bind_captions_and_footnotes(doc.nodes)
    return doc.finish()


def _parse_table(lines: list[str], start: int, doc: Doc) -> int:
    header = _split_row(lines[start])
    n_cols = len(header)
    cells: list[dict] = []
    for col, text in enumerate(header):
        cells.append({"row": 0, "col": col, "rowspan": 1, "colspan": 1, "is_header": True, "text": text})
    row = 1
    i = start + 2  # skip header + separator ~keep
    n = len(lines)
    while i < n and "|" in lines[i] and lines[i].strip():
        for col, text in enumerate(_split_row(lines[i])):
            cells.append({"row": row, "col": col, "rowspan": 1, "colspan": 1, "is_header": False, "text": text})
            n_cols = max(n_cols, col + 1)
        row += 1
        i += 1
    doc.nodes.append(
        {
            "kind": "table",
            "n_rows": row,
            "n_cols": n_cols,
            "header_rows": 1,
            "cells": cells,
            "spans_recoverable": False,
        }
    )
    return i


def _parse_list_block(lines: list[str], start: int, doc: Doc) -> int:
    """Parse a contiguous list block; nesting depth derived from indentation."""
    i = start
    n = len(lines)
    indents: list[int] = []  # stack of indent widths -> depth ~keep
    item_indices: list[int] = []  # node index per depth for parent linkage ~keep
    while i < n:
        line = lines[i]
        if not line.strip():
            # allow a single blank line inside a list, else end block ~keep
            if i + 1 < n and (_ULIST_RE.match(lines[i + 1]) or _OLIST_RE.match(lines[i + 1])):
                i += 1
                continue
            break
        m_u = _ULIST_RE.match(line)
        m_o = _OLIST_RE.match(line)
        if not (m_u or m_o):
            break
        ordered = m_o is not None
        m = m_o if ordered else m_u
        indent = len(m.group(1).expandtabs(4))
        text = m.group(2).strip()

        while indents and indent < indents[-1]:
            indents.pop()
            item_indices.pop()
        if not indents or indent > indents[-1]:
            indents.append(indent)
            item_indices.append(-1)
        depth = len(indents) - 1
        parent_item = item_indices[depth - 1] if depth > 0 else None

        idx = len(doc.nodes)
        doc.nodes.append(
            {
                "kind": "list_item",
                "depth": depth,
                "ordered": ordered,
                "parent_item": parent_item if parent_item != -1 else None,
                "text": text,
            }
        )
        item_indices[depth] = idx
        i += 1
    return i


def _nearest_preceding(nodes: list[dict], frm: int, kinds: tuple[str, ...]) -> int | None:
    for j in range(frm - 1, -1, -1):
        if nodes[j]["kind"] in kinds:
            return j
    return None


def _bind_captions_and_footnotes(nodes: list[dict]) -> None:
    for i, node in enumerate(nodes):
        if node["kind"] != "paragraph":
            continue
        text = node["text"]
        if _is_footnote(text):
            target = _nearest_preceding(nodes, i, ("paragraph", "table"))
            nodes[i] = {"kind": "footnote", "binds_to": target, "text": text}
        elif _is_caption(text):
            target = _nearest_preceding(nodes, i, ("image", "table", "figure"))
            nodes[i] = {"kind": "caption", "binds_to": target, "text": text}


# --------------------------------------------------------------------------- #
# HTML table span recovery (used when an `<id>.html` sits beside the `.md`)
# --------------------------------------------------------------------------- #


class _TableCollector(HTMLParser):
    """Collect tables with real rowspan/colspan from source HTML."""

    def __init__(self) -> None:
        super().__init__(convert_charrefs=True)
        self.tables: list[dict] = []
        self._cur: dict | None = None
        self._row = -1
        self._col = 0
        self._cell: dict | None = None
        self._in_head = False

    def handle_starttag(self, tag: str, attrs: list[tuple[str, str | None]]) -> None:
        a = dict(attrs)
        if tag == "table":
            self._cur = {"cells": [], "n_cols": 0, "header_rows": 0}
            self._row = -1
        elif tag == "thead":
            self._in_head = True
        elif tag == "tr" and self._cur is not None:
            self._row += 1
            self._col = 0
        elif tag in ("td", "th") and self._cur is not None:
            rowspan = int(a.get("rowspan") or 1)
            colspan = int(a.get("colspan") or 1)
            is_header = tag == "th" or self._in_head
            self._cell = {
                "row": self._row,
                "col": self._col,
                "rowspan": rowspan,
                "colspan": colspan,
                "is_header": is_header,
                "text": "",
            }
            self._col += colspan
            if is_header:
                self._cur["header_rows"] = max(self._cur["header_rows"], self._row + 1)
            self._cur["n_cols"] = max(self._cur["n_cols"], self._col)

    def handle_data(self, data: str) -> None:
        if self._cell is not None:
            self._cell["text"] += data

    def handle_endtag(self, tag: str) -> None:
        if tag in ("td", "th") and self._cell is not None:
            self._cell["text"] = " ".join(self._cell["text"].split()).strip()
            self._cur["cells"].append(self._cell)
            self._cell = None
        elif tag == "thead":
            self._in_head = False
        elif tag == "table" and self._cur is not None:
            n_rows = max((c["row"] for c in self._cur["cells"]), default=-1) + 1
            self.tables.append(
                {
                    "kind": "table",
                    "n_rows": n_rows,
                    "n_cols": self._cur["n_cols"],
                    "header_rows": self._cur["header_rows"] or (1 if n_rows else 0),
                    "cells": self._cur["cells"],
                    "spans_recoverable": True,
                }
            )
            self._cur = None


def _html_tables(html: str) -> list[dict]:
    collector = _TableCollector()
    collector.feed(html)
    return collector.tables


def _apply_html_spans(doc: dict, html: str) -> None:
    """Replace GFM tables in-order with span-aware HTML tables where counts match."""
    html_tables = _html_tables(html)
    if not html_tables:
        return
    md_table_positions = [i for i, n in enumerate(doc["nodes"]) if n["kind"] == "table"]
    for pos, html_table in zip(md_table_positions, html_tables, strict=False):
        doc["nodes"][pos] = html_table


# --------------------------------------------------------------------------- #
# Driver
# --------------------------------------------------------------------------- #


def build_for_markdown_file(md_path: Path) -> Path:
    doc = parse_markdown(md_path.read_text(encoding="utf-8"))
    html_path = md_path.with_suffix(".html")
    if html_path.exists():
        _apply_html_spans(doc, html_path.read_text(encoding="utf-8"))
    out_path = md_path.with_suffix(".structural.json")
    out_path.write_text(json.dumps(doc, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    return out_path


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description="Emit typed structural sidecars beside ground-truth markdown.")
    parser.add_argument("paths", nargs="*", help="directories to scan for *.md ground truth")
    parser.add_argument("--file", help="build a sidecar for a single markdown file")
    args = parser.parse_args(argv)

    targets: list[Path] = []
    if args.file:
        targets.append(Path(args.file))
    for d in args.paths:
        targets.extend(sorted(Path(d).rglob("*.md")))

    if not targets:
        parser.error("no input: pass a directory or --file")

    count = 0
    for md_path in targets:
        if md_path.name.endswith(".structural.json"):
            continue
        out = build_for_markdown_file(md_path)
        count += 1
        print(f"wrote {out}")
    print(f"done: {count} sidecar(s)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
