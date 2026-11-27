#!/usr/bin/env python3
"""Build a stratified synthetic-scan OCR corpus from native PDFs in test_documents.

Every page comes from a native PDF whose text layer passes automatic reliability checks. The page
is rendered with poppler and, for the degraded variants, run through a seeded ImageMagick scan
degradation. Ground truth is poppler `pdftotext` in its default reading-order mode, an extractor
independent of xberg; no xberg output is ever used.

    python3 -I synth_ocr_corpus.py inventory --corpus ../../../test_documents --out OUT
    python3 -I synth_ocr_corpus.py build     --corpus ../../../test_documents --out OUT
    python3 -I synth_ocr_corpus.py verify    --out OUT --samples 10

Stages:
- inventory: per sampled page, extract text, detect language/script, classify layout and apply the
  reliability checks; writes OUT/inventory.jsonl (cached; delete it to recompute).
- build: stratified selection, rendering, degradation, ground truth, fixtures, cohorts and
  OUT/manifest.json with tool versions, parameters, exclusion counts and stratum counts.
- verify: runs plain `tesseract` on clean 300 dpi samples and reports token F1 and reading-order
  agreement against the default, `-layout` and `-raw` pdftotext modes.

Deterministic for a fixed `--seed`, corpus and tool versions (recorded in the manifest).
Requires poppler (pdftotext, pdftoppm, pdfimages, pdfinfo), ImageMagick 7 and, for verify,
tesseract. Standard library only.
"""

from __future__ import annotations

import argparse
import bisect
import collections
import concurrent.futures
import hashlib
import html
import itertools
import json
import os
import random
import re
import shutil
import subprocess
import sys
import unicodedata
from dataclasses import dataclass, field
from pathlib import Path

DATASET = "xberg-synth-scan-v1"
GT_SOURCE = "pdf_text_layer"
DEFAULT_SEED = 20261127
DEFAULT_PER_STRATUM = 20
MAX_PAGES_PER_DOC = 40
MAX_PAGES_PER_DOC_PER_STRATUM = 4
TEXT_FLAGS: list[str] = []
SUBPROCESS_TIMEOUT_S = 120

MIN_TOKENS = 25
SPARSE_TOKENS = 120
MAX_GARBAGE_FRACTION = 0.005
MIN_ALNUM_FRACTION = 0.6
MIN_STOPWORD_RATE = 0.06
MIN_LANGUAGE_MARGIN = 1.3
MAX_SINGLE_LETTER_RATE = 0.3
MAX_MEAN_TOKEN_LEN = 15.0
MAX_ORDER_JUMP_FRACTION = 0.05
MIN_ORDER_JUMPS = 2
MAX_TOUCHING_WORD_FRACTION = 0.01
TOUCHING_GAP_PT = 0.5
MIN_INKED_WORD_FRACTION = 0.97
INK_CHECK_DPI = 100
INK_THRESHOLD = 230
MAX_IMAGE_AREA_FRACTION = 0.05
MAX_PAGE_SIDE_PT = 1500.0
MULTI_COLUMN_LINE_FRACTION = 0.4
TABLE_MIN_ROWS = 5
TABLE_MIN_CELLS = 3

EXCLUDED_PREFIXES = ("pdf_scanned/", "user_reports/", "ordinance-2197.pdf")
EXCLUDED_SUBSTRINGS = ("from-oss-fuzz",)
EXTRA_PREFIX = "sources:"
SOURCES_MANIFEST = "sources.json"

# Single-letter words are dropped: in technical text they are variables ("a", "v", "k"), which made
# code-heavy English pages score as Czech or Polish.
STOPWORDS: dict[str, frozenset[str]] = {
    lang: frozenset(word for word in words.split() if len(word) > 1)
    for lang, words in {
        "en": "the of and to in is that for it with as was on are by this be from or which an at not",
        "de": "der die und in den von zu das mit sich des auf für ist im dem nicht ein eine als auch es an",
        "fr": "le la les de des et en un une du est que pour dans qui sur par au pas plus ne se ce sont",
        "es": "el la de que y en los del se las por un una con para es al lo como más pero sus su",
        "it": "il di che e la per un in non una sono del della le si con da gli dei nel alla è",
        "pt": "de que o a do da em para com não uma os no se na por mais as dos como mas ao é",
        "nl": "de het een en van in is dat op te zijn met voor niet aan er die door als ook",
        "sv": "och att det en på är av för med till den har inte om ett var jag men från",
        "da": "og at det en på er af for med til den har ikke om et var fra de",
        "no": "og at det en på er av for med til den har ikke om et var fra de",
        "fi": "ja on ei se että oli ovat mutta kuin myös tai hän niin joka sekä kun",
        "pl": "i w na z do nie się jest że to od o jak po za przez dla oraz jego",
        "cs": "a v se na je že s z do o k to jako by pro ale od jsou nebo",
        "tr": "ve bir bu da de için ile olarak çok daha gibi olan en ama sonra kadar",
        "ro": "și de la în a cu pe că care din nu o un este pentru mai sau",
        "hu": "a az és hogy nem is egy meg van de ez csak mint már volt",
        "id": "yang dan di dari ini itu dengan untuk tidak dalam akan pada juga ke",
        "vi": "và của là có các được trong cho một những không với người này",
        "ru": "и в не на что с по как это из к о для его но от же все",  # noqa: RUF001
        "uk": "і в на що не з до та як це у за від його але є",  # noqa: RUF001
        "bg": "и в на да се от за не че с по са като е",  # noqa: RUF001
        "el": "και το της του η να σε που με για τα την από ο οι τη",  # noqa: RUF001
        "ar": "في من على إلى أن عن مع هذا التي الذي هذه كان ما لا",
        "fa": "و در به از که این را با است برای آن یک می",
        "ur": "کے میں اور کی کو سے ہے کا نے یہ پر",
        "he": "של את על לא עם זה הוא כי גם או אם היא",
    }.items()
}

