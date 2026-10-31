#!/usr/bin/env python3
"""Guard against a config field that is silently absent from every generated binding.

GH#1745. `mise run alef:verify` (`alef verify --exit-code`) is a freshness check: it
recomputes each generated file's expected hash from a hash of its own generation
inputs and compares that to what is committed. It is green whenever a fresh
regeneration WOULD reproduce the committed tree -- and it is equally green when a
field was simply never wired into the generator's field list in the first place,
because in that case a regen changes nothing and no hash moves. A field added to
`ExtractionConfig`, `ConcurrencyConfig` or `OcrConfig` that every binding builds via
`..Default::default()` / keyword defaults can therefore ship on Python, Node, PHP,
WASM and the C FFI header with no way to set it, and the freshness gate never
notices. Found while regenerating bindings for #1727; reproduced against a prior
2026-09-12 measurement where the same gate returned success with five binding
manifests reverted.

This is the cheaper of the two fixes GH#1745 proposes. Rather than running a full
`alef all --clean` regeneration in CI (minutes, and it needs sibling polyrepo
checkouts -- see the `alef-bindings-freshness` job in .github/workflows/ci-lint.yaml),
it asserts a purely structural property: every public, non-skipped field of the
config structs in TRACKED_STRUCTS appears BY NAME, in some casing convention,
somewhere inside each language's own generated binding surface.

Known limitation: this proves textual presence, not correct wiring -- a field could
be present as a doc-comment mention, or a short generic field name (e.g. "enabled")
could already appear in the surface for an unrelated reason, in which case the check
would not catch that specific field missing for that specific language. It is
deliberately the coarse, cheap check the issue names as an alternative to a full
regen-and-diff; it exists to catch the exact defect class described above, not to
replace `mise run alef:verify` or a full regeneration.

Fields marked `#[serde(skip)]` (e.g. `ExtractionConfig::cancel_token`,
`ExtractionConfig::source_name`, `OcrConfig::tessdata_bytes`) never reach the wire
format at all and are excluded from the required set, as are `#[cfg(...)]`-gated
fields (e.g. `ExtractionConfig::tree_sitter`), whose presence in a given binding
depends on which features that binding's generation build enabled -- a question this
static, per-struct check has no way to answer per language.

Exit codes: 0 = every field found in every language's surface, 1 = at least one
field is missing from at least one language, 2 = malformed input (struct not found,
unbalanced braces, or a language surface path does not exist).
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]

# Config structs whose fields every binding constructs positionally or via a
# default-spread -- the exact drift class GH#1745 describes. Not exhaustive of every
# `#[serde(deny_unknown_fields)]` struct under crates/xberg/src/core/config: extend
# this tuple when a new top-level config struct gets the same field-by-field mirror
# treatment across bindings. ~keep
TRACKED_STRUCTS: tuple[tuple[str, str], ...] = (
    ("crates/xberg/src/core/config/extraction/core.rs", "ExtractionConfig"),
    ("crates/xberg/src/core/config/concurrency.rs", "ConcurrencyConfig"),
    ("crates/xberg/src/core/config/ocr.rs", "OcrConfig"),
)

# Each language's generated-binding surface: files/directories searched recursively
# for a field name. Build outputs, vendored deps and node_modules are excluded so a
# stale or foreign artifact can neither mask nor fake coverage. ~keep
LANGUAGE_SURFACES: dict[str, tuple[str, ...]] = {
    "python": ("packages/python/xberg",),
    "node": ("crates/xberg-node/index.d.ts", "crates/xberg-node/src"),
    "php": ("crates/xberg-php/src",),
    "wasm": ("crates/xberg-wasm/src",),
    "c": ("crates/xberg-ffi/include", "crates/xberg-ffi/src"),
    "csharp": ("packages/csharp/src",),
    "dart": ("packages/dart/lib", "packages/dart/rust/src"),
    "go": ("packages/go/binding.go", "packages/go/include"),
    "java": ("packages/java/io",),
    "kotlin": ("packages/kotlin-android/src",),
    "ruby": ("packages/ruby/sig", "packages/ruby/ext/xberg_rb/src"),
    "swift": ("packages/swift/Sources", "packages/swift/rust/src"),
    "zig": ("packages/zig/src",),
    "elixir": ("packages/elixir/lib", "packages/elixir/native/xberg_nif/src"),
}

EXCLUDED_DIR_NAMES = frozenset(
    {
        "node_modules",
        "vendor",
        "deps",
        "_build",
        ".dart_tool",
        "tmp",
        "pkg",
        ".kotlin",
        "gradle",
        ".alef-cache",
        "__pycache__",
        ".bundle",
    }
)

# Matches a top-level `pub field_name: Type,` line. Deliberately requires whitespace
# directly after `pub` so `pub(crate)`/`pub(super)` fields -- not part of the public
# wire surface -- are excluded without a separate filter. ~keep
FIELD_LINE_RE = re.compile(r"^\s*pub\s+(\w+)\s*:")

# `#[serde(skip)]` (and the matching `#[cfg_attr(alef, alef(skip))]`) removes a field
# from the wire format entirely -- e.g. `cancel_token`, `source_name`, `tessdata_bytes` --
# so bindings never see it and it must not be required here. `skip_serializing_if` is a
# different, unrelated attribute (an optional field that IS still part of the wire
# format) and must not trip this. ~keep
SKIP_ATTR_RE = re.compile(r"skip(?!_serializing_if)")
CFG_ATTR_RE = re.compile(r"^\s*#\[cfg\(")


def _fail(message: str) -> None:
    print(f"FAIL: {message}", file=sys.stderr)


def find_struct_body(text: str, struct_name: str, source: Path) -> str:
    """Return the source text between a `pub struct <struct_name> { ... }`'s braces."""
    match = re.search(rf"\bpub struct {re.escape(struct_name)}\b[^{{]*\{{", text)
    if not match:
        raise ValueError(f"{source}: could not find `pub struct {struct_name}` with a body")

    depth = 1
    index = match.end()
    while index < len(text) and depth > 0:
        char = text[index]
        if char == "{":
            depth += 1
        elif char == "}":
            depth -= 1
        index += 1

    if depth != 0:
        raise ValueError(f"{source}: unbalanced braces scanning `{struct_name}`")

    return text[match.end() : index - 1]


