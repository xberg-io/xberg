"""Read the pinned build inputs that decide which third-party code ships in Xberg artifacts.

The notice bundle must state the versions and provenance the build actually used, so nothing in
this module types a version: every pin is parsed out of the build input that selects it --
`scripts/ci/build-macos-heif-deps.sh`, the Dockerfiles, the OCR build scripts, the crates that
vendor native sources, and the pinned asset checksum manifests. `generate-notices.py` renders
those pins and `vendor-notice-sources.py` fetches their license texts, so a bumped pin either
finds its vendored text or fails the freshness check.
"""

from __future__ import annotations

import hashlib
import re
import tarfile
import tomllib
import zipfile
from dataclasses import dataclass, field
from pathlib import Path, PurePosixPath

NOTICES_DIR = Path("scripts/ci/notices")
MANIFEST_NAME = "SOURCES.toml"
TEMPLATE_NAME = "THIRD_PARTY_LICENSES.md.in"
OUTPUT_NAME = "THIRD_PARTY_LICENSES.md"

DOCKER_URL = re.compile(
    r"(?P<component>libheif|libde265|libaom|x265)[-_]v?(?P<version>\d+(?:\.\d+)+)\.(?:tar\.gz|tgz|zip)"
)
SHA256 = re.compile(r"\b[0-9a-f]{64}\b")
ASSIGNMENT = re.compile(r"^(?P<name>[A-Z][A-Z0-9_]*)=[\"']?(?P<value>[^\"'\s]+)[\"']?$")
HEX_DIGEST = re.compile(r"^(?P<digest>[0-9a-f]{64})\s+(?P<name>\S+)$")
CONST_STR = re.compile(r'const (?P<name>[A-Z0-9_]+): &str = "(?P<value>[^"]+)";')

# apk/apt package names that carry each bundled component. The Dockerfile package list is the
# only record of what the final image installs, so these map the two spellings and nothing else.
DISTRO_PACKAGES = {
    "libheif": ("libheif", "libheif-dev"),
    "libde265": ("libde265", "libde265-dev", "libheif-libde265"),
    "libaom": ("aom", "aom-dev", "libaom", "libheif-aom"),
    "libdav1d": ("dav1d", "dav1d-dev", "libdav1d", "libdav1d7", "libheif-dav1d"),
    "onnxruntime": ("onnxruntime", "onnxruntime-dev"),
    "tesseract": ("tesseract", "libtesseract", "libtesseract-dev", "tesseract-ocr"),
    "leptonica": ("libleptonica", "libleptonica-dev"),
}

# component id, display family, asset filename prefix, license notices that cover it.
FONT_FAMILIES = (
    ("dejavu", "DejaVu Sans", "DejaVuSans", ("LICENSE-DejaVu",)),
    (
        "droid-sans-fallback",
        "Droid Sans Fallback",
        "DroidSansFallback",
        ("LICENSE-fallback-fonts", "spdx/Apache-2.0.txt"),
    ),
    ("noto-emoji", "Noto Emoji", "NotoEmoji", ("LICENSE-fallback-fonts", "spdx/OFL-1.1.txt")),
)

FONT_NOTICE_DIR = Path("crates/xberg-native-pdf/src/fonts/assets")
LIBWPD_VENDOR_DIR = Path("crates/xberg-libwpd/vendor")
TESSERACT_BUILD = Path("crates/xberg-tesseract/build.rs")
OCR_BUILD = Path("scripts/ci/install-system-deps/build-ocr-linux.sh")
PDFIUM_CHECKSUMS = Path("scripts/pdfium-runtime-checksums.sha256")
PDFIUM_ACTION = Path(".github/actions/download-pdfium/action.yml")
ONNX_CHECKSUMS = Path("scripts/ci/actions/setup-onnx-runtime/checksums.sha256")


@dataclass(frozen=True)
class Graph:
    """One dependency graph a released artifact is built from."""

    label: str
    manifest: str
    features: str
    target: str