TESSERACT_CODES = {
    "en": "eng",
    "de": "deu",
    "fr": "fra",
    "es": "spa",
    "it": "ita",
    "pt": "por",
    "nl": "nld",
    "sv": "swe",
    "da": "dan",
    "no": "nor",
    "fi": "fin",
    "pl": "pol",
    "cs": "ces",
    "tr": "tur",
    "ro": "ron",
    "hu": "hun",
    "id": "ind",
    "vi": "vie",
    "ru": "rus",
    "uk": "ukr",
    "bg": "bul",
    "el": "ell",
    "ar": "ara",
    "fa": "fas",
    "ur": "urd",
    "he": "heb",
    "zh-Hans": "chi_sim",
    "zh-Hant": "chi_tra",
    "ja": "jpn",
    "ko": "kor",
    "hi": "hin",
    "bn": "ben",
    "ta": "tam",
    "th": "tha",
    "ka": "kat",
    "hy": "hye",
}

SCRIPT_RANGES = (
    ("Latin", 0x0041, 0x024F),
    ("Greek", 0x0370, 0x03FF),
    ("Cyrillic", 0x0400, 0x04FF),
    ("Armenian", 0x0530, 0x058F),
    ("Hebrew", 0x0590, 0x05FF),
    ("Arabic", 0x0600, 0x06FF),
    ("Arabic", 0x0750, 0x077F),
    ("Devanagari", 0x0900, 0x097F),
    ("Bengali", 0x0980, 0x09FF),
    ("Tamil", 0x0B80, 0x0BFF),
    ("Thai", 0x0E00, 0x0E7F),
    ("Georgian", 0x10A0, 0x10FF),
    ("Hangul", 0x1100, 0x11FF),
    ("Latin", 0x1E00, 0x1EFF),
    ("Kana", 0x3040, 0x30FF),
    ("Hangul", 0x3130, 0x318F),
    ("Han", 0x3400, 0x4DBF),
    ("Han", 0x4E00, 0x9FFF),
    ("Hangul", 0xAC00, 0xD7AF),
    ("Han", 0xF900, 0xFAFF),
    ("Arabic", 0xFB50, 0xFDFF),
    ("Arabic", 0xFE70, 0xFEFF),
    ("Han", 0x20000, 0x2FA1F),
)
SCRIPT_STARTS = [start for _, start, _ in SCRIPT_RANGES]
SINGLE_SCRIPT_LANGS = {
    "Greek": "el",
    "Hebrew": "he",
    "Devanagari": "hi",
    "Bengali": "bn",
    "Tamil": "ta",
    "Thai": "th",
    "Georgian": "ka",
    "Armenian": "hy",
    "Hangul": "ko",
}
TRADITIONAL_ONLY = frozenset("們這個來時會說對為國學發與經點過還種體開關長門問間題處應從當")
SIMPLIFIED_ONLY = frozenset("们这个来时会说对为国学发与经点过还种体开关长门问间题处应从当")
UNCONTESTED_MARGIN = 99.0
# poppler emits lam-alef ligatures alef-first ("االستخراج" for "الاستخراج"): an alef followed by
# alef/hamza-alef and lam inside a word does not occur in correctly ordered Arabic.
ARABIC_LIGATURE_ORDER_RE = re.compile("\u0627[\u0627\u0623\u0625\u0622]\u0644")
PERSIAN_LETTERS = frozenset("پچژگ")
URDU_LETTERS = frozenset("ٹڈڑےں")


def pdftotext_text_flags() -> list[str]:
    """Keep end-of-line hyphens so ground truth matches the rendered image; poppler >= 24.10 dehyphenates
    by default, older releases have no switch.
    """
    usage = subprocess.run(["pdftotext", "-h"], capture_output=True, text=True, check=False)
    flags = ["-enc", "UTF-8", "-nopgbrk"]
    if "-remove-hyphens" in usage.stdout + usage.stderr:
        flags += ["-remove-hyphens", "none"]
    return flags


def run(cmd: list[str], *, binary: bool = False) -> str | bytes:
    proc = subprocess.run(cmd, capture_output=True, timeout=SUBPROCESS_TIMEOUT_S, check=True)
    return proc.stdout if binary else proc.stdout.decode("utf-8", errors="replace")


def tool_versions() -> dict[str, str]:
    def first_line(cmd: list[str]) -> str:
        proc = subprocess.run(cmd, capture_output=True, text=True, check=False)
        return (proc.stdout or proc.stderr).strip().splitlines()[0]

    versions = {
        "pdftotext": first_line(["pdftotext", "-v"]),
        "pdftoppm": first_line(["pdftoppm", "-v"]),
        "imagemagick": first_line(["magick", "-version"]),
    }
    if shutil.which("tesseract"):
        versions["tesseract"] = first_line(["tesseract", "--version"])
    return versions


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1 << 20), b""):
            digest.update(chunk)
    return digest.hexdigest()


def stable_key(seed: int, *parts: object) -> str:
    return hashlib.sha256("|".join(map(str, (seed, *parts))).encode()).hexdigest()


def seeded_rng(seed: int, *parts: object) -> random.Random:
    """Reproducible sampling and degradation parameters; nothing here is security-sensitive."""
    return random.Random(stable_key(seed, *parts))  # noqa: S311


