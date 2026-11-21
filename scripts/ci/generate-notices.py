#!/usr/bin/env python3
"""Generate the third-party notice bundle from the real dependency graph and pinned sources.

`THIRD_PARTY_LICENSES.md` is rendered from `scripts/ci/notices/THIRD_PARTY_LICENSES.md.in`:
every version in the prose comes from a build input, the native inventory comes from the
digests the build verifies, and the license texts come from the dependency graph and the
vendored upstream texts. `--check` fails when the committed file is stale, which is what stops
a dependency bump from shipping notices that describe the previous release.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import subprocess
import sys
from dataclasses import dataclass, field
from pathlib import Path

import notices_sources as sources

SPDX_TOKEN = re.compile(r"[A-Za-z0-9.+-]+")
LICENSE_FILE_PREFIXES = ("license", "copying", "notice", "unlicense", "patents")
FENCE = re.compile(r"^(`{3,})", re.MULTILINE)
PLACEHOLDER = re.compile(r"\{\{([a-z0-9_.-]+)\}\}")


@dataclass(frozen=True)
class Text:
    """One distinct license text, referenced by its digest."""

    sha256: str
    name: str
    content: str

    @property
    def label(self) -> str:
        """Stable identifier used to reference the text from inventories."""
        return f"T-{self.sha256[:12]}"


@dataclass
class Crate:
    """One third-party Rust crate in the shipped dependency graph."""

    name: str
    version: str
    expression: str
    texts: list[Text] = field(default_factory=list)


@dataclass
class Bundle:
    """Everything the template interpolates."""

    inputs: sources.BuildInputs
    components: dict[str, sources.Component]
    texts_by_component: dict[str, list[tuple[sources.LicenseText, str]]]
    crates: list[Crate]
    graphs: list[sources.Graph]
    spdx: dict[str, sources.LicenseText]
    offers: list[sources.Offer]
    offer_term: str = ""
    offer_contact: str = ""
    vcpkg: sources.Vcpkg | None = None
    texts: dict[str, Text] = field(default_factory=dict)
    component_uses: dict[str, list[str]] = field(default_factory=dict)

    def text(self, name: str, content: bytes) -> Text:
        """Register a text once, keyed by its normalised digest."""
        record = text_from(name, content)
        return self.texts.setdefault(record.sha256, record)


def normalized(content: bytes) -> tuple[str, str]:
    """Decode a license text and normalise line endings so a checkout cannot change its digest."""
    text = content.decode("utf-8").replace("\r\n", "\n").replace("\r", "\n")
    return text, hashlib.sha256(text.encode("utf-8")).hexdigest()


def text_from(name: str, content: bytes) -> Text:
    """Build a text record from raw bytes, keyed by its normalised digest."""
    body, sha256 = normalized(content)
    return Text(sha256, name, body)


def run_cargo(arguments: list[str], root: Path) -> str:
    """Run cargo and turn its failure into one actionable line."""
    result = subprocess.run(["cargo", *arguments], cwd=root, text=True, capture_output=True, check=False)
    if result.returncode != 0:
        detail = (result.stderr or result.stdout).strip().splitlines()
        raise ValueError(f"cargo {' '.join(arguments[:3])} failed: {detail[-1] if detail else result.returncode}")
    return result.stdout


def crate_inventory(root: Path, graphs: list[sources.Graph]) -> list[tuple[str, str]]:
    """Collect the unique (name, version) pairs the shipped graphs resolve."""
    found: set[tuple[str, str]] = set()
    for graph in graphs:
        arguments = [
            "tree",
            "--locked",
            "--manifest-path",
            graph.manifest,
            "--edges",
            "normal,build",
            "--target",
            "all",
            "--prefix",
            "none",
            "--format",
            "{p}",
        ]
        if graph.features:
            arguments += ["--features", graph.features]
        for line in run_cargo(arguments, root).splitlines():
            match = re.match(r"^(?P<name>\S+) v(?P<version>\S+?)(?: \(\*\))?$", line.strip())
            if match:
                found.add((match.group("name"), match.group("version")))
    return sorted(found)


def package_index(root: Path, graphs: list[sources.Graph]) -> dict[tuple[str, str], dict[str, object]]:
    """Map each resolved crate to its manifest path and license expression."""
    index: dict[tuple[str, str], dict[str, object]] = {}
    workspaces: set[str] = set()
    for graph in graphs:
        metadata = json.loads(
            run_cargo(["metadata", "--locked", "--format-version", "1", "--manifest-path", graph.manifest], root)
        )
        workspace = metadata["workspace_root"]
        if workspace in workspaces:
            continue
        workspaces.add(workspace)
        for package in metadata["packages"]:
            index[(package["name"], package["version"])] = package
    return index


def registry_directory(name: str, version: str) -> Path:
    """Locate a crate's unpacked source when cargo metadata omitted it (an unenabled optional dep)."""
    cargo_home = Path(os.environ.get("CARGO_HOME", Path.home() / ".cargo"))
    matches = sorted((cargo_home / "registry" / "src").glob(f"*/{name}-{version}"))
    if len(matches) != 1:
        raise ValueError(f"no unpacked source for {name} {version}; run cargo fetch --locked")
    return matches[0]