@dataclass(frozen=True)
class Pin:
    """One resolved upstream artifact: a version with the digest the build verifies."""

    component: str
    version: str
    sha256: str | None
    url: str
    origin: str


@dataclass(frozen=True)
class Distribution:
    """A component shipped by a distribution package instead of a pinned source build."""

    component: str
    provider: str
    package: str


@dataclass(frozen=True)
class Font:
    """A font embedded through `include_bytes!`, identified by the bytes that ship."""

    component: str
    family: str
    asset: str
    sha256: str


@dataclass(frozen=True)
class Channel:
    """One published artifact and how its copyleft components were built."""

    id: str
    artifact: str
    build: str
    components: tuple[str, ...]
    build_script: str
    revision: str
    patched: bool

    @property
    def needs_offer(self) -> bool:
        """A channel needs a written offer when it conveys GPL code or a patched copyleft build."""
        return self.patched or "x265" in self.components


@dataclass(frozen=True)
class VcpkgPort:
    """One vcpkg port the Windows build compiles, with the patches it applies."""

    name: str
    version: str
    source: str
    digest: str
    patches: tuple[str, ...]


@dataclass(frozen=True)
class Vcpkg:
    """The pinned vcpkg baseline and the ports that produce the Windows codec DLLs."""

    baseline: str
    ports: tuple[VcpkgPort, ...]


@dataclass(frozen=True)
class Offer:
    """A written offer of Corresponding Source, and the channels it covers."""

    path: str
    sha256: str
    channels: tuple[str, ...]


@dataclass
class BuildInputs:
    """Every pin, distro package and font the shipped build inputs select."""

    pins: dict[str, list[Pin]] = field(default_factory=dict)
    distributions: dict[str, list[Distribution]] = field(default_factory=dict)
    fonts: list[Font] = field(default_factory=list)
    pdfium_revision: str = ""
    channels: list[Channel] = field(default_factory=list)
    version: str = ""

    def add(self, pin: Pin) -> None:
        entries = self.pins.setdefault(pin.component, [])
        if pin not in entries:
            entries.append(pin)

    def add_distribution(self, distribution: Distribution) -> None:
        entries = self.distributions.setdefault(distribution.component, [])
        if distribution not in entries:
            entries.append(distribution)

    def versions(self, component: str) -> list[str]:
        """Distinct pinned versions for a component, ordered oldest to newest."""
        return sorted({pin.version for pin in self.pins.get(component, [])}, key=version_key)

    def upstream_url(self, component: str) -> str:
        """The first pinned release URL for a component (all of them for a source build)."""
        urls = sorted({pin.url for pin in self.pins.get(component, []) if pin.url})
        return urls[0] if urls else ""

    def channel(self, channel_id: str) -> Channel:
        """Look up a channel by id."""
        for channel in self.channels:
            if channel.id == channel_id:
                return channel
        raise ValueError(f"no build channel {channel_id}")


@dataclass(frozen=True)
class LicenseText:
    """A license file recorded in the notice manifest, with its exact bytes pinned."""

    name: str
    path: str
    sha256: str


@dataclass(frozen=True)
class ManifestPin:
    """A manifest row matching one pinned artifact digest."""

    version: str
    url: str
    sha256: str | None
    texts: tuple[LicenseText, ...]


@dataclass(frozen=True)
class Component:
    """A non-Rust component and the license texts that cover it."""

    id: str
    license: str
    summary: str
    texts: tuple[LicenseText, ...]
    pins: dict[str | None, ManifestPin]
    always: bool = False


def version_key(version: str) -> tuple[int, ...]:
    """Sort dotted numeric versions numerically, falling back to zero for non-numeric tags."""
    return tuple(int(part) if part.isdigit() else 0 for part in re.split(r"[.+-]", version))


def digest(path: Path) -> str:
    """Return the sha256 of a file."""
    return hashlib.sha256(path.read_bytes()).hexdigest()