@dataclass
class Sources:
    """Where source PDFs come from: the pinned corpus, plus an optional uncommitted directory of extra
    native PDFs whose `sources.json` records each file's URL and licence.

    Document ids are corpus-relative paths, or `sources:<path>` for the extra directory.
    """

    corpus: Path
    extra: Path | None = None
    provenance: dict[str, dict[str, str]] = field(default_factory=dict)
    shas: dict[str, str] = field(default_factory=dict)

    def resolve(self, doc: str) -> Path:
        """Filesystem path of a document id."""
        if doc.startswith(EXTRA_PREFIX):
            if self.extra is None:
                raise SystemExit(f"{doc} needs --sources")
            return self.extra / doc.removeprefix(EXTRA_PREFIX)
        return self.corpus / doc

    def origin(self, doc: str) -> dict[str, str]:
        """Provenance recorded in fixtures: URL and licence for extra PDFs, the corpus pin otherwise."""
        if doc.startswith(EXTRA_PREFIX):
            return self.provenance[doc]
        return {"origin": "xberg-io/test_documents", "pinned_in": "corpus.lock.json"}

    def documents(self) -> list[str]:
        """Corpus PDFs pinned in corpus.lock.json (minus scans, user reports and fuzz inputs), then extra
        PDFs; byte-identical copies are dropped, the first id wins.
        """
        objects = json.loads((self.corpus / "corpus.lock.json").read_text(encoding="utf-8"))["objects"]
        seen: set[str] = set()
        docs = []
        for rel in sorted(objects):
            if (
                not rel.lower().endswith(".pdf")
                or rel.startswith(EXCLUDED_PREFIXES)
                or any(part in rel for part in EXCLUDED_SUBSTRINGS)
                or objects[rel]["sha256"] in seen
                or not (self.corpus / rel).is_file()
            ):
                continue
            seen.add(objects[rel]["sha256"])
            self.shas[rel] = objects[rel]["sha256"]
            docs.append(rel)
        for doc, sha in sorted(self._extra_documents().items()):
            if sha not in seen:
                seen.add(sha)
                self.shas[doc] = sha
                docs.append(doc)
        return docs

    def _extra_documents(self) -> dict[str, str]:
        if self.extra is None:
            return {}
        manifest = self.extra / SOURCES_MANIFEST
        if not manifest.is_file():
            raise SystemExit(f"--sources {self.extra} has no {SOURCES_MANIFEST} recording URL and licence per PDF")
        entries = json.loads(manifest.read_text(encoding="utf-8"))
        pdfs = sorted(p.relative_to(self.extra).as_posix() for p in self.extra.rglob("*.pdf") if p.is_file())
        missing = [rel for rel in pdfs if not _valid_provenance(entries.get(rel))]
        if missing:
            raise SystemExit(f"{SOURCES_MANIFEST} lacks a url and license for: {', '.join(missing)}")
        for rel in pdfs:
            entry = entries[rel]
            self.provenance[EXTRA_PREFIX + rel] = {
                key: str(entry[key]) for key in ("url", "license", "note") if key in entry
            }
        return {EXTRA_PREFIX + rel: sha256_file(self.extra / rel) for rel in pdfs}

    def fingerprint(self, docs: list[str]) -> str:
        """Hash of the document set; a changed set invalidates the cached inventory."""
        return hashlib.sha256(json.dumps([(doc, self.shas[doc]) for doc in docs]).encode()).hexdigest()


def _valid_provenance(entry: object) -> bool:
    return isinstance(entry, dict) and all(isinstance(entry.get(key), str) and entry[key] for key in ("url", "license"))


# Tokenization mirrors benchmark-harness src/quality/mod.rs `tokenize` closely enough for
# corpus checks and the verify spot check; the harness itself stays authoritative for scores.
def is_cjk(char: str) -> bool:
    code = ord(char)
    return any(
        start <= code <= end
        for start, end in (
            (0x1100, 0x11FF),
            (0x2E80, 0x2EFF),
            (0x3000, 0x303F),
            (0x3040, 0x30FF),
            (0x3130, 0x318F),
            (0x31F0, 0x31FF),
            (0x3400, 0x4DBF),
            (0x4E00, 0x9FFF),
            (0xA960, 0xA97F),
            (0xAC00, 0xD7FF),
            (0xF900, 0xFAFF),
            (0xFF66, 0xFF9F),
            (0x20000, 0x2FA1F),
        )
    )


def tokenize(text: str) -> list[str]:
    text = unicodedata.normalize("NFKC", text).lower().replace("|", " ")
    tokens: list[str] = []
    for word in text.split():
        kept = "".join(c for c in word if c.isalnum() or c in ".,").strip(".,")
        if not kept:
            continue
        run_chars: list[str] = []
        for group_is_cjk, group in _cjk_groups(kept):
            if group_is_cjk:
                run_chars.extend(group)
            elif set(group) <= {".", ","}:
                continue
            else:
                _flush_bigrams(tokens, run_chars)
                tokens.append(group)
        _flush_bigrams(tokens, run_chars)
    return tokens


def _cjk_groups(word: str) -> list[tuple[bool, str]]:
    groups: list[tuple[bool, str]] = []
    for char in word:
        flag = is_cjk(char)
        if groups and groups[-1][0] == flag:
            groups[-1] = (flag, groups[-1][1] + char)
        else:
            groups.append((flag, char))
    return groups


def _flush_bigrams(tokens: list[str], run_chars: list[str]) -> None:
    if len(run_chars) == 1:
        tokens.append(run_chars[0])
    else:
        tokens.extend(a + b for a, b in itertools.pairwise(run_chars))
    run_chars.clear()


def token_f1(extracted: str, truth: str) -> float:
    ext, gt = collections.Counter(tokenize(extracted)), collections.Counter(tokenize(truth))
    overlap = sum((ext & gt).values())
    if not overlap:
        return 0.0
    precision, recall = overlap / sum(ext.values()), overlap / sum(gt.values())
    return 2 * precision * recall / (precision + recall)


def reading_order(extracted: str, truth: str) -> float | None:
    """Anchor-based LIS, as in the harness `reading_order_score` (None below 8 anchors)."""
    ext, gt = tokenize(extracted), tokenize(truth)
    ext_counts, gt_counts = collections.Counter(ext), collections.Counter(gt)
    ext_index = {tok: i for i, tok in enumerate(ext) if ext_counts[tok] == 1}
    sequence = [ext_index[tok] for tok in gt if gt_counts[tok] == 1 and tok in ext_index]
    if len(sequence) < 8:
        return None
    tails: list[int] = []
    for value in sequence:
        pos = bisect.bisect_left(tails, value)
        tails[pos : pos + 1] = [value]
    return len(tails) / len(sequence)


def script_of(char: str) -> str | None:
    code = ord(char)
    idx = bisect.bisect_right(SCRIPT_STARTS, code) - 1
    if idx >= 0:
        name, start, end = SCRIPT_RANGES[idx]
        if start <= code <= end:
            return name
    return None


