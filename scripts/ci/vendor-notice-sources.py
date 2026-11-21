#!/usr/bin/env python3
"""Vendor the license texts for every pinned third-party component into the repository.

The notice bundle may not point at a URL instead of carrying the text, and generation must be
offline and byte-stable, so the texts a pinned archive ships are copied here once, keyed by the
digest of the archive the build verifies. `generate-notices.py` then reads them back and refuses
to run when a build input pins a digest this manifest has never seen.

Downloads happen only when a pin is new or its vendored text is missing; everything else is
reused. Run this after bumping a native dependency, then regenerate the bundle.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import urllib.request
from dataclasses import dataclass
from pathlib import Path

import notices_sources as sources

VENDORED = sources.NOTICES_DIR / "vendored"
SPDX_TAG = "v3.27.0"
SPDX_URL = f"https://raw.githubusercontent.com/spdx/license-list-data/{SPDX_TAG}/text/{{id}}.txt"
ARCHIVE_CACHE = Path.home() / ".cache" / "xberg-notices"


@dataclass(frozen=True)
class Spec:
    """How one component's license texts reach the manifest."""

    id: str
    license: str
    summary: str
    # (archive member basename, display name); used for pinned download archives.
    archive_texts: tuple[tuple[str, str], ...] = ()
    # (repo-relative path, display name); texts already committed in this repository.
    repo_texts: tuple[tuple[str, str], ...] = ()
    # Canonical SPDX license ids to vendor for components with no upstream text in the archive.
    spdx: tuple[str, ...] = ()
    # A committed archive shared by every pin (the vendored libwpd sources).
    repo_archive: str = ""
    # Texts required by license terms even when no pin names them (LGPL-3.0 section 4(b)).
    always: bool = False


SPECS = (
    Spec("libheif", "LGPL-3.0-or-later", "HEIF/HEIC/AVIF container (shared library)", (("COPYING", "COPYING"),)),
    Spec("libde265", "LGPL-3.0-or-later", "HEVC decoder", (("COPYING", "COPYING"),)),
    Spec("libaom", "BSD-2-Clause", "AV1 encoder/decoder", (("LICENSE", "LICENSE"), ("PATENTS", "PATENTS"))),
    Spec("x265", "GPL-2.0-or-later", "HEVC encoder", (("COPYING", "COPYING"),)),
    Spec("libdav1d", "BSD-2-Clause", "AV1 decoder", spdx=("BSD-2-Clause",)),
    Spec(
        "tesseract",
        "Apache-2.0",
        "OCR engine",
        repo_texts=(("crates/xberg-tesseract/licenses/tesseract-LICENSE", "LICENSE"),),
    ),
    Spec(
        "leptonica",
        "BSD-2-Clause",
        "image processing library used by Tesseract",
        repo_texts=(("crates/xberg-tesseract/licenses/leptonica-LICENSE", "LICENSE"),),
    ),
    Spec(
        "libwpd",
        "MPL-2.0",
        "WordPerfect parser (built from the vendored source archive)",
        (("COPYING.MPL", "COPYING.MPL"),),
        repo_archive="crates/xberg-libwpd/vendor/libwpd-0.10.3.tar.gz",
    ),
    Spec(
        "librevenge",
        "MPL-2.0",
        "document import framework used by libwpd (vendored source archive)",
        (("COPYING.MPL", "COPYING.MPL"),),
        repo_archive="crates/xberg-libwpd/vendor/librevenge-0.0.6.tar.gz",
    ),
    Spec("boost", "BSL-1.0", "header-only subset compiled into the libwpd shim", spdx=("BSL-1.0",)),
    Spec(
        "pdfium",
        "BSD-3-Clause",
        "PDF rendering engine",
        repo_texts=(("crates/xberg-pdfium-render/include/LICENSE.pdfium", "LICENSE"),),
    ),
    Spec("onnxruntime", "MIT", "ONNX Runtime inference engine", (("LICENSE", "LICENSE"),)),
    Spec(
        "dejavu",
        "Bitstream-Vera",
        "DejaVu Sans and DejaVu Sans Bold (embedded PDF fonts)",
        repo_texts=(("crates/xberg-native-pdf/src/fonts/assets/LICENSE-DejaVu", "LICENSE"),),
    ),
    Spec(
        "droid-sans-fallback",
        "Apache-2.0",
        "Droid Sans Fallback (embedded CJK form font)",
        repo_texts=(("crates/xberg-native-pdf/src/fonts/assets/LICENSE-fallback-fonts", "NOTICE"),),
        spdx=("Apache-2.0",),
    ),
    Spec(
        "noto-emoji",
        "OFL-1.1",
        "Noto Emoji (embedded emoji form font)",
        repo_texts=(("crates/xberg-native-pdf/src/fonts/assets/LICENSE-fallback-fonts", "NOTICE"),),
        spdx=("OFL-1.1",),
    ),
    Spec(
        "gpl-3.0-text",
        "GPL-3.0-only",
        "GNU GPL v3 text carried beside the LGPL v3 components (LGPL-3.0 section 4(b))",
        repo_texts=(("scripts/ci/notices/GPL-3.0.txt", "GPL-3.0.txt"),),
        always=True,
    ),
)