def read_graphs(root: Path, *, path: str = "scripts/ci/license-graphs.txt") -> list[Graph]:
    """Parse the graph table shared with `check-licenses.sh`."""
    graphs = []
    for number, row in enumerate((root / path).read_text(encoding="utf-8").splitlines(), start=1):
        if not row or row.startswith("#"):
            continue
        parts = row.split("|")
        if len(parts) != 4:
            raise ValueError(f"{path}:{number}: expected label|manifest|features|target, got {row!r}")
        graphs.append(Graph(*parts))
    if not graphs:
        raise ValueError(f"{path} names no graph")
    return graphs


def _assignments(path: Path) -> dict[str, list[str]]:
    values: dict[str, list[str]] = {}
    for line in path.read_text(encoding="utf-8").splitlines():
        match = ASSIGNMENT.match(line.strip())
        if match:
            values.setdefault(match.group("name"), []).append(match.group("value"))
    return values


def _heif_script_pins(root: Path, inputs: BuildInputs) -> None:
    script = root / "scripts/ci/build-macos-heif-deps.sh"
    values = _assignments(script)
    for name, versions in values.items():
        if not name.endswith("_VERSION"):
            continue
        base = name.removesuffix("_VERSION")
        digests = values.get(f"{base}_SHA256", [])
        urls = values.get(f"{base}_URL", [])
        if len(digests) != len(versions) or len(urls) not in (1, len(versions)):
            raise ValueError(f"{script}: {name} has no matching digest/URL for every version")
        for index, version in enumerate(versions):
            sha256 = digests[index]
            url = urls[index if len(urls) > 1 else 0]
            inputs.add(Pin(base.lower(), version, sha256, url.replace(f"${name}", version), script.as_posix()))


def _docker_pins(root: Path, inputs: BuildInputs) -> None:
    for dockerfile in sorted((root / "docker").glob("Dockerfile*")):
        lines = dockerfile.read_text(encoding="utf-8").splitlines()
        for index, line in enumerate(lines):
            match = DOCKER_URL.search(line)
            if not match:
                continue
            digests = [found for row in lines[index : index + 4] for found in SHA256.findall(row)]
            if not digests:
                raise ValueError(f"{dockerfile}:{index + 1}: no digest beside {match.group(0)}")
            url = next((token for token in line.split() if match.group(0) in token), match.group(0))
            inputs.add(
                Pin(match.group("component").lower(), match.group("version"), digests[0], url, dockerfile.as_posix())
            )


def _docker_arg(path: Path, name: str) -> str | None:
    match = re.search(rf"(?m)^ARG {name}=(\S+)$", path.read_text(encoding="utf-8"))
    return match.group(1) if match else None


def _package_tokens(lines: list[str]) -> list[str]:
    tokens: list[str] = []
    collecting = False
    for line in lines:
        stripped = line.strip()
        if not collecting:
            collecting = bool(re.search(r"\b(apk add|apt-get install)\b", stripped))
            if not collecting:
                continue
            stripped = re.split(r"\b(?:apk add|apt-get install)\b", stripped, maxsplit=1)[1]
        tokens.extend(token for token in stripped.replace("\\", " ").split() if not token.startswith(("-", "&&")))
        collecting = stripped.endswith("\\")
    return tokens


def _docker_base(path: Path, inputs: BuildInputs) -> None:
    lines = path.read_text(encoding="utf-8").splitlines()
    stages = [index for index, line in enumerate(lines) if line.startswith("FROM ")]
    if not stages:
        raise ValueError(f"{path} has no FROM line")
    name, _, tag = lines[stages[-1]].split()[1].split("@", 1)[0].partition(":")
    if name == "alpine":
        provider = f"Alpine {tag}"
    elif name == "debian":
        provider = f"Debian {tag}"
    else:
        return
    # Only the final stage ships: packages installed in a builder are build-time inputs.
    packages = _package_tokens(lines[stages[-1] + 1 :])
    for component, spellings in DISTRO_PACKAGES.items():
        for package in packages:
            base = re.split(r"[=:]", package, maxsplit=1)[0]
            if base in spellings:
                inputs.add_distribution(Distribution(component, provider, base))
                break