def license_files(directory: Path) -> list[Path]:
    """Every license-bearing file a crate ships, in a stable order."""
    return [
        path
        for path in sorted(directory.iterdir())
        if path.is_file() and path.name.lower().startswith(LICENSE_FILE_PREFIXES)
    ]


def spdx_ids(expression: str) -> list[str]:
    """License ids a declared SPDX expression names, without operators or exceptions."""
    cleaned = re.sub(r"\s+WITH\s+\S+", "", expression)
    ids = [token for token in SPDX_TOKEN.findall(cleaned) if token not in {"AND", "OR"}]
    return sorted(dict.fromkeys(ids))


def collect_crates(root: Path, graphs: list[sources.Graph], spdx: dict[str, sources.LicenseText]) -> list[Crate]:
    """Read each crate's shipped license text, falling back to the canonical SPDX text."""
    index = package_index(root, graphs)
    crates = []
    for name, version in crate_inventory(root, graphs):
        package = index.get((name, version))
        if package is None:
            directory = registry_directory(name, version)
            expression = ""
        else:
            directory = Path(str(package["manifest_path"])).parent
            expression = str(package.get("license") or "")
        files = license_files(directory)
        declared = package.get("license_file") if package else None
        if declared:
            declared_path = directory / str(declared)
            if declared_path.is_file() and declared_path not in files:
                files.append(declared_path)
        crate = Crate(name, version, expression)
        for path in files:
            crate.texts.append(text_from(path.name, path.read_bytes()))
        if not files:
            missing = [license_id for license_id in spdx_ids(expression) if license_id not in spdx]
            if not expression or missing:
                raise ValueError(
                    f"{name} {version} ships no license file and its expression {expression!r} is not covered "
                    f"by the vendored SPDX texts (missing: {', '.join(missing) or 'none'}); "
                    "run scripts/ci/vendor-notice-sources.py"
                )
            for license_id in spdx_ids(expression):
                crate.texts.append(text_from(f"{license_id}.txt", (root / spdx[license_id].path).read_bytes()))
        crates.append(crate)
    return crates


def fence(content: str) -> str:
    """Fence a license text without letting backticks inside it close the block."""
    longest = max((len(match.group(1)) for match in FENCE.finditer(content)), default=0)
    marker = "`" * max(3, longest + 1)
    return f"{marker}text\n{content.rstrip()}\n{marker}"


def markdown_cell(value: str) -> str:
    """Escape a value for a Markdown table cell."""
    return value.replace("|", "\\|").replace("\n", " ")


def plural(count: int, singular: str, plural_form: str | None = None) -> str:
    """Render a count with its noun."""
    return f"{count} {singular if count == 1 else plural_form or singular + 's'}"