def detect_language(text: str) -> dict[str, object]:
    counts = collections.Counter(s for s in map(script_of, text) if s)
    total = sum(counts.values())
    if not total:
        return {
            "lang": None,
            "script": None,
            "script_share": 0.0,
            "latin_share": 0.0,
            "stopword_rate": 0.0,
            "language_margin": 0.0,
        }
    script, top = counts.most_common(1)[0]
    if script == "Han" and counts["Kana"] > 0.05 * total:
        script = "Kana"
    lang, stop_rate, margin = _language_for_script(script, text)
    script_count = top if script != "Kana" else counts["Kana"] + counts["Han"]
    return {
        "lang": lang,
        "script": "Japanese" if script == "Kana" else script,
        "script_share": round(script_count / total, 3),
        "latin_share": round(counts["Latin"] / total, 3),
        "stopword_rate": round(stop_rate, 3),
        "language_margin": round(margin, 2),
    }


def _language_for_script(script: str, text: str) -> tuple[str | None, float, float]:
    """Language, its stopword rate, and the ratio of that rate to the runner-up candidate's."""
    words = [w.strip(".,;:!?()[]\"'«»„“”") for w in unicodedata.normalize("NFKC", text).lower().split()]
    words = [w for w in words if w]
    if script == "Kana":
        return "ja", 1.0, UNCONTESTED_MARGIN
    if script == "Han":
        trad = sum(c in TRADITIONAL_ONLY for c in text)
        simp = sum(c in SIMPLIFIED_ONLY for c in text)
        return ("zh-Hant" if trad > simp else "zh-Hans"), 1.0, UNCONTESTED_MARGIN
    if script == "Arabic":
        candidates = ["ar", "fa", "ur"]
        if sum(c in URDU_LETTERS for c in text) > 5:
            candidates = ["ur"]
        elif sum(c in PERSIAN_LETTERS for c in text) > 5:
            candidates = ["fa"]
    elif script == "Latin":
        candidates = [
            "en",
            "de",
            "fr",
            "es",
            "it",
            "pt",
            "nl",
            "sv",
            "da",
            "fi",
            "pl",
            "cs",
            "tr",
            "ro",
            "hu",
            "id",
            "vi",
        ]
    elif script == "Cyrillic":
        candidates = ["ru", "uk", "bg"]
    else:
        lang = SINGLE_SCRIPT_LANGS.get(script)
        if lang in STOPWORDS:
            return lang, _stop_rate(words, STOPWORDS[lang]), UNCONTESTED_MARGIN
        return lang, 1.0, UNCONTESTED_MARGIN
    rates = {lang: _stop_rate(words, STOPWORDS[lang]) for lang in candidates}
    ranked = sorted(candidates, key=lambda lang: (-rates[lang], candidates.index(lang)))
    if len(ranked) == 1:
        return ranked[0], rates[ranked[0]], UNCONTESTED_MARGIN
    return ranked[0], rates[ranked[0]], rates[ranked[0]] / max(rates[ranked[1]], 0.01)


def _stop_rate(words: list[str], stopwords: frozenset[str]) -> float:
    return sum(w in stopwords for w in words) / len(words) if words else 0.0


@dataclass
class Line:
    """One pdftotext -bbox-layout line: its box in PDF points and its words with their boxes."""

    x0: float
    y0: float
    x1: float
    y1: float
    words: list[tuple[float, float, float, float, str]]


WORD_RE = re.compile(r'<word xMin="([\d.]+)" yMin="([\d.]+)" xMax="([\d.]+)" yMax="([\d.]+)">(.*?)</word>', re.DOTALL)
LINE_RE = re.compile(r'<line xMin="([\d.]+)" yMin="([\d.]+)" xMax="([\d.]+)" yMax="([\d.]+)">(.*?)</line>', re.DOTALL)
PAGE_RE = re.compile(r'<page width="([\d.]+)" height="([\d.]+)">')


def parse_bbox_layout(xhtml: str) -> tuple[float, float, list[Line]]:
    page = PAGE_RE.search(xhtml)
    width, height = (float(page.group(1)), float(page.group(2))) if page else (0.0, 0.0)
    lines = []
    for match in LINE_RE.finditer(xhtml):
        words = [
            (float(a), float(b), float(c), float(d), html.unescape(t))
            for a, b, c, d, t in WORD_RE.findall(match.group(5))
        ]
        if words:
            lines.append(Line(*(float(match.group(i)) for i in range(1, 5)), words))
    return width, height, lines


def touching_words(lines: list[Line]) -> int:
    """Adjacent alphanumeric words with no gap: one word split by a font change, as small caps are
    ("A" + "DVANCED"), which pdftotext emits as two words while the image shows one.
    """
    count = 0
    for line in lines:
        for left, right in itertools.pairwise(line.words):
            if right[0] - left[2] < TOUCHING_GAP_PT and left[4][-1:].isalnum() and right[4][:1].isalnum():
                count += 1
    return count


def order_jumps(lines: list[Line]) -> int:
    """Backward jumps inside one column: the next line starts well above the previous one while
    sharing its horizontal span. Correct column-wise order only jumps up when changing column.
    """
    jumps = 0
    for prev, nxt in itertools.pairwise(lines):
        height = max(prev.y1 - prev.y0, 1.0)
        overlap = min(prev.x1, nxt.x1) - max(prev.x0, nxt.x0)
        narrower = max(min(prev.x1 - prev.x0, nxt.x1 - nxt.x0), 1.0)
        if nxt.y0 < prev.y0 - 2 * height and overlap > 0.3 * narrower:
            jumps += 1
    return jumps


def classify_layout(width: float, lines: list[Line], token_count: int) -> tuple[str, dict[str, float]]:
    side_by_side = 0
    long_lines = [ln for ln in lines if ln.x1 - ln.x0 >= 0.25 * width]
    for line in long_lines:
        if any(
            other is not line
            and min(line.y1, other.y1) - max(line.y0, other.y0) > 0.5 * (line.y1 - line.y0)
            and (other.x0 > line.x1 + 5 or other.x1 < line.x0 - 5)
            for other in long_lines
        ):
            side_by_side += 1
    column_fraction = side_by_side / len(long_lines) if long_lines else 0.0
    table_rows = _table_rows(lines)
    stats = {"column_line_fraction": round(column_fraction, 3), "table_rows": table_rows}
    if table_rows >= TABLE_MIN_ROWS:
        return "table", stats
    if column_fraction >= MULTI_COLUMN_LINE_FRACTION:
        return "multi_column", stats
    if token_count < SPARSE_TOKENS:
        return "sparse", stats
    return "single_column", stats