# SPDX ids referenced by Rust crates that ship no license file of their own. The generator
# resolves the exact set from the graph; these are vendored unconditionally so the set only ever
# needs re-vendoring when a crate introduces a new license id.
BASE_SPDX = (
    "0BSD",
    "Apache-2.0",
    "BSD-2-Clause",
    "BSD-3-Clause",
    "BSL-1.0",
    "CC0-1.0",
    "LGPL-2.1-or-later",
    "MIT",
    "OFL-1.1",
    "Zlib",
)


def sha256_bytes(content: bytes) -> str:
    """Return the sha256 of in-memory bytes."""
    return hashlib.sha256(content).hexdigest()


def download(url: str, destination: Path) -> None:
    """Fetch a pinned asset once, reusing the cache when its digest has not changed."""
    if destination.is_file():
        return
    destination.parent.mkdir(parents=True, exist_ok=True)
    request = urllib.request.Request(url, headers={"User-Agent": "xberg-notice-vendor/1"})  # noqa: S310
    # Pinned https URLs from the build inputs; scheme confusion is not reachable here. ~keep
    with urllib.request.urlopen(request, timeout=120) as response:  # noqa: S310
        payload = response.read()
    destination.write_bytes(payload)


def fetch_archive(url: str, expected: str, cache: Path) -> Path:
    """Download a pinned archive and prove its digest before reading a license out of it."""
    name = hashlib.sha256(url.encode()).hexdigest()[:16] + "-" + (url.rsplit("/", 1)[-1] or "archive")
    path = cache / name
    download(url, path)
    actual = sources.digest(path)
    if actual != expected:
        raise ValueError(f"{url} has sha256 {actual}, expected {expected}")
    return path


def archive_text(archive: Path, member: str) -> bytes:
    """Read a license file out of a verified archive."""
    return sources.archive_member(archive, member)


def write_text(path: Path, content: bytes) -> None:
    """Write a vendored text deterministically, keeping the exact bytes the archive carried."""
    path.parent.mkdir(parents=True, exist_ok=True)
    if path.is_file() and path.read_bytes() == content:
        return
    path.write_bytes(content)


def vendor_spdx(root: Path, ids: tuple[str, ...], cache: Path) -> dict[str, tuple[str, str]]:
    """Vendor canonical SPDX license texts and return id -> (path, digest)."""
    vendored = {}
    for license_id in ids:
        url = SPDX_URL.format(id=license_id)
        path = cache / "spdx" / f"{license_id}.txt"
        download(url, path)
        content = path.read_bytes()
        if not content.strip():
            raise ValueError(f"empty SPDX text for {license_id}")
        target = root / VENDORED / "spdx" / f"{license_id}.txt"
        write_text(target, content)
        vendored[license_id] = (target.relative_to(root).as_posix(), sha256_bytes(content))
    return vendored


def component_texts(
    root: Path, spec: Spec, pins: list[sources.Pin], cache: Path
) -> dict[str, list[tuple[str, str, str]]]:
    """Collect the license texts for one component, downloading each pinned archive once."""
    texts: dict[str, list[tuple[str, str, str]]] = {}
    if spec.spdx:
        for license_id in spec.spdx:
            path = root / VENDORED / "spdx" / f"{license_id}.txt"
            if not path.is_file():
                raise ValueError(f"missing vendored SPDX text {path}; run with the network available")
            texts[""] = [
                *texts.get("", []),
                (f"{license_id}.txt", path.relative_to(root).as_posix(), sources.digest(path)),
            ]
    if spec.repo_texts:
        for path, name in spec.repo_texts:
            texts.setdefault("", []).append((name, path, sources.digest(root / path)))
    if spec.repo_texts and not spec.archive_texts:
        return texts
    if not pins:
        return texts
    if spec.repo_archive:
        archive = root / spec.repo_archive
        for _, name in spec.archive_texts:
            content = sources.archive_member(archive, name)
            target = root / VENDORED / spec.id / name
            write_text(target, content)
            texts.setdefault("", []).append((name, target.relative_to(root).as_posix(), sha256_bytes(content)))
        return texts
    by_version: dict[str, list[sources.Pin]] = {}
    for pin in pins:
        if pin.url and pin.sha256:
            by_version.setdefault(pin.version, []).append(pin)
    for version in sorted(by_version, key=sources.version_key):
        candidates = sorted(
            by_version[version], key=lambda item: (not item.url.endswith((".tgz", ".tar.gz")), item.url)
        )
        pin = candidates[0]
        archive = fetch_archive(pin.url, pin.sha256 or "", cache)
        for member, name in spec.archive_texts:
            content = archive_text(archive, member)
            target = root / VENDORED / spec.id / version / name
            write_text(target, content)
            texts.setdefault(version, []).append((name, target.relative_to(root).as_posix(), sha256_bytes(content)))
    return texts


