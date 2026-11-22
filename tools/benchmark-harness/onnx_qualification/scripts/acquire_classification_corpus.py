#!/usr/bin/env python3
# The only URLs this tool opens are the pinned public API and image endpoints, and ``open_url``
# rejects every scheme other than http/https before the request is built. ~keep
# ruff: noqa: S310
"""Acquire the held-out document-type corpus for CPU classifier qualification.

Source: the U.S. Federal Register public API (federalregister.gov). Federal Register
documents are U.S. Government works and therefore public domain (17 U.S.C. section 105).
The corpus is reproducible: every selected document is pinned by its Federal Register
document number, its publication date and the SHA-256 of the exact bytes fetched. Nothing
here redistributes a document that the API does not already serve publicly.

The document ``type`` published by the Office of the Federal Register is authoritative
metadata, never inferred from the filename or the text. A family is a top-level issuing
agency: documents produced by one agency share headings and boilerplate, so a family is
kept entirely inside one split.

The fixed metadata block that states the type (``[Proposed Rules]``, ``ACTION: ...``) is
removed before the text is frozen, because leaving it in place would put the gold label in
the model input. Body text still says "this rule" or "we propose" in many documents; the
audit reports that residual signal instead of pretending it was removed.
"""

from __future__ import annotations

import argparse
import hashlib
import html
import json
import re
import sys
import time
import urllib.error
import urllib.parse
import urllib.request
from collections import Counter, defaultdict
from dataclasses import dataclass
from pathlib import Path
from urllib.parse import urlparse

API = "https://www.federalregister.gov/api/v1/documents.json"
USER_AGENT = "xberg-qualification/1.0 (model qualification; xberg-io)"

# Federal Register type code -> (folded type, model label). A document whose type is not in
# ``TYPES`` is an unknown-class example: it is frozen with an empty gold set so the run can
# measure whether the classifier abstains instead of forcing it into the known vocabulary.
TYPES = {
    "Rule": ("RULE", "final rule"),
    "Proposed Rule": ("PRORULE", "proposed rule"),
    "Notice": ("NOTICE", "public notice"),
}
UNKNOWN_TYPE = ("Presidential Document", "PRESDOCU", "presidential document")

SPLIT_WEIGHTS = {"training": 4, "calibration": 2, "held_out": 4}
SPLIT_ORDER = ["training", "calibration", "held_out"]

MAX_TEXT_BYTES = 60_000
MAX_PER_FAMILY = 12


@dataclass(frozen=True)
class Record:
    """One document selected from the Federal Register API."""

    document_number: str
    type_code: str
    folded_type: str
    family: str
    family_name: str
    subfamily: str
    publication_date: str
    title: str
    source_url: str
    agencies: tuple[str, ...]


def open_url(url: str, timeout: int = 60) -> bytes:
    """Fetch a pinned public URL, refusing anything that is not HTTP(S)."""
    scheme = urlparse(url).scheme
    if scheme not in ("http", "https"):
        raise ValueError(f"refusing non-HTTP URL scheme {scheme!r}")
    request = urllib.request.Request(url, headers={"User-Agent": USER_AGENT})
    with urllib.request.urlopen(request, timeout=timeout) as response:
        return response.read()


def fetch_json(url: str) -> dict:
    """Fetch and decode a JSON document."""
    return json.loads(open_url(url).decode("utf-8"))


def fetch_document_page(type_code: str, start: str, end: str) -> list[Record]:
    """Collect every document of one type in a closed publication-date window."""
    records: list[Record] = []
    page = 1
    while True:
        query = {
            "per_page": "1000",
            "page": str(page),
            "order": "oldest",
            "conditions[type][]": type_code,
            "conditions[publication_date][gte]": start,
            "conditions[publication_date][lte]": end,
            "fields[]": [
                "document_number",
                "type",
                "title",
                "publication_date",
                "raw_text_url",
                "agencies",
            ],
        }
        payload = fetch_json(f"{API}?{urllib.parse.urlencode(query, doseq=True)}")
        records.extend(to_record(item) for item in payload["results"])
        if page >= int(payload["total_pages"]):
            return records
        page += 1


def slug_of(agency: dict) -> str:
    """Normalized agency slug, falling back to the name when the API omits one."""
    slug = agency.get("slug")
    if slug:
        return slug
    name = agency.get("name") or agency.get("raw_name") or "unknown-agency"
    return re.sub(r"[^a-z0-9]+", "-", name.lower()).strip("-") or "unknown-agency"


def to_record(item: dict) -> Record:
    """Convert one API result into a selection record."""
    agencies = item.get("agencies") or []
    top = [agency for agency in agencies if agency.get("parent_id") is None] or agencies
    leaf = agencies[-1] if agencies else {"name": "Unknown Agency"}
    family = top[0] if top else {"name": "Unknown Agency"}
    folded_type = TYPES[item["type"]][1] if item["type"] in TYPES else UNKNOWN_TYPE[2]
    type_code = TYPES[item["type"]][0] if item["type"] in TYPES else UNKNOWN_TYPE[1]
    return Record(
        document_number=item["document_number"],
        type_code=type_code,
        folded_type=folded_type,
        family=slug_of(family),
        family_name=family.get("name") or family.get("raw_name") or "Unknown Agency",
        subfamily=slug_of(leaf),
        publication_date=item["publication_date"],
        title=item["title"] or "",
        source_url=item["raw_text_url"],
        agencies=tuple(slug_of(agency) for agency in agencies),
    )