def _ocr_pins(root: Path, inputs: BuildInputs) -> None:
    script = root / OCR_BUILD
    pattern = re.compile(
        r"ocr_build_source (?P<component>leptonica|tesseract) (?P<version>\S+) \\\n\s*(?P<url>\S+) \\\n\s*(?P<sha256>[0-9a-f]{64})"
    )
    for match in pattern.finditer(script.read_text(encoding="utf-8")):
        inputs.add(
            Pin(
                match.group("component"),
                match.group("version"),
                match.group("sha256"),
                match.group("url"),
                OCR_BUILD.as_posix(),
            )
        )


def _tesseract_pins(root: Path, inputs: BuildInputs) -> None:
    constants = dict(CONST_STR.findall((root / TESSERACT_BUILD).read_text(encoding="utf-8")))
    for component, upper, template in (
        ("leptonica", "LEPTONICA", "https://codeload.github.com/DanBloomberg/leptonica/zip/"),
        ("tesseract", "TESSERACT", "https://codeload.github.com/tesseract-ocr/tesseract/zip/"),
    ):
        inputs.add(
            Pin(
                component,
                constants[f"{upper}_VERSION"],
                constants[f"{upper}_SHA256"],
                template + constants[f"{upper}_REVISION"],
                TESSERACT_BUILD.as_posix(),
            )
        )


def _libwpd_pins(root: Path, inputs: BuildInputs) -> None:
    build = (root / "crates/xberg-libwpd/build.rs").read_text(encoding="utf-8")
    constants = dict(re.findall(r'const ([A-Z0-9_]+): &str = "([0-9a-f]{64})";', build))
    provenance = (root / LIBWPD_VENDOR_DIR / "PROVENANCE.md").read_text(encoding="utf-8")
    for component, upper in (("librevenge", "LIBREVENGE"), ("libwpd", "LIBWPD")):
        archives = sorted((root / LIBWPD_VENDOR_DIR).glob(f"{component}-*.tar.gz"))
        if len(archives) != 1:
            raise ValueError(f"{LIBWPD_VENDOR_DIR} must hold exactly one {component} archive")
        version = archives[0].name.removesuffix(".tar.gz").removeprefix(f"{component}-")
        if digest(archives[0]) != constants.get(f"{upper}_SHA256"):
            raise ValueError(f"{archives[0]} does not match {upper}_SHA256 in build.rs")
        source = re.search(
            rf"{component} {re.escape(version)}\n\n- Archive: `[^`]+`\n- Source: <(?P<url>[^>]+)>", provenance
        )
        inputs.add(
            Pin(
                component,
                version,
                constants[f"{upper}_SHA256"],
                source.group("url") if source else "",
                LIBWPD_VENDOR_DIR.as_posix(),
            )
        )
    boost = root / LIBWPD_VENDOR_DIR / "boost-subset.tar.gz"
    boost_sha = constants.get("BOOST_SUBSET_SHA256")
    if digest(boost) != boost_sha:
        raise ValueError(f"{boost} does not match BOOST_SUBSET_SHA256 in build.rs")
    version = re.search(r"- Version: ([0-9.]+)", provenance)
    if version is None:
        raise ValueError(f"{LIBWPD_VENDOR_DIR}/PROVENANCE.md names no Boost version")
    inputs.add(Pin("boost", version.group(1), boost_sha, "", LIBWPD_VENDOR_DIR.as_posix()))