PORT_NAMES = {"libaom": "aom", "libdav1d": "dav1d"}


def channel_versions(bundle: Bundle, channel: sources.Channel) -> dict[str, str]:
    """Resolve the exact version each channel builds, which is not always the union of pins."""
    resolved = {}
    for component in channel.components:
        if channel.build == "vcpkg-port" and bundle.vcpkg is not None:
            port = next(
                (item for item in bundle.vcpkg.ports if item.name == PORT_NAMES.get(component, component)), None
            )
            if port is not None:
                resolved[component] = port.version
                continue
        versions = bundle.inputs.versions(component)
        if not versions:
            continue
        # The encoder-enabled Elixir archive is the only consumer of the newer libaom pin; the
        # decode-only builds use the older one. ~keep
        resolved[component] = versions[-1] if channel.id == "elixir-macos-arm64" else versions[0]
    return resolved


def component_versions(bundle: Bundle, channel: sources.Channel) -> str:
    """Render a channel's copyleft components with the versions that channel builds."""
    versions = channel_versions(bundle, channel)
    return ", ".join(f"{component} {version}" for component, version in versions.items())


def build_description(bundle: Bundle, channel: sources.Channel) -> str:
    """Describe how a channel builds its copyleft components, including any patches."""
    if channel.build == "vcpkg-port":
        ports = {port.name: port for port in (bundle.vcpkg.ports if bundle.vcpkg else ())}
        details = []
        for component in channel.components:
            port = ports.get(PORT_NAMES.get(component, component))
            if port is None:
                details.append(f"{component} (port metadata missing)")
            elif port.patches:
                details.append(f"{component} patched ({', '.join(f'`{patch}`' for patch in port.patches)})")
            else:
                details.append(f"{component} unmodified")
        return f"vcpkg port set at baseline `{channel.revision}`: " + "; ".join(details)
    if channel.build == "upstream-tarball-static":
        return (
            f"unmodified upstream release tarballs, statically linked by `{channel.build_script}` "
            f"at release {channel.revision}"
        )
    return f"unmodified upstream release tarballs via `{channel.build_script}` at release {channel.revision}"


def corresponding_source(bundle: Bundle) -> str:
    """Build the per-channel Corresponding Source table.

    Each cell resolves one version and then that version's own pin or port, so the version,
    URL and digest always describe the same release.
    """
    rows = ["| Artifact | Copyleft components | Build and patches | Upstream source |", "| --- | --- | --- | --- |"]
    for channel in sorted(bundle.inputs.channels, key=lambda item: item.id):
        versions = channel_versions(bundle, channel)
        sources = []
        for component in channel.components:
            port_name = PORT_NAMES.get(component, component)
            port = None
            if channel.build == "vcpkg-port" and bundle.vcpkg is not None:
                port = next((item for item in bundle.vcpkg.ports if item.name == port_name), None)
            if port is not None:
                digest = f" ({port.digest})" if port.digest else ""
                sources.append(f"{component}: <{port.source}>{digest}")
                continue
            pin = bundle.inputs.pin(component, versions[component])
            sources.append(f"{component}: <{pin.url}> (sha256 {pin.sha256})")
        rows.append(
            f"| {channel.artifact} | {component_versions(bundle, channel)} | "
            f"{build_description(bundle, channel)} | {'; '.join(sources) or '—'} |"
        )
    return "\n".join(rows)


def inline_offer(bundle: Bundle, channel: sources.Channel) -> str:
    """Render the written offer for a channel that ships no separate offer file."""
    versions = channel_versions(bundle, channel)
    components = ", ".join(f"{component} {version}" for component, version in versions.items())
    return (
        f"**{channel.artifact}.** Xberg offers, valid for {bundle.offer_term} from the date this release was "
        "distributed, to give anyone who possesses this object code a complete copy of its Corresponding "
        "Source on a durable medium suitable for interchanging data, for a charge no more than the cost of "
        "physically performing source distribution (GNU GPL version 3, section 6(b)). The Corresponding "
        f"Source is the complete machine-readable source for {components}, including every patch and build "
        "script named in the table above, at the release that produced this archive. To obtain it, email "
        f'{bundle.offer_contact} with the subject "{channel.artifact} corresponding source" and the release '
        "version. This offer covers the LGPL-3.0-or-later components on the same terms, satisfying "
        "LGPL-3.0 section 4(d)."
    )