def normalize_raw_text(raw: bytes) -> tuple[str, dict]:
    """Return document text plus the factual record of what stripping removed."""
    text = raw.decode("utf-8", "replace")
    inner = re.search(r"(?is)<body>\s*<pre>", text)
    if inner:
        text = text[inner.end() :]
    text = re.split(r"(?i)</pre>", text, maxsplit=1)[0]
    text = html.unescape(re.sub(r"<[^>]+>", "", text))
    removed: list[str] = []
    kept: list[str] = []
    dropping_action = False
    for line in text.splitlines():
        stripped = line.strip()
        if re.match(
            r"(?i)^\[\s*(proposed rules?|rules?|notices?|presidential documents?)\s*\]$",
            stripped,
        ):
            removed.append(stripped)
            continue
        if re.match(r"(?i)^\[\s*federal register volume", stripped):
            removed.append(stripped)
            continue
        if re.match(r"(?i)^from the federal register online", stripped):
            removed.append(stripped)
            continue
        if re.match(r"(?i)^\[\s*fr doc no:", stripped):
            removed.append(stripped)
            continue
        if re.match(r"(?i)^action:", stripped):
            dropping_action = True
            removed.append(stripped)
            continue
        if dropping_action:
            if stripped:
                removed.append(stripped)
                continue
            dropping_action = False
        kept.append(line)
    normalized = re.sub(r"\n{3,}", "\n\n", "\n".join(kept)).strip()
    normalized = clamp_utf8(normalized, MAX_TEXT_BYTES)
    return normalized, {"removed_metadata_lines": removed[:12]}


def clamp_utf8(text: str, limit: int) -> str:
    """Keep at most ``limit`` UTF-8 bytes without splitting a character."""
    encoded = text.encode("utf-8")
    if len(encoded) <= limit:
        return text
    return encoded[:limit].decode("utf-8", "ignore").rstrip()


def signature_of(members: list[Record]) -> tuple[str, ...]:
    """The set of document types a family produces, used to stratify splits."""
    return tuple(sorted({record.folded_type for record in members}))


def assign_splits(families: dict[str, list[Record]]) -> dict[str, str]:
    """Deterministic family-stable split assignment, stratified by class signature.

    Families grouping the same set of document types are distributed across splits so every
    split observes every known class. Ties break on the family digest, so the assignment is
    reproducible without a random seed and independent of API result order.
    """
    buckets: dict[tuple[str, ...], list[str]] = defaultdict(list)
    for family, members in families.items():
        buckets[signature_of(members)].append(family)
    slots = [split for split in SPLIT_ORDER for _ in range(SPLIT_WEIGHTS[split])]
    assignment: dict[str, str] = {}
    offset = 0
    for signature in sorted(buckets):
        ordered = sorted(buckets[signature], key=lambda family: hashlib.sha256(family.encode()).hexdigest())
        if signature == (UNKNOWN_TYPE[2],):
            # The unknown-class cohort is reserved for held-out so rejection is measured on
            # enough examples; these documents never enter a known-class family.
            for family in ordered:
                assignment[family] = "held_out"
            continue
        for index, family in enumerate(ordered):
            assignment[family] = slots[(offset + index) % len(slots)]
        offset += len(ordered)
    return assignment


def select(families: dict[str, list[Record]], caps: dict[str, int]) -> dict[str, list[Record]]:
    """Cap documents per family and per class, keeping a deterministic order."""
    selected: dict[str, list[Record]] = defaultdict(list)
    unknown = sorted(
        (record for members in families.values() for record in members if record.folded_type == UNKNOWN_TYPE[2]),
        key=lambda record: record.document_number,
    )[: caps[UNKNOWN_TYPE[2]]]
    for record in unknown:
        selected[record.family].append(record)
    per_type: Counter[str] = Counter()
    for family in sorted(families):
        members = sorted(
            (record for record in families[family] if record.folded_type != UNKNOWN_TYPE[2]),
            key=lambda record: record.document_number,
        )
        kept: list[Record] = []
        for record in members:
            if len(kept) >= MAX_PER_FAMILY:
                break
            if per_type[record.folded_type] >= caps[record.folded_type]:
                continue
            per_type[record.folded_type] += 1
            kept.append(record)
        selected[family].extend(kept)
    return {family: members for family, members in selected.items() if members}