def _pdfium_pin(root: Path, inputs: BuildInputs) -> None:
    action = (root / PDFIUM_ACTION).read_text(encoding="utf-8")
    default = re.search(r'(?ms)pdfium-version:.*?\n\s+default: "(\d+)"', action)
    if default is None:
        raise ValueError(f"{PDFIUM_ACTION} has no pdfium-version default")
    revision = default.group(1)
    for dockerfile in sorted((root / "docker").glob("Dockerfile*")):
        arg = _docker_arg(dockerfile, "PDFIUM_VERSION")
        if arg is not None and arg != revision:
            raise ValueError(f"{dockerfile}: PDFIUM_VERSION={arg} but {PDFIUM_ACTION} pins {revision}")
    digests = {}
    for line in (root / PDFIUM_CHECKSUMS).read_text(encoding="utf-8").splitlines():
        match = HEX_DIGEST.match(line.strip())
        if match and match.group("name").startswith(f"{revision}-"):
            digests[match.group("name")] = match.group("digest")
    if not digests:
        raise ValueError(f"{PDFIUM_CHECKSUMS} has no digest for pdfium revision {revision}")
    name = min(digests)
    url = f"https://github.com/bblanchon/pdfium-binaries/releases/download/chromium/{revision}/{name.removeprefix(f'{revision}-')}"
    inputs.pdfium_revision = revision
    inputs.add(Pin("pdfium", revision, digests[name], url, PDFIUM_CHECKSUMS.as_posix()))


def _onnxruntime_pins(root: Path, inputs: BuildInputs) -> None:
    # The NuGet fallback the release jobs verify is the artifact whose digest covers every
    # platform, so it is the pin that carries the license text; the release archives are listed
    # in the same checksum file.
    for line in (root / ONNX_CHECKSUMS).read_text(encoding="utf-8").splitlines():
        match = HEX_DIGEST.match(line.strip())
        if not match or not match.group("name").startswith("microsoft.ml.onnxruntime."):
            continue
        asset = match.group("name")
        version = re.search(r"microsoft\.ml\.onnxruntime\.(\d+\.\d+\.\d+)\.nupkg$", asset)
        if version is None:
            continue
        inputs.add(
            Pin(
                "onnxruntime",
                version.group(1),
                match.group("digest"),
                f"https://api.nuget.org/v3-flatcontainer/microsoft.ml.onnxruntime/{version.group(1)}/{asset}",
                ONNX_CHECKSUMS.as_posix(),
            )
        )
    for dockerfile in sorted((root / "docker").glob("Dockerfile*")):
        version = _docker_arg(dockerfile, "ONNXRUNTIME_VERSION")
        if version is None:
            continue
        if version not in inputs.versions("onnxruntime"):
            raise ValueError(f"{dockerfile}: ONNXRUNTIME_VERSION={version} is not covered by {ONNX_CHECKSUMS}")


def _font_inputs(root: Path) -> list[Font]:
    fonts = []
    for path in sorted((root / FONT_NOTICE_DIR).glob("*.ttf")):
        for component, family, prefix, _ in FONT_FAMILIES:
            if path.name.startswith(prefix):
                fonts.append(Font(component, family, path.relative_to(root).as_posix(), digest(path)))
                break
        else:
            raise ValueError(f"{path} has no license mapping in FONT_FAMILIES")
    if not fonts:
        raise ValueError(f"no bundled fonts under {FONT_NOTICE_DIR}")
    return fonts


def _workspace_version(root: Path) -> str:
    manifest = (root / "Cargo.toml").read_text(encoding="utf-8")
    match = re.search(r'(?m)^version = "([^"]+)"', manifest)
    if match is None:
        raise ValueError("Cargo.toml has no workspace version")
    return match.group(1)


def _windows_target(root: Path) -> str:
    workflow = (root / ".github/workflows/publish.yaml").read_text(encoding="utf-8")
    match = re.search(r"\{ os: windows-2022, target: (?P<target>[\w-]+) \}", workflow)
    if match is None:
        raise ValueError("publish.yaml has no Windows CLI matrix target")
    return match.group("target")