def _table_rows(lines: list[Line]) -> int:
    """Rows of at least TABLE_MIN_CELLS horizontally separated cells sharing a baseline."""
    rows: dict[int, list[Line]] = collections.defaultdict(list)
    for line in lines:
        rows[round(line.y1 / 3)].append(line)
    count = 0
    for row in rows.values():
        cells = sorted(row, key=lambda ln: ln.x0)
        separated = 1 + sum(1 for a, b in itertools.pairwise(cells) if b.x0 - a.x1 > 8)
        if separated >= TABLE_MIN_CELLS:
            count += 1
    return count


def text_garbage_fraction(text: str) -> float:
    chars = [c for c in text if not c.isspace()]
    if not chars:
        return 1.0
    bad = sum(1 for c in chars if c == "�" or 0xE000 <= ord(c) <= 0xF8FF or unicodedata.category(c) == "Cc")
    return (bad + 10 * text.count("(cid:")) / len(chars)


def inked_word_fraction(pdf: Path, page: int, lines: list[Line]) -> float:
    pgm = run(["pdftoppm", "-f", str(page), "-l", str(page), "-r", str(INK_CHECK_DPI), "-gray", str(pdf)], binary=True)
    header = re.match(rb"P5\s+(\d+)\s+(\d+)\s+(\d+)\s", pgm)
    if not header:
        return 0.0
    width, height = int(header.group(1)), int(header.group(2))
    pixels = memoryview(pgm)[header.end() :]
    scale = INK_CHECK_DPI / 72.0
    words = [w for line in lines for w in line.words]
    inked = 0
    for x0, y0, x1, y1, _ in words:
        left, right = max(int(x0 * scale), 0), min(int(x1 * scale) + 1, width)
        top, bottom = max(int(y0 * scale), 0), min(int(y1 * scale) + 1, height)
        if (
            left < right
            and top < bottom
            and any(min(pixels[row * width + left : row * width + right]) < INK_THRESHOLD for row in range(top, bottom))
        ):
            inked += 1
    return inked / len(words) if words else 0.0


def image_area_by_page(pdf: Path) -> dict[int, float]:
    areas: dict[int, float] = collections.defaultdict(float)
    try:
        listing = run(["pdfimages", "-list", str(pdf)])
    except (subprocess.SubprocessError, OSError):
        return areas
    for row in listing.splitlines()[2:]:
        cols = row.split()
        if len(cols) < 14 or cols[2] not in ("image", "mask", "stencil"):
            continue
        try:
            page, w, h, xppi, yppi = int(cols[0]), int(cols[3]), int(cols[4]), float(cols[12]), float(cols[13])
        except ValueError:
            continue
        if xppi > 0 and yppi > 0:
            areas[page] += (w / xppi * 72) * (h / yppi * 72)
    return areas


def page_count(pdf: Path) -> int:
    info = run(["pdfinfo", str(pdf)])
    match = re.search(r"^Pages:\s+(\d+)", info, re.MULTILINE)
    return int(match.group(1)) if match else 0


def sampled_pages(rel: str, pages: int, seed: int) -> list[int]:
    if pages <= MAX_PAGES_PER_DOC:
        return list(range(1, pages + 1))
    return sorted(seeded_rng(seed, rel).sample(range(1, pages + 1), MAX_PAGES_PER_DOC))


def inventory_document(sources: Sources, rel: str, seed: int) -> list[dict[str, object]]:
    pdf = sources.resolve(rel)
    try:
        pages = page_count(pdf)
    except (subprocess.SubprocessError, OSError) as err:
        return [{"doc": rel, "page": 0, "pages": 0, "excluded": [f"pdfinfo_failed:{type(err).__name__}"]}]
    areas = image_area_by_page(pdf)
    rows = []
    for page in sampled_pages(rel, pages, seed):
        try:
            rows.append(inventory_page(pdf, rel, page, pages, areas.get(page, 0.0)))
        except (subprocess.SubprocessError, OSError) as err:
            rows.append(
                {"doc": rel, "page": page, "pages": pages, "excluded": [f"extract_failed:{type(err).__name__}"]}
            )
    return rows


def inventory_page(pdf: Path, rel: str, page: int, pages: int, image_area: float) -> dict[str, object]:
    span = ["-f", str(page), "-l", str(page)]
    text = run(["pdftotext", *span, *TEXT_FLAGS, str(pdf), "-"])
    width, height, lines = parse_bbox_layout(run(["pdftotext", *span, "-enc", "UTF-8", "-bbox-layout", str(pdf), "-"]))
    tokens = tokenize(text)
    lang = detect_language(text)
    words = text.split()
    letters = [c for c in text if not c.isspace()]
    row: dict[str, object] = {
        "doc": rel,
        "page": page,
        "pages": pages,
        "width_pt": width,
        "height_pt": height,
        "tokens": len(tokens),
        "lines": len(lines),
        **lang,
        "garbage_fraction": round(text_garbage_fraction(text), 4),
        "alnum_fraction": round(sum(c.isalnum() for c in letters) / len(letters), 3) if letters else 0.0,
        "single_letter_rate": round(sum(len(w) == 1 and w.isalpha() for w in words) / len(words), 3) if words else 0.0,
        "mean_token_len": round(sum(map(len, words)) / len(words), 2) if words else 0.0,
        "order_jumps": order_jumps(lines),
        "touching_words": touching_words(lines),
        "arabic_ligature_errors": len(ARABIC_LIGATURE_ORDER_RE.findall(text)),
        "words": sum(len(line.words) for line in lines),
        "image_area_fraction": round(image_area / (width * height), 3) if width and height else 0.0,
    }
    row["layout"], layout_stats = classify_layout(width, lines, len(tokens))
    row.update(layout_stats)
    row["excluded"] = exclusion_reasons(row)
    if not row["excluded"]:
        row["inked_word_fraction"] = round(inked_word_fraction(pdf, page, lines), 3)
        if row["inked_word_fraction"] < MIN_INKED_WORD_FRACTION:
            row["excluded"] = ["invisible_text"]
    return row