def written_offers(bundle: Bundle) -> str:
    """Render the written offers of Corresponding Source and the channels they cover."""
    parts = []
    covered = set()
    if bundle.offers:
        rows = ["| Offer | Covers |", "| --- | --- |"]
        for offer in sorted(bundle.offers, key=lambda item: item.path):
            basename = offer.path.rsplit("/", 1)[-1]
            shipped = (
                f" (shipped in the archive as `{offer.shipped_as}`)"
                if offer.shipped_as and offer.shipped_as != basename
                else ""
            )
            rows.append(f"| [`{offer.path}`]({offer.path}){shipped} | {', '.join(offer.channels)} |")
            covered.update(offer.channels)
        parts.append("\n".join(rows))
    for channel in sorted(bundle.inputs.channels, key=lambda item: item.id):
        if not channel.needs_offer or channel.id in covered:
            continue
        parts.append(inline_offer(bundle, channel))
    if not parts:
        return "No channel in this bundle conveys GPL code or a patched copyleft build."
    return "\n\n".join(parts)


def text_labels(bundle: Bundle, component_id: str) -> str:
    """Reference a component's license texts by their stable labels."""
    labels = {
        bundle.texts[record.sha256].label
        for record, _ in bundle.texts_by_component.get(component_id, [])
        if record.sha256 in bundle.texts
    }
    return ", ".join(f"`{label}`" for label in sorted(labels))


def native_table(bundle: Bundle) -> str:
    """Build the per-component inventory table from pins and distribution packages."""
    rows = ["| Component | License | Version and provenance | License texts |", "| --- | --- | --- | --- |"]
    assets = {font.component: [] for font in bundle.inputs.fonts}
    for font in bundle.inputs.fonts:
        assets[font.component].append(font)
    for component_id in sorted(bundle.components):
        component = bundle.components[component_id]
        if component_id in assets:
            for font in sorted(assets[component_id], key=lambda item: item.asset):
                rows.append(
                    f"| {component.summary} (`{component_id}`) | {component.license} | "
                    f"embedded asset `{font.asset}` at sha256 `{font.sha256[:16]}` | {text_labels(bundle, component_id)} |"
                )
            continue
        versions: dict[str, list[str]] = {}
        for pin in bundle.inputs.pins.get(component_id, []):
            versions.setdefault(pin.version, []).append(pin.url)
        for version in sorted(versions, key=sources.version_key):
            urls = sorted({url for url in versions[version] if url})
            provenance = f"[pinned source]({urls[0]})" if urls else "built from the vendored source archive"
            rows.append(
                f"| {component.summary} (`{component_id}`) | {component.license} | {version} — {provenance} | "
                f"{text_labels(bundle, component_id)} |"
            )
        for distribution in sorted(bundle.inputs.distributions.get(component_id, []), key=lambda item: item.provider):
            rows.append(
                f"| {component.summary} (`{component_id}`) | {component.license} | "
                f"{distribution.provider} package `{distribution.package}` | {text_labels(bundle, component_id)} |"
            )
        if not versions and not bundle.inputs.distributions.get(component_id):
            rows.append(
                f"| {component.summary} (`{component_id}`) | {component.license} | required beside LGPL-3.0 | "
                f"{text_labels(bundle, component_id)} |"
            )
    return "\n".join(rows)