def _windows_baseline(root: Path) -> str:
    script = root / "scripts/ci/install-system-deps/install-windows.ps1"
    match = re.search(r"\$baseline = '([0-9a-f]{40})'", script.read_text(encoding="utf-8"))
    if match is None:
        raise ValueError(f"{script} has no pinned vcpkg baseline")
    return match.group(1)


def heif_feature_set(text: str) -> tuple[str, ...]:
    """Read the libheif feature list from the Windows install script (ignoring comments)."""
    for line in text.splitlines():
        if line.lstrip().startswith("#"):
            continue
        match = re.search(r"libheif\[([a-z0-9,]+)\]", line)
        if match:
            return tuple(match.group(1).split(","))
    raise ValueError("install-windows.ps1 names no libheif feature set")


def _windows_components(script: str) -> tuple[str, ...]:
    features = set(heif_feature_set(script))
    components = ["libheif"]
    for feature, component in (("libde265", "libde265"), ("aom", "libaom"), ("dav1d", "libdav1d")):
        if feature in features:
            components.append(component)
    return tuple(components)


def _channels(root: Path, inputs: BuildInputs, vcpkg: Vcpkg | None) -> list[Channel]:
    """Derive how each published artifact builds the copyleft libraries it conveys."""
    windows_components = _windows_components(
        (root / "scripts/ci/install-system-deps/install-windows.ps1").read_text(encoding="utf-8")
    )
    windows_patched = bool(vcpkg and any(port.patches for port in vcpkg.ports if port.name in windows_components))
    elixir_components = tuple(
        component for component in ("libheif", "libde265", "libaom", "x265") if inputs.pins.get(component)
    )
    source_components = tuple(
        component for component in ("libheif", "libde265", "libaom") if inputs.pins.get(component)
    )
    version = inputs.version
    return [
        Channel(
            "windows-cli",
            f"xberg-cli-{_windows_target(root)}.zip",
            "vcpkg-port",
            windows_components,
            "scripts/ci/install-system-deps/install-windows.ps1",
            vcpkg.baseline if vcpkg else "",
            windows_patched,
        ),
        Channel(
            "elixir-macos-arm64",
            "macOS arm64 Elixir NIF archive",
            "upstream-tarball",
            elixir_components,
            "scripts/ci/build-macos-heif-deps.sh",
            version,
            False,
        ),
        Channel(
            "cli-source-builds",
            "macOS and Linux CLI archives, Docker images and manylinux wheels",
            "upstream-tarball",
            source_components,
            "scripts/ci/build-macos-heif-deps.sh, docker/Dockerfile.*",
            version,
            False,
        ),
        Channel(
            "php-macos-arm64",
            "macOS arm64 PHP extension archive",
            "upstream-tarball-static",
            tuple(component for component in source_components if component != "x265"),
            "scripts/ci/build-macos-heif-deps.sh",
            version,
            False,
        ),
    ]


def build_inputs(root: Path) -> BuildInputs:
    """Collect every pin, distribution package and font from the repository's build inputs."""
    inputs = BuildInputs()
    inputs.version = _workspace_version(root)
    _heif_script_pins(root, inputs)
    for dockerfile in sorted((root / "docker").glob("Dockerfile*")):
        _docker_pins(root, inputs)
        _docker_base(dockerfile, inputs)
    _ocr_pins(root, inputs)
    _tesseract_pins(root, inputs)
    _libwpd_pins(root, inputs)
    _pdfium_pin(root, inputs)
    _onnxruntime_pins(root, inputs)
    inputs.fonts = _font_inputs(root)
    for font in inputs.fonts:
        inputs.add(Pin(font.component, font.family, font.sha256, "", font.asset))
    return inputs