def extract_fields(body: str) -> list[str]:
    """Return the required (non-`skip`, non-`cfg`-gated) top-level `pub` field names."""
    fields: list[str] = []
    pending_skip = False
    pending_cfg = False
    for line in body.splitlines():
        stripped = line.strip()
        if stripped.startswith("#["):
            if SKIP_ATTR_RE.search(stripped):
                pending_skip = True
            if CFG_ATTR_RE.match(line):
                pending_cfg = True
            continue
        if not stripped or stripped.startswith("//"):
            continue
        match = FIELD_LINE_RE.match(line)
        if match:
            if not pending_skip and not pending_cfg:
                fields.append(match.group(1))
            pending_skip = False
            pending_cfg = False
    return fields


def casing_variants(field: str) -> tuple[str, ...]:
    """Return the snake_case, camelCase and PascalCase spellings of a field name."""
    parts = field.split("_")
    camel = parts[0] + "".join(part.capitalize() for part in parts[1:])
    pascal = "".join(part.capitalize() for part in parts)
    return (field, camel, pascal)


def surface_text(paths: tuple[str, ...]) -> str:
    """Concatenate every readable file under the given surface paths."""
    chunks: list[str] = []
    for raw_path in paths:
        path = REPO_ROOT / raw_path
        if not path.exists():
            raise ValueError(f"language surface path does not exist: {raw_path}")
        if path.is_file():
            chunks.append(path.read_text(encoding="utf-8", errors="ignore"))
            continue
        for file_path in sorted(path.rglob("*")):
            if not file_path.is_file():
                continue
            if EXCLUDED_DIR_NAMES.intersection(file_path.relative_to(path).parts):
                continue
            chunks.append(file_path.read_text(encoding="utf-8", errors="ignore"))
    return "\n".join(chunks)


def check_struct(module_path: str, struct_name: str, language_text: dict[str, str]) -> list[str]:
    """Return one failure line per (field, language) pair missing the field."""
    source = REPO_ROOT / module_path
    text = source.read_text(encoding="utf-8")
    body = find_struct_body(text, struct_name, source)
    fields = extract_fields(body)
    if not fields:
        raise ValueError(f"{source}: found `pub struct {struct_name}` but extracted zero fields")

    failures: list[str] = []
    for field in fields:
        variants = casing_variants(field)
        for language, content in language_text.items():
            if not any(variant in content for variant in variants):
                failures.append(f"{struct_name}.{field} missing from {language} surface ({module_path})")
    return failures


def main() -> int:
    try:
        language_text = {language: surface_text(paths) for language, paths in LANGUAGE_SURFACES.items()}
    except ValueError as error:
        _fail(str(error))
        return 2

    all_failures: list[str] = []
    for module_path, struct_name in TRACKED_STRUCTS:
        try:
            all_failures.extend(check_struct(module_path, struct_name, language_text))
        except ValueError as error:
            _fail(str(error))
            return 2

    if all_failures:
        for failure in all_failures:
            _fail(failure)
        print(f"FAIL: {len(all_failures)} field/language gap(s)", file=sys.stderr)
        return 1

    field_count = sum(
        len(extract_fields(find_struct_body((REPO_ROOT / path).read_text(encoding="utf-8"), name, REPO_ROOT / path)))
        for path, name in TRACKED_STRUCTS
    )
    print(f"OK: {field_count} tracked field(s) across {len(TRACKED_STRUCTS)} struct(s) found in every binding surface")
    return 0


if __name__ == "__main__":
    sys.exit(main())