def font_table(bundle: Bundle) -> str:
    """Build the embedded-font inventory from the bytes that ship."""
    rows = ["| Font | Asset | sha256 | License |", "| --- | --- | --- | --- |"]
    for font in sorted(bundle.inputs.fonts, key=lambda item: item.asset):
        component = bundle.components[font.component]
        rows.append(f"| {font.family} | `{font.asset}` | `{font.sha256[:16]}` | {component.license} |")
    return "\n".join(rows)


def text_sections(bundle: Bundle) -> str:
    """Render every distinct license text once, listing what it covers."""
    users: dict[str, list[str]] = {}
    for crate in bundle.crates:
        for text in crate.texts:
            users.setdefault(text.sha256, []).append(f"{crate.name} {crate.version}")
    for sha256, component_ids in bundle.component_uses.items():
        users.setdefault(sha256, []).extend(component_ids)
    sections = []
    for sha256, text in sorted(bundle.texts.items(), key=lambda item: item[0]):
        covered = sorted(set(users.get(sha256, [])))
        sections.append(
            f"### {text.label}\n\nSource: `{text.name}`. Applies to {plural(len(covered), 'entry', 'entries')}.\n"
        )
        sections.append(fence(text.content))
        sections.append("\nApplies to: " + ", ".join(covered) + "\n")
    return "\n".join(sections)


def resolve_versions(bundle: Bundle, name: str) -> str:
    """Render a component's distinct pinned versions for prose."""
    versions = bundle.inputs.versions(name)
    if not versions:
        raise ValueError(f"no pinned version for {name}")
    if len(versions) == 1:
        return versions[0]
    return ", ".join(versions[:-1]) + " and " + versions[-1]


def values(bundle: Bundle) -> dict[str, str]:
    """Every value the template may interpolate."""
    table = {
        "native_components": native_table(bundle),
        "corresponding_source": corresponding_source(bundle),
        "written_offers": written_offers(bundle),
        "fonts": font_table(bundle),
        "license_texts": text_sections(bundle),
        "rust_crate_count": str(len(bundle.crates)),
        "rust_text_count": str(len({text.sha256 for crate in bundle.crates for text in crate.texts})),
        "graph_count": str(len(bundle.graphs)),
        "graph_list": "\n".join(f"- {graph.label}" for graph in bundle.graphs),
        "native_component_count": str(len(bundle.components)),
        "pdfium_revision": bundle.inputs.pdfium_revision,
    }
    for component_id in bundle.components:
        versions = bundle.inputs.versions(component_id)
        if versions:
            table[f"versions.{component_id}"] = resolve_versions(bundle, component_id)
        providers = sorted(
            {distribution.provider for distribution in bundle.inputs.distributions.get(component_id, [])}
        )
        table[f"providers.{component_id}"] = ", ".join(providers)
    return table


def render_template(template: str, mapping: dict[str, str]) -> str:
    """Replace `{{name}}` placeholders, refusing unknown names so a typo cannot ship."""

    def replace(match: re.Match[str]) -> str:
        name = match.group(1)
        if name not in mapping:
            raise ValueError(f"template placeholder {{{{{name}}}}} has no value")
        return mapping[name]

    return PLACEHOLDER.sub(replace, template)


def build(root: Path, *, rust_inventory: Path | None = None, native_inputs: Path | None = None) -> Bundle:
    """Assemble the bundle from the repository (or from the test seams)."""
    if native_inputs:
        payload = json.loads(native_inputs.read_text(encoding="utf-8"))
        inputs = sources.BuildInputs(
            pins={
                component: [sources.Pin(**pin) for pin in pins] for component, pins in payload.get("pins", {}).items()
            },
            distributions={
                component: [sources.Distribution(**item) for item in items]
                for component, items in payload.get("distributions", {}).items()
            },
            fonts=[sources.Font(**item) for item in payload.get("fonts", [])],
            pdfium_revision=payload.get("pdfium_revision", ""),
            channels=[sources.Channel(**item) for item in payload.get("channels", [])],
            version=payload.get("version", ""),
        )
    else:
        inputs = sources.build_inputs(root)
        sources.attach_channels(root, inputs)
    graphs = sources.read_graphs(root)
    components = sources.load_manifest(root)
    spdx = sources.load_spdx(root)
    offers = sources.load_offers(root)
    sources.assert_offers(root, inputs, offers)
    texts_by_component = sources.resolve_texts(inputs, components)
    crates = read_rust_inventory(root, rust_inventory, spdx) if rust_inventory else collect_crates(root, graphs, spdx)
    term, contact = sources.offer_terms(root, offers)
    bundle = Bundle(
        inputs, components, texts_by_component, crates, graphs, spdx, offers, term, contact, sources.load_vcpkg(root)
    )
    for component_id, entries in texts_by_component.items():
        for record, _ in entries:
            text = bundle.text(record.name, (root / record.path).read_bytes())
            bundle.component_uses.setdefault(text.sha256, []).append(f"`{component_id}`")
    for crate in crates:
        for text in crate.texts:
            bundle.texts.setdefault(text.sha256, text)
    return bundle