def attach_channels(root: Path, inputs: BuildInputs) -> None:
    """Add the channel table, which needs the vendored vcpkg port metadata."""
    vcpkg = load_vcpkg(root)
    baseline = _windows_baseline(root)
    if vcpkg is None or vcpkg.baseline != baseline:
        raise ValueError(f"no vendored vcpkg metadata for baseline {baseline}; run scripts/ci/vendor-notice-sources.py")
    inputs.channels = _channels(root, inputs, vcpkg)


def load_manifest(root: Path) -> dict[str, Component]:
    """Load the vendored-text manifest written by `vendor-notice-sources.py`."""
    path = root / NOTICES_DIR / MANIFEST_NAME
    data = tomllib.loads(path.read_text(encoding="utf-8"))
    components: dict[str, Component] = {}
    for row in data.get("component") or []:
        texts = tuple(_text(root, item) for item in row.get("texts") or [])
        pins = {}
        for pin in row.get("pin") or []:
            pins[pin.get("sha256")] = ManifestPin(
                version=str(pin.get("version") or ""),
                url=str(pin.get("url") or ""),
                sha256=pin.get("sha256"),
                texts=tuple(_text(root, item) for item in pin.get("texts") or []),
            )
        component = Component(row["id"], row["license"], row["summary"], texts, pins, bool(row.get("always")))
        if not component.texts and not any(pin.texts for pin in component.pins.values()):
            raise ValueError(f"{MANIFEST_NAME}: {component.id} has no license text")
        components[component.id] = component
    if not components:
        raise ValueError(f"{path} names no component")
    return components


def _text(root: Path, item: dict[str, str]) -> LicenseText:
    path = item["path"]
    content = (root / path).read_bytes()
    actual = hashlib.sha256(content).hexdigest()
    if actual != item["sha256"]:
        raise ValueError(f"{path} does not match its recorded digest; re-run vendor-notice-sources.py")
    return LicenseText(name=item["name"], path=path, sha256=actual)


def resolve_texts(inputs: BuildInputs, components: dict[str, Component]) -> dict[str, list[tuple[LicenseText, str]]]:
    """Match every build-input pin to its vendored texts, keyed by component.

    Returns the texts grouped by component id; each entry pairs the text with the version or
    distribution that uses it. A pin the manifest has no row for fails here, which is what makes
    a version bump in a build input require re-vendoring instead of silently shipping stale text.
    """
    resolved: dict[str, list[tuple[LicenseText, str]]] = {}
    for component_id, pins in inputs.pins.items():
        component = components.get(component_id)
        if component is None:
            raise ValueError(f"no notice manifest component for {component_id}; re-run vendor-notice-sources.py")
        for pin in pins:
            if pin.sha256 is None:
                texts = component.texts
            else:
                record = component.pins.get(pin.sha256)
                if record is None:
                    raise ValueError(
                        f"{component_id} {pin.version} ({pin.sha256}, from {pin.origin}) has no vendored license "
                        f"texts; run scripts/ci/vendor-notice-sources.py"
                    )
                texts = record.texts
            resolved.setdefault(component_id, []).extend((text, pin.version) for text in texts)
    for component_id in inputs.distributions:
        component = components.get(component_id)
        if component is None:
            raise ValueError(f"no notice manifest component for {component_id}; re-run vendor-notice-sources.py")
        resolved.setdefault(component_id, []).extend((text, "") for text in component.texts)
    for component_id, component in components.items():
        if component.always:
            resolved.setdefault(component_id, []).extend((text, "") for text in component.texts)
        elif component_id not in resolved:
            raise ValueError(f"notice manifest component {component_id} matches no build input; drop it")
    return resolved


def load_vcpkg(root: Path) -> Vcpkg | None:
    """Load the vendored vcpkg ports that produce the Windows codec DLLs."""
    path = root / NOTICES_DIR / MANIFEST_NAME
    data = tomllib.loads(path.read_text(encoding="utf-8"))
    rows = data.get("vcpkg") or []
    if not rows:
        return None
    row = rows[0]
    ports = tuple(
        VcpkgPort(
            name=port["name"],
            version=port["version"],
            source=port.get("source", ""),
            digest=port.get("digest", ""),
            patches=tuple(port.get("patches") or []),
        )
        for port in row.get("port") or []
    )
    return Vcpkg(row["baseline"], ports)