def exclusion_reasons(row: dict[str, object]) -> list[str]:
    checks = [
        ("too_few_tokens", row["tokens"] < MIN_TOKENS),
        ("oversize_page", max(row["width_pt"], row["height_pt"]) > MAX_PAGE_SIDE_PT),
        ("cid_garbage", row["garbage_fraction"] > MAX_GARBAGE_FRACTION),
        ("symbol_garbage", row["alnum_fraction"] < MIN_ALNUM_FRACTION),
        ("no_language", row["lang"] is None),
        ("no_language_signal", row["stopword_rate"] < MIN_STOPWORD_RATE),
        ("ambiguous_language", row["language_margin"] < MIN_LANGUAGE_MARGIN),
        ("split_letters", row["script"] == "Latin" and row["single_letter_rate"] > MAX_SINGLE_LETTER_RATE),
        ("glued_words", row["script"] == "Latin" and row["mean_token_len"] > MAX_MEAN_TOKEN_LEN),
        ("order_chaos", row["order_jumps"] > max(MIN_ORDER_JUMPS, MAX_ORDER_JUMP_FRACTION * row["lines"])),
        ("split_words", row["touching_words"] > max(2, MAX_TOUCHING_WORD_FRACTION * row["words"])),
        ("arabic_ligature_order", row["arabic_ligature_errors"] > 0),
        ("raster_images", row["image_area_fraction"] > MAX_IMAGE_AREA_FRACTION),
    ]
    return [name for name, failed in checks if failed]


def cmd_inventory(args: argparse.Namespace) -> Path:
    out = args.out / "inventory.jsonl"
    stamp = args.out / "inventory.sources.sha256"
    docs = args.sources.documents()
    if not docs:
        raise SystemExit(f"no source PDFs found under {args.corpus}; fetch the corpus first")
    fingerprint = args.sources.fingerprint(docs)
    if out.exists() and stamp.exists() and stamp.read_text().strip() == fingerprint and not args.force:
        print(f"inventory cached: {out}", file=sys.stderr)
        return out
    args.out.mkdir(parents=True, exist_ok=True)
    rows: list[dict[str, object]] = []
    with concurrent.futures.ThreadPoolExecutor(max_workers=args.jobs) as pool:
        for i, doc_rows in enumerate(pool.map(lambda rel: inventory_document(args.sources, rel, args.seed), docs)):
            rows.extend(doc_rows)
            if (i + 1) % 50 == 0:
                print(f"inventory: {i + 1}/{len(docs)} documents", file=sys.stderr)
    with out.open("w", encoding="utf-8") as handle:
        for row in rows:
            handle.write(json.dumps(row, ensure_ascii=False, sort_keys=True) + "\n")
    stamp.write_text(fingerprint + "\n")
    print(f"inventory: {len(docs)} documents, {len(rows)} pages -> {out}", file=sys.stderr)
    return out


def load_inventory(out: Path) -> list[dict[str, object]]:
    with (out / "inventory.jsonl").open(encoding="utf-8") as handle:
        return [json.loads(line) for line in handle]


def select_pages(rows: list[dict[str, object]], per_stratum: int, seed: int) -> list[dict[str, object]]:
    """Every language and every layout class gets up to `per_stratum` pages, spread over documents."""
    eligible = sorted(
        (r for r in rows if not r["excluded"]),
        key=lambda r: stable_key(seed, r["doc"], r["page"]),
    )
    chosen: dict[tuple[str, int], dict[str, object]] = {}
    per_doc: collections.Counter[tuple[str, str]] = collections.Counter()
    by_lang = collections.defaultdict(list)
    for row in eligible:
        by_lang[row["lang"]].append(row)
    for lang in sorted(by_lang, key=lambda name: (len(by_lang[name]), name)):
        _take(by_lang[lang], f"lang:{lang}", per_stratum, chosen, per_doc, prefer_rare_layout=True)
    layouts = sorted({r["layout"] for r in eligible})
    for layout in layouts:
        have = sum(1 for r in chosen.values() if r["layout"] == layout)
        pool = [r for r in eligible if r["layout"] == layout]
        _take(pool, f"layout:{layout}", per_stratum - have, chosen, per_doc, prefer_rare_layout=False)
    return sorted(chosen.values(), key=lambda r: (r["lang"], r["doc"], r["page"]))


def _take(
    pool: list[dict[str, object]],
    stratum: str,
    want: int,
    chosen: dict[tuple[str, int], dict[str, object]],
    per_doc: collections.Counter[tuple[str, str]],
    *,
    prefer_rare_layout: bool,
) -> None:
    if want <= 0:
        return
    layout_rank = collections.Counter(r["layout"] for r in pool)
    candidates = [r for r in pool if (r["doc"], r["page"]) not in chosen]
    if prefer_rare_layout:
        candidates.sort(key=lambda r: layout_rank[r["layout"]])
    taken = 0
    for cap in (1, 2, MAX_PAGES_PER_DOC_PER_STRATUM, MAX_PAGES_PER_DOC):
        for row in candidates:
            if taken >= want:
                return
            key = (row["doc"], row["page"])
            if key in chosen or per_doc[(stratum, row["doc"])] >= cap:
                continue
            chosen[key] = row
            per_doc[(stratum, row["doc"])] += 1
            taken += 1


def page_id(row: dict[str, object]) -> str:
    stem = re.sub(r"[^A-Za-z0-9]+", "_", Path(str(row["doc"])).stem).strip("_").lower()[:40]
    digest = hashlib.sha256(str(row["doc"]).encode()).hexdigest()[:6]
    return f"{stem}_{digest}_p{int(row['page']):03d}"


DEGRADATION_PROFILES = {
    "mild": {"rotate_deg": 1.0, "blur_sigma": (0.3, 0.8), "noise_attenuate": (0.15, 0.35), "jpeg_quality": (60, 85)},
    "harsh": {"rotate_deg": 1.0, "blur_sigma": (0.6, 1.6), "noise_attenuate": (0.3, 0.7), "jpeg_quality": (30, 50)},
}


