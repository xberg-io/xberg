#!/usr/bin/env python3
# The only URLs this tool opens are the pinned public API and image endpoints, and ``open_url``
# rejects every scheme other than http/https before the request is built. ~keep
# ruff: noqa: S310
"""Acquire the held-out image-captioning corpus for CPU captioner qualification.

Source: COCO 2017 val2017 captions and images (cocodataset.org). The annotation archive is
pinned by SHA-256; each selected image is pinned by its COCO file name and the SHA-256 of the
exact JPEG bytes fetched. Only reference captions and digests are committed: the images are
Flickr-hosted and are fetched for evaluation only, never redistributed.

COCO is used because its terms of use permit evaluation use of the images and publish the
annotations under CC BY 4.0. The annotation file itself carries no license field, so the
audit cites cocodataset.org/#termsofuse and records it as a claim about the upstream
distribution, not as a relicense of the photographs.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import sys
import time
import urllib.request
from pathlib import Path
from urllib.parse import urlparse

ANNOTATIONS = "http://images.cocodataset.org/annotations/annotations_trainval2017.zip"
IMAGE_BASE = "http://images.cocodataset.org/val2017"
USER_AGENT = "xberg-qualification/1.0 (model qualification; xberg-io)"

STOPWORDS = frozenset(
    {
        "a",
        "an",
        "and",
        "are",
        "as",
        "at",
        "be",
        "been",
        "being",
        "by",
        "for",
        "from",
        "had",
        "has",
        "have",
        "he",
        "her",
        "him",
        "his",
        "how",
        "i",
        "if",
        "in",
        "into",
        "is",
        "it",
        "its",
        "just",
        "like",
        "me",
        "my",
        "no",
        "not",
        "of",
        "on",
        "or",
        "our",
        "out",
        "over",
        "she",
        "so",
        "some",
        "than",
        "that",
        "the",
        "their",
        "them",
        "then",
        "there",
        "these",
        "they",
        "this",
        "those",
        "to",
        "too",
        "up",
        "us",
        "was",
        "we",
        "were",
        "what",
        "when",
        "where",
        "which",
        "while",
        "who",
        "why",
        "will",
        "with",
        "you",
        "your",
    }
)


def open_url(url: str, timeout: int = 120) -> bytes:
    """Fetch a pinned public URL, refusing anything that is not HTTP(S)."""
    scheme = urlparse(url).scheme
    if scheme not in ("http", "https"):
        raise ValueError(f"refusing non-HTTP URL scheme {scheme!r}")
    request = urllib.request.Request(url, headers={"User-Agent": USER_AGENT})
    with urllib.request.urlopen(request, timeout=timeout) as response:
        return response.read()


def sha256_file(path: Path) -> str:
    """SHA-256 of a file, streamed."""
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for block in iter(lambda: handle.read(1 << 20), b""):
            digest.update(block)
    return digest.hexdigest()


def content_words(caption: str) -> set[str]:
    """Lowercase content words used as the omission proxy."""
    words = re.findall(r"[a-z]+", caption.lower())
    return {word for word in words if len(word) > 2 and word not in STOPWORDS}


def numbers(caption: str) -> set[str]:
    """Numeric tokens in a caption."""
    return set(re.findall(r"\b\d+(?:\.\d+)?\b", caption))


def select_images(images: list[dict], by_image: dict[int, list[str]], count: int) -> list[dict]:
    """Deterministic stratified stride selection over sorted val2017 ids."""

    def has_numbers(image: dict) -> bool:
        return any(numbers(caption) for caption in by_image.get(image["id"], []))

    numbered = [image for image in images if has_numbers(image)]
    numbered_ids = {image["id"] for image in numbered}
    plain = [image for image in images if image["id"] not in numbered_ids]

    def stride_take(pool: list[dict], wanted: int) -> list[dict]:
        if wanted <= 0:
            return []
        stride = max(1, len(pool) // wanted)
        return pool[::stride][:wanted]

    wanted_numbered = count // 3
    selected = stride_take(numbered, wanted_numbered) + stride_take(plain, count - wanted_numbered)
    selected.sort(key=lambda image: image["id"])
    if len(selected) != count:
        raise SystemExit(f"selected {len(selected)} images, expected {count}")
    return selected


def freeze_image(image: dict, captions: list[str], images_dir: Path, numbered: set[int]) -> dict:
    """Download one image if needed and pin it with its reference captions."""
    target = images_dir / image["file_name"]
    if not target.exists():
        target.write_bytes(open_url(f"{IMAGE_BASE}/{image['file_name']}"))
        time.sleep(0.1)
    if len(captions) < 5:
        raise SystemExit(f"image {image['file_name']} has {len(captions)} captions, expected 5")
    return {
        "id": f"coco-val2017-{image['id']:012d}",
        "image_file": image["file_name"],
        "image_url": f"{IMAGE_BASE}/{image['file_name']}",
        "image_sha256": sha256_file(target),
        "image_bytes": target.stat().st_size,
        "width": image["width"],
        "height": image["height"],
        "split": "held_out",
        "stratum": "numbers" if image["id"] in numbered else "background",
        "reference_captions": captions,
        "reference_numbers": sorted(set().union(*(numbers(caption) for caption in captions))),
        "reference_terms": sorted(set().union(*(content_words(caption) for caption in captions))),
    }


def build_manifest(records: list[dict], args: argparse.Namespace) -> dict:
    """Assemble the committed caption manifest with its canonical digest."""
    manifest = {
        "schema": 1,
        "purpose": "Held-out image-captioning corpus for CPU ONNX candidate qualification",
        "source": {
            "name": "COCO 2017 val2017 captions",
            "publisher": "COCO Consortium",
            "citation": "Lin et al., Microsoft COCO: Common Objects in Context (ECCV 2014)",
            "annotations_url": ANNOTATIONS,
            "annotations_sha256": sha256_file(args.annotations),
            "annotations_info": json.loads(args.annotations.read_text(encoding="utf-8")).get("info", {}),
            "licenses": [
                {
                    "applies_to": "annotations",
                    "license": "CC-BY-4.0",
                    "url": "https://cocodataset.org/#termsofuse",
                    "note": "Published by the COCO Consortium; the archive carries no license field, so the audit records the upstream claim",
                },
                {
                    "applies_to": "images",
                    "license": "Flickr-hosted, evaluation-only",
                    "url": "https://cocodataset.org/#termsofuse",
                    "note": "Not redistributed and not committed; fetched per image from the pinned URL for evaluation",
                },
            ],
            "accessed": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
        },
        "selection": {
            "rule": "deterministic stride over sorted ids, stratified: count//3 number-bearing images and the rest background",
            "count": args.count,
            "split": "held_out",
            "note": "Reference captions are five per image; nothing here is a document-caption ground truth with visible-fact annotations, so fact precision needs human raters",
        },
        "counts": {
            "images": len(records),
            "reference_captions": sum(len(record["reference_captions"]) for record in records),
            "images_with_reference_numbers": sum(1 for record in records if record["reference_numbers"]),
        },
        "images": records,
    }
    manifest["digest"] = hashlib.sha256(
        json.dumps(manifest["images"], sort_keys=True, separators=(",", ":")).encode()
    ).hexdigest()
    return manifest


def main() -> int:
    """Acquire, freeze and write the caption manifest."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--annotations", type=Path, required=True, help="extracted captions_val2017.json")
    parser.add_argument("--images", type=Path, required=True, help="image cache directory")
    parser.add_argument("--count", type=int, default=30)
    parser.add_argument("--manifest", type=Path, required=True)
    args = parser.parse_args()

    payload = json.loads(args.annotations.read_text(encoding="utf-8"))
    by_image: dict[int, list[str]] = {}
    for annotation in payload["annotations"]:
        by_image.setdefault(annotation["image_id"], []).append(annotation["caption"].strip())
    images = sorted(payload["images"], key=lambda image: image["id"])
    selected = select_images(images, by_image, args.count)
    numbered = {image["id"] for image in selected if any(numbers(caption) for caption in by_image.get(image["id"], []))}
    args.images.mkdir(parents=True, exist_ok=True)
    records = [freeze_image(image, sorted(by_image.get(image["id"], [])), args.images, numbered) for image in selected]
    manifest = build_manifest(records, args)
    args.manifest.parent.mkdir(parents=True, exist_ok=True)
    args.manifest.write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")
    sys.stdout.write(json.dumps(manifest["counts"], indent=2) + "\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