def load_offers(root: Path) -> list[Offer]:
    """Load the written source offers the release stages beside the notices."""
    path = root / NOTICES_DIR / MANIFEST_NAME
    data = tomllib.loads(path.read_text(encoding="utf-8"))
    return [Offer(row["path"], row["sha256"], tuple(row.get("channels") or [])) for row in data.get("offer") or []]


def assert_offers(root: Path, inputs: BuildInputs, offers: list[Offer]) -> None:
    """Every channel conveying GPL or patched copyleft code needs a written offer to source terms.

    A channel covered by an offer file references it; a channel without one gets an offer rendered
    inline in the bundle from the same terms, so a patched artifact can never ship with neither.
    With no offer file at all there is no project term or contact to offer under, so fail.
    """
    for offer in offers:
        path = root / offer.path
        if not path.is_file():
            raise ValueError(f"{offer.path} is listed as a written offer but does not exist")
        if digest(path) != offer.sha256:
            raise ValueError(f"{offer.path} does not match its recorded digest; re-run vendor-notice-sources.py")
    if any(channel.needs_offer for channel in inputs.channels) and not offers:
        raise ValueError(
            "a channel conveys GPL or patched copyleft code but no written offer of Corresponding Source is "
            "available; restore GPL-SOURCE-OFFER.txt and re-run vendor-notice-sources.py"
        )


def offer_terms(root: Path, offers: list[Offer]) -> tuple[str, str]:
    """Read the project's stated offer term and contact from the written offer it ships."""
    for offer in offers:
        text = (root / offer.path).read_text(encoding="utf-8")
        term = re.search(r"valid for ([a-z0-9 ]+?) from", text, re.IGNORECASE)
        contact = re.search(r"[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}", text)
        if term and contact:
            return term.group(1).strip(), contact.group(0)
    raise ValueError("no written offer states the offer term and contact")


def load_spdx(root: Path) -> dict[str, LicenseText]:
    """Load the canonical SPDX texts referenced by Rust crates that ship no license file."""
    path = root / NOTICES_DIR / MANIFEST_NAME
    data = tomllib.loads(path.read_text(encoding="utf-8"))
    return {
        row["id"]: _text(root, {"name": f"{row['id']}.txt", "path": row["path"], "sha256": row["sha256"]})
        for row in data.get("spdx") or []
    }


def archive_member(archive: Path, member: str) -> bytes:
    """Read one named license file out of a pinned archive without extracting anything else."""
    if zipfile.is_zipfile(archive):
        with zipfile.ZipFile(archive) as bundle:
            names = _shallowest(
                [name for name in bundle.namelist() if PurePosixPath(name).name == member], archive, member
            )
            return bundle.read(names[0])
    with tarfile.open(archive, "r:*") as bundle:
        entries = [
            entry for entry in bundle.getmembers() if entry.isfile() and PurePosixPath(entry.name).name == member
        ]
        chosen = _shallowest([entry.name for entry in entries], archive, member)[0]
        match = next(entry for entry in entries if entry.name == chosen)
        stream = bundle.extractfile(match)
        if stream is None:
            raise ValueError(f"{archive}:{member} is unreadable")
        with stream:
            return stream.read()


def _shallowest(names: list[str], archive: Path, member: str) -> list[str]:
    """Pick the top-level copy of a license file, ignoring vendored sub-copies."""
    if not names:
        raise ValueError(f"{archive} has no entry named {member}")
    depth = min(len(PurePosixPath(name).parts) for name in names)
    top = sorted(name for name in names if len(PurePosixPath(name).parts) == depth)
    if len(top) > 1:
        raise ValueError(f"{archive} has ambiguous entries named {member}: {top}")
    return top