def degradation_params(seed: int, pid: str, dpi: int, profile: str) -> dict[str, float | int | str]:
    # The mild key omits the profile name so its parameters match corpora built before profiles existed.
    rng = seeded_rng(seed, pid, dpi) if profile == "mild" else seeded_rng(seed, pid, dpi, profile)
    spec = DEGRADATION_PROFILES[profile]
    return {
        "profile": profile,
        "rotate_deg": round(rng.uniform(-spec["rotate_deg"], spec["rotate_deg"]), 2),
        "blur_sigma": round(rng.uniform(*spec["blur_sigma"]) * dpi / 300, 2),
        "noise_attenuate": round(rng.uniform(*spec["noise_attenuate"]), 2),
        "jpeg_quality": rng.randint(*spec["jpeg_quality"]),
        "magick_seed": rng.randint(1, 2**31 - 1),
    }


def render_variant(sources: Sources, out: Path, row: dict[str, object], variant: dict[str, object], seed: int) -> dict:
    pid = page_id(row)
    dpi = int(variant["dpi"])
    images = out / "images"
    clean_png = images / f"{pid}_clean{dpi}.png"
    if not clean_png.exists():
        partial = images / f".partial_{pid}_clean{dpi}"
        run(
            [
                "pdftoppm",
                "-f",
                str(row["page"]),
                "-l",
                str(row["page"]),
                "-r",
                str(dpi),
                "-gray",
                "-png",
                "-singlefile",
                str(sources.resolve(str(row["doc"]))),
                str(partial),
            ]
        )
        partial.with_suffix(".png").replace(clean_png)
    if not variant["degraded"]:
        return {"path": clean_png, "params": None}
    params = degradation_params(seed, pid, dpi, str(variant["profile"]))
    target = images / f"{pid}_{variant['name']}.jpg"
    if not target.exists():
        partial = images / f".partial_{target.name}"
        run(
            [
                "magick",
                "-seed",
                str(params["magick_seed"]),
                str(clean_png),
                "-colorspace",
                "Gray",
                "-background",
                "white",
                "-rotate",
                str(params["rotate_deg"]),
                "-blur",
                f"0x{params['blur_sigma']}",
                "-attenuate",
                str(params["noise_attenuate"]),
                "+noise",
                "Gaussian",
                "-strip",
                "-quality",
                str(params["jpeg_quality"]),
                f"jpg:{partial}",
            ]
        )
        partial.replace(target)
    return {"path": target, "params": params}


VARIANTS = (
    {"name": "clean300", "dpi": 300, "degraded": False},
    {"name": "scan150", "dpi": 150, "degraded": True, "profile": "mild"},
    {"name": "scan200", "dpi": 200, "degraded": True, "profile": "mild"},
    {"name": "scan300", "dpi": 300, "degraded": True, "profile": "mild"},
    {"name": "harsh", "dpi": 100, "degraded": True, "profile": "harsh"},
)


def write_ground_truth(sources: Sources, out: Path, row: dict[str, object]) -> Path:
    target = out / "ground_truth" / f"{page_id(row)}.txt"
    if not target.exists():
        text = run(
            [
                "pdftotext",
                "-f",
                str(row["page"]),
                "-l",
                str(row["page"]),
                *TEXT_FLAGS,
                str(sources.resolve(str(row["doc"]))),
                "-",
            ]
        )
        partial = target.with_name(f".partial_{target.name}")
        partial.write_text(text.rstrip("\f\n") + "\n", encoding="utf-8")
        partial.replace(target)
    return target


def ocr_language(row: dict[str, object]) -> str:
    code = TESSERACT_CODES[str(row["lang"])]
    if code != "eng" and row["script"] != "Latin" and float(row["latin_share"]) > 0.1:
        code += "+eng"
    return code


def build_fixture(sources: Sources, out: Path, row: dict, variant: dict, *, seed: int, versions: dict) -> str:
    pid = page_id(row)
    gt = write_ground_truth(sources, out, row)
    image = render_variant(sources, out, row, variant, seed)
    path: Path = image["path"]
    name = f"synth_{str(row['lang']).lower().replace('-', '_')}_{pid}_{variant['name']}"
    fixture = {
        "document": f"../images/{path.name}",
        "file_type": path.suffix.lstrip("."),
        "file_size": path.stat().st_size,
        "expected_frameworks": ["xberg"],
        "metadata": {
            "description": f"Synthetic {variant['name']} render of {row['doc']} page {row['page']}",
            "category": "image-ocr-synthetic",
            "dataset": DATASET,
            "requires_ocr": True,
            "languages": [row["lang"]],
            "ocr_language": ocr_language(row),
            "script": row["script"],
            "layout": row["layout"],
            "dpi": variant["dpi"],
            "variant": variant["name"],
            "degradation": image["params"],
            "source_document": row["doc"],
            "source_page": row["page"],
            "source_sha256": sources.shas[str(row["doc"])],
            "source_provenance": sources.origin(str(row["doc"])),
            "seed": seed,
            "gt_extractor": f"{versions['pdftotext']}: pdftotext {' '.join(TEXT_FLAGS)} (reading-order mode)",
        },
        "ground_truth": {"text_file": f"../ground_truth/{gt.name}", "source": GT_SOURCE},
    }
    (out / "fixtures" / f"{name}.json").write_text(
        json.dumps(fixture, indent=2, ensure_ascii=False) + "\n", encoding="utf-8"
    )
    return f"{name}.json"


def batch_size_for(count: int) -> int:
    return next(size for size in (8, 4, 2, 1) if count % size == 0)


def write_cohort(out: Path, name: str, fixtures: list[str]) -> None:
    cohort = {
        "schema_version": 1,
        "name": name,
        "batch_size": batch_size_for(len(fixtures)),
        "ocr_enabled": True,
        "fixtures": sorted(fixtures),
    }
    (out / "cohorts" / f"{name}.json").write_text(json.dumps(cohort, indent=2) + "\n", encoding="utf-8")