def read_rust_inventory(root: Path, path: Path, spdx: dict[str, sources.LicenseText]) -> list[Crate]:
    """Read a recorded crate inventory (the offline seam used by the drift test)."""
    crates = []
    for item in json.loads(path.read_text(encoding="utf-8"))["crates"]:
        crate = Crate(item["name"], item["version"], item.get("license", ""))
        for entry in item.get("texts", []):
            crate.texts.append(text_from(entry["name"], (root / entry["path"]).read_bytes()))
        if not crate.texts:
            for license_id in spdx_ids(crate.expression):
                crate.texts.append(text_from(f"{license_id}.txt", (root / spdx[license_id].path).read_bytes()))
        crates.append(crate)
    return crates


def generated(root: Path, **kwargs: object) -> str:
    """Render the committed notice file for a repository root."""
    bundle = build(root, **kwargs)
    template = (root / sources.NOTICES_DIR / sources.TEMPLATE_NAME).read_text(encoding="utf-8")
    return render_template(template, values(bundle))


def write(root: Path, content: str) -> Path:
    """Write the notice bundle."""
    output = root / sources.OUTPUT_NAME
    output.write_bytes(content.encode("utf-8"))
    return output


def check(root: Path, content: str) -> None:
    """Fail when the committed bundle differs from what the build inputs produce."""
    output = root / sources.OUTPUT_NAME
    if not output.is_file():
        raise ValueError(f"{output} does not exist; run scripts/ci/generate-notices.py")
    current = output.read_bytes()
    expected = content.encode("utf-8")
    if current == expected:
        print(f"{output} is current ({len(content.encode('utf-8'))} bytes)")
        return
    expected_lines = content.splitlines()
    actual_lines = current.decode("utf-8").splitlines()
    for number, (expected, actual) in enumerate(zip(expected_lines, actual_lines, strict=False), start=1):
        if expected != actual:
            raise ValueError(
                f"{output} is stale at line {number}:\n  committed: {actual[:160]}\n  generated: {expected[:160]}\n"
                "run scripts/ci/generate-notices.py"
            )
    raise ValueError(
        f"{output} is stale: generated {len(expected_lines)} lines, committed {len(actual_lines)}; "
        "run scripts/ci/generate-notices.py"
    )


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parents[2])
    parser.add_argument("--check", action="store_true", help="fail when the committed bundle is stale")
    parser.add_argument("--rust-inventory", type=Path, help=argparse.SUPPRESS)
    parser.add_argument("--native-inputs", type=Path, help=argparse.SUPPRESS)
    args = parser.parse_args()
    root = args.root.resolve()
    try:
        content = generated(root, rust_inventory=args.rust_inventory, native_inputs=args.native_inputs)
        if args.check:
            check(root, content)
        else:
            print(f"Wrote {write(root, content)}")
    except (OSError, ValueError, subprocess.SubprocessError, json.JSONDecodeError) as error:
        print(f"::error::{error}", file=sys.stderr)
        sys.exit(1)


if __name__ == "__main__":
    main()