VCPKG_RAW = "https://raw.githubusercontent.com/microsoft/vcpkg/{baseline}/ports/{port}/{file}"
VCPKG_PORTS = {
    "libheif": ("libheif", None),
    "libde265": ("libde265", "libde265"),
    "libaom": ("aom", "aom"),
    "libdav1d": ("dav1d", "dav1d"),
}


def vendor_vcpkg(root: Path, cache: Path) -> tuple[str, list[tuple[str, str, str, str, list[str]]]]:
    """Vendor the port metadata the Windows build compiles, keyed by its pinned baseline."""
    script = (root / "scripts/ci/install-system-deps/install-windows.ps1").read_text(encoding="utf-8")
    baseline = re.search(r"\$baseline = '([0-9a-f]{40})'", script)
    if baseline is None:
        raise ValueError("install-windows.ps1 has no pinned vcpkg baseline")
    enabled = set(sources.heif_feature_set(script))
    ports = []
    for component in ("libheif", "libde265", "libaom", "libdav1d"):
        port, feature = VCPKG_PORTS[component]
        if feature is not None and feature not in enabled:
            continue
        portfile_url = VCPKG_RAW.format(baseline=baseline.group(1), port=port, file="portfile.cmake")
        manifest_url = VCPKG_RAW.format(baseline=baseline.group(1), port=port, file="vcpkg.json")
        portfile = download_text(portfile_url, cache / "vcpkg" / f"{port}-portfile.cmake")
        manifest = json.loads(download_text(manifest_url, cache / "vcpkg" / f"{port}-vcpkg.json"))
        version = manifest.get("version") or manifest.get("version-semver") or manifest.get("version-string")
        digest = re.search(r"SHA512\s+([0-9a-f]{128})", portfile)
        commit = re.search(r"\bREF\s+([0-9a-f]{40})\b", portfile)
        recorded = digest.group(1) if digest else (f"commit {commit.group(1)}" if commit else "")
        patches = []
        block = re.search(r"PATCHES\s*(.*?)\n\)", portfile, re.DOTALL)
        if block:
            patches = [
                line.strip().strip('"')
                for line in block.group(1).splitlines()
                if line.strip() and not line.strip().startswith("#")
            ]
        repo = re.search(r"REPO\s+(\S+)", portfile)
        git_url = re.search(r"URL\s+\"([^\"]+)\"", portfile)
        source = (
            f"https://github.com/{repo.group(1)}"
            if repo
            else (git_url.group(1) if git_url else manifest.get("homepage", ""))
        )
        ports.append((port, str(version or ""), source, recorded, patches))
    return baseline.group(1), ports


def download_text(url: str, destination: Path) -> str:
    """Fetch one pinned text file, reusing the cache."""
    download(url, destination)
    return destination.read_text(encoding="utf-8")


def discover_offers(root: Path) -> list[tuple[str, str, list[str], str]]:
    """Find written source offers and the channels whose artifacts their text names."""
    return [
        (path.relative_to(root).as_posix(), sources.digest(path), channels, shipped_as)
        for path, shipped_as, channels in sources.discover_offer_files(root)
    ]


def toml_array(key: str, values: list[str]) -> list[str]:
    """Render a string array the way poly formats it: one line until it exceeds 120 columns."""
    single = f"{key} = [" + ", ".join(toml_value(value) for value in values) + "]"
    if len(single) <= 120:
        return [single]
    return [f"{key} = [", *(f"  {toml_value(value)}," for value in values), "]"]


def toml_value(value: str) -> str:
    """Quote a string for the generated TOML manifest."""
    return '"' + value.replace("\\", "\\\\").replace('"', '\\"') + '"'