def cmd_build(args: argparse.Namespace) -> None:
    cmd_inventory(args)
    rows = load_inventory(args.out)
    selected = select_pages(rows, args.per_stratum, args.seed)
    for sub in ("images", "ground_truth", "fixtures", "cohorts"):
        (args.out / sub).mkdir(parents=True, exist_ok=True)
    versions = tool_versions()

    def build_page(row: dict[str, object]) -> list[tuple[str, str]]:
        # Variants of one page share the clean render, so they run sequentially in one task.
        return [
            (variant["name"], build_fixture(args.sources, args.out, row, variant, seed=args.seed, versions=versions))
            for variant in VARIANTS
        ]

    by_variant: dict[str, list[str]] = collections.defaultdict(list)
    with concurrent.futures.ThreadPoolExecutor(max_workers=args.jobs) as pool:
        for page_fixtures in pool.map(build_page, selected):
            for variant, name in page_fixtures:
                by_variant[variant].append(name)
    for variant, names in sorted(by_variant.items()):
        write_cohort(args.out, f"ocr-synth-{variant}-v1", names)
    write_manifest(args, rows, selected, versions)


def stratum_counts(rows: list[dict[str, object]]) -> dict[str, dict[str, int]]:
    return {
        "language": dict(sorted(collections.Counter(str(r["lang"]) for r in rows).items())),
        "script": dict(sorted(collections.Counter(str(r["script"]) for r in rows).items())),
        "layout": dict(sorted(collections.Counter(str(r["layout"]) for r in rows).items())),
        "documents_per_language": {
            lang: len({r["doc"] for r in rows if r["lang"] == lang}) for lang in sorted({str(r["lang"]) for r in rows})
        },
    }


def write_manifest(args: argparse.Namespace, rows: list, selected: list, versions: dict) -> None:
    eligible = [r for r in rows if not r["excluded"]]
    manifest = {
        "dataset": DATASET,
        "seed": args.seed,
        "per_stratum": args.per_stratum,
        "tools": versions,
        "variants": list(VARIANTS),
        "ground_truth": {"source": GT_SOURCE, "mode": "pdftotext reading-order mode, per page", "flags": TEXT_FLAGS},
        "inventory": {
            "documents": len({r["doc"] for r in rows}),
            "pages_inspected": len(rows),
            "pages_eligible": len(eligible),
            "exclusions": dict(collections.Counter(reason for r in rows for reason in r["excluded"]).most_common()),
            "eligible_strata": stratum_counts(eligible),
        },
        "selected": {"pages": len(selected), "strata": stratum_counts(selected)},
        "under_filled": {
            f"lang:{lang}": count
            for lang, count in stratum_counts(selected)["language"].items()
            if count < args.per_stratum
        }
        | {
            f"layout:{layout}": count
            for layout, count in stratum_counts(selected)["layout"].items()
            if count < args.per_stratum
        },
        "pages": [
            {
                "id": page_id(r),
                "doc": r["doc"],
                "page": r["page"],
                "lang": r["lang"],
                "layout": r["layout"],
                "provenance": args.sources.origin(str(r["doc"])),
            }
            for r in selected
        ],
    }
    (args.out / "manifest.json").write_text(json.dumps(manifest, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    print(json.dumps({k: manifest[k] for k in ("inventory", "selected", "under_filled")}, indent=2, ensure_ascii=False))


def cmd_verify(args: argparse.Namespace) -> None:
    manifest = json.loads((args.out / "manifest.json").read_text(encoding="utf-8"))
    fixtures = sorted((args.out / "fixtures").glob(f"*_{args.variant}.json"))
    # Keyed without the variant so every variant samples the same pages.
    sample = seeded_rng(args.seed, "verify").sample(fixtures, min(args.samples, len(fixtures)))
    results = []
    for path in sample:
        fixture = json.loads(path.read_text(encoding="utf-8"))
        meta = fixture["metadata"]
        image = (path.parent / fixture["document"]).resolve()
        ocr = run(["tesseract", str(image), "-", "-l", meta["ocr_language"]])
        gt_default = (path.parent / fixture["ground_truth"]["text_file"]).read_text(encoding="utf-8")
        row = {
            "fixture": path.name,
            "lang": meta["languages"][0],
            "layout": meta["layout"],
            "tf1": round(token_f1(ocr, gt_default), 3),
        }
        pdf = args.sources.resolve(meta["source_document"])
        span = ["-f", str(meta["source_page"]), "-l", str(meta["source_page"]), *TEXT_FLAGS]
        for mode, flags in (("default", []), ("layout", ["-layout"]), ("raw", ["-raw"])):
            gt = gt_default if mode == "default" else run(["pdftotext", *span, *flags, str(pdf), "-"])
            score = reading_order(ocr, gt)
            row[f"ro_{mode}"] = None if score is None else round(score, 3)
        results.append(row)
        print(json.dumps(row, ensure_ascii=False))
    summary = {
        "variant": args.variant,
        "samples": len(results),
        "dataset": manifest["dataset"],
        **{
            f"mean_{key}": round(
                sum(r[key] for r in results if r[key] is not None) / max(1, sum(r[key] is not None for r in results)), 3
            )
            for key in ("tf1", "ro_default", "ro_layout", "ro_raw")
        },
    }
    print(json.dumps(summary))


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("stage", choices=("inventory", "build", "verify"))
    parser.add_argument("--corpus", type=Path, default=Path(__file__).resolve().parents[3] / "test_documents")
    parser.add_argument(
        "--sources",
        type=Path,
        help=f"extra native PDFs outside corpus.lock.json; {SOURCES_MANIFEST} there maps each PDF to its url and license",
    )
    parser.add_argument("--out", type=Path, required=True)
    parser.add_argument("--seed", type=int, default=DEFAULT_SEED)
    parser.add_argument("--per-stratum", type=int, default=DEFAULT_PER_STRATUM)
    parser.add_argument("--jobs", type=int, default=os.cpu_count() or 4)
    parser.add_argument("--force", action="store_true", help="recompute the cached inventory")
    parser.add_argument("--samples", type=int, default=10, help="verify: number of fixtures to OCR")
    parser.add_argument("--variant", default="clean300", help="verify: variant suffix to sample")
    args = parser.parse_args()
    TEXT_FLAGS.extend(pdftotext_text_flags())
    args.corpus, args.out = args.corpus.resolve(), args.out.resolve()
    args.sources = Sources(args.corpus, args.sources.resolve() if args.sources else None)
    if args.stage == "inventory":
        cmd_inventory(args)
    elif args.stage == "build":
        cmd_build(args)
    else:
        cmd_verify(args)
    return 0


if __name__ == "__main__":
    sys.exit(main())