def freeze_documents(
    chosen: dict[str, list[Record]], assignment: dict[str, str], out: Path
) -> tuple[list[dict], list[dict]]:
    """Fetch, normalize and pin every selected document."""
    out.mkdir(parents=True, exist_ok=True)
    documents = []
    failures = []
    for family in sorted(chosen):
        split = assignment[family]
        for record in chosen[family]:
            try:
                raw = open_url(record.source_url)
            except (urllib.error.URLError, TimeoutError) as error:  # pragma: no cover - network
                failures.append({"document_number": record.document_number, "error": str(error)})
                continue
            text, strip_record = normalize_raw_text(raw)
            if len(text) < 400:
                failures.append({"document_number": record.document_number, "error": "text too short after stripping"})
                continue
            known = record.type_code in {code for code, _ in TYPES.values()}
            document_id = f"fr-{record.document_number}"
            (out / f"{document_id}.txt").write_text(text, encoding="utf-8")
            documents.append(
                {
                    "id": document_id,
                    "family": record.family,
                    "subfamily": record.subfamily,
                    "split": split,
                    "type": record.folded_type,
                    "type_code": record.type_code,
                    "known_class": known,
                    "document_number": record.document_number,
                    "publication_date": record.publication_date,
                    "agency": record.family_name,
                    "source_url": record.source_url,
                    "source_sha256": hashlib.sha256(raw).hexdigest(),
                    "text_sha256": hashlib.sha256(text.encode()).hexdigest(),
                    "text_bytes": len(text.encode()),
                    "removed_metadata_lines": strip_record["removed_metadata_lines"],
                }
            )
            time.sleep(0.05)
    documents.sort(key=lambda document: document["id"])
    return documents, failures


def build_manifest(documents: list[dict], failures: list[dict], args: argparse.Namespace) -> dict:
    """Assemble the committed manifest with its canonical digest."""
    counts = Counter(
        (document["split"], document["type"] if document["known_class"] else "unknown") for document in documents
    )
    manifest = {
        "schema": 1,
        "purpose": "Held-out document-type classification corpus for CPU ONNX candidate qualification",
        "source": {
            "name": "U.S. Federal Register",
            "publisher": "Office of the Federal Register / U.S. Government Publishing Office",
            "api": API,
            "license": "US-PD (17 U.S.C. 105)",
            "license_url": "https://www.archives.gov/global-pages/privacy.html",
            "accessed": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
            "value": "The document type is published metadata from the Office of the Federal Register",
        },
        "selection": {
            "publication_date_gte": args.start,
            "publication_date_lte": args.end,
            "type_codes": [code for code, _ in TYPES.values()] + [UNKNOWN_TYPE[1]],
            "known_labels": [label for _, label in TYPES.values()],
            "unknown_class": UNKNOWN_TYPE[2],
            "cap_per_known_class": args.cap,
            "cap_for_unknown_class": args.unknown_cap,
            "max_documents_per_family": MAX_PER_FAMILY,
            "family": "top-level issuing agency slug; one family stays in one split",
            "split_rule": "deterministic weighted round-robin over class-signature buckets, ties on family SHA-256",
            "text_rule": "HTML wrapper removed; bracketed type banner, ACTION: block, FR volume/online/FR-doc lines removed; first 60000 UTF-8 bytes kept",
        },
        "counts": {
            "documents": len(documents),
            "families": len({document["family"] for document in documents}),
            "by_split_and_class": {f"{split}/{folded}": count for (split, folded), count in sorted(counts.items())},
            "failures": len(failures),
        },
        "documents": documents,
        "failures": failures,
    }
    manifest["digest"] = hashlib.sha256(
        json.dumps(manifest["documents"], sort_keys=True, separators=(",", ":")).encode()
    ).hexdigest()
    return manifest


def main() -> int:
    """Acquire, freeze and write the corpus manifest."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--start", required=True, help="publication date lower bound (inclusive)")
    parser.add_argument("--end", required=True, help="publication date upper bound (inclusive)")
    parser.add_argument("--cap", type=int, default=120, help="maximum documents per known class")
    parser.add_argument("--unknown-cap", type=int, default=48, help="maximum unknown-class documents")
    parser.add_argument("--out", required=True, type=Path, help="corpus directory for the text cache")
    parser.add_argument("--manifest", required=True, type=Path, help="manifest JSON to write")
    args = parser.parse_args()

    caps = {folded: args.cap for _, folded in TYPES.values()}
    caps[UNKNOWN_TYPE[2]] = args.unknown_cap
    records: list[Record] = []
    for type_code in [code for code, _ in TYPES.values()] + [UNKNOWN_TYPE[1]]:
        fetched = fetch_document_page(type_code, args.start, args.end)
        sys.stdout.write(f"{type_code}: {len(fetched)} documents in window\n")
        records.extend(fetched)
        time.sleep(1)
    families: dict[str, list[Record]] = defaultdict(list)
    for record in records:
        families[record.family].append(record)
    chosen = select(families, caps)
    assignment = assign_splits(chosen)
    documents, failures = freeze_documents(chosen, assignment, args.out)
    manifest = build_manifest(documents, failures, args)
    args.manifest.parent.mkdir(parents=True, exist_ok=True)
    args.manifest.write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")
    sys.stdout.write(json.dumps(manifest["counts"], indent=2) + "\n")
    if failures:
        sys.stderr.write(f"{len(failures)} documents failed and are recorded in the manifest\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