def render_manifest(rows: list[str]) -> str:
    """Render the manifest with a stable header and one block per component."""
    header = (
        "# Generated by scripts/ci/vendor-notice-sources.py from the pinned build inputs.\n"
        "# Do not edit: bump the dependency in its build input, then re-run the generator.\n"
        f"# SPDX texts: https://github.com/spdx/license-list-data {SPDX_TAG}.\n\n"
    )
    return header + "\n".join(rows) + "\n"


def component_rows(root: Path, spec: Spec, pins: list[sources.Pin], cache: Path) -> list[str]:
    """Render one component block, including every distinct pin and its texts."""
    per_pin = component_texts(root, spec, pins, cache)
    fallback = per_pin.get("") or []
    if not fallback:
        newest = max((version for version in per_pin if version), key=sources.version_key, default="")
        fallback = per_pin.get(newest, [])
    if not fallback:
        raise ValueError(f"{spec.id} has no license text")
    rows = ["[[component]]", f"id = {toml_value(spec.id)}", f"license = {toml_value(spec.license)}"]
    rows.append(f"summary = {toml_value(spec.summary)}")
    if spec.always:
        rows.append("always = true")
    for name, path, digest in fallback:
        rows.extend(
            [
                "",
                "[[component.texts]]",
                f"name = {toml_value(name)}",
                f"path = {toml_value(path)}",
                f"sha256 = {toml_value(digest)}",
            ]
        )
    seen: set[tuple[str, str | None, str]] = set()
    for pin in sorted(pins, key=lambda item: (sources.version_key(item.version), item.sha256 or "", item.url)):
        if (pin.version, pin.sha256, pin.url) in seen:
            continue
        seen.add((pin.version, pin.sha256, pin.url))
        rows.extend(["", "[[component.pin]]", f"version = {toml_value(pin.version)}", f"url = {toml_value(pin.url)}"])
        if pin.sha256:
            rows.append(f"sha256 = {toml_value(pin.sha256)}")
        for name, path, digest in per_pin.get(pin.version) or fallback:
            rows.extend(
                [
                    "",
                    "[[component.pin.texts]]",
                    f"name = {toml_value(name)}",
                    f"path = {toml_value(path)}",
                    f"sha256 = {toml_value(digest)}",
                ]
            )
    rows.append("")
    return rows


def release_rows(root: Path, cache: Path, vendored_spdx: dict[str, tuple[str, str]]) -> list[str]:
    """Render the offers, vcpkg ports and SPDX texts the generator also consumes."""
    rows: list[str] = []
    for path, digest, channels, shipped_as in discover_offers(root):
        rows.extend(
            [
                "[[offer]]",
                f"path = {toml_value(path)}",
                f"sha256 = {toml_value(digest)}",
                f"shipped_as = {toml_value(shipped_as)}",
                *toml_array("channels", channels),
                "",
            ]
        )
    baseline, ports = vendor_vcpkg(root, cache)
    rows.extend(["[[vcpkg]]", f"baseline = {toml_value(baseline)}"])
    for name, version, source, digest, patches in ports:
        rows.extend(
            [
                "",
                "[[vcpkg.port]]",
                f"name = {toml_value(name)}",
                f"version = {toml_value(version)}",
                f"source = {toml_value(source)}",
                f"digest = {toml_value(digest)}",
                *toml_array("patches", patches),
            ]
        )
    rows.append("")
    for license_id, (path, digest) in sorted(vendored_spdx.items()):
        rows.extend(
            [
                "[[spdx]]",
                f"id = {toml_value(license_id)}",
                f"path = {toml_value(path)}",
                f"sha256 = {toml_value(digest)}",
                "",
            ]
        )
    return rows


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parents[2])
    parser.add_argument("--cache", type=Path, default=ARCHIVE_CACHE)
    args = parser.parse_args()
    root = args.root.resolve()
    cache = args.cache

    inputs = sources.build_inputs(root)
    spdx_ids = tuple(sorted(set(BASE_SPDX) | {item for spec in SPECS for item in spec.spdx}))
    vendored_spdx = vendor_spdx(root, spdx_ids, cache)

    rows: list[str] = []
    for spec in SPECS:
        pins = inputs.pins.get(spec.id, [])
        if not pins and not spec.always and spec.id not in inputs.distributions:
            raise ValueError(f"{spec.id} has no build-input pin; drop the spec or add its pin")
        rows.extend(component_rows(root, spec, pins, cache))
    rows.extend(release_rows(root, cache, vendored_spdx))

    manifest = root / sources.NOTICES_DIR / sources.MANIFEST_NAME
    manifest.write_text(render_manifest(rows), encoding="utf-8")
    print(f"Wrote {manifest}")


if __name__ == "__main__":
    main()
