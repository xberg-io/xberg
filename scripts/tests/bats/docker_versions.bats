setup() {
	bats_load_library xberg-bats
	xberg_setup
	REPO_ROOT="$(cd "$BATS_TEST_DIRNAME/../../.." && pwd -P)"
}

@test "Docker images should_pin_verified_current_bases_without_raising_manylinux_floor" {
	python3 - "$REPO_ROOT" <<'PYTHON'
from pathlib import Path
import sys
root = Path(sys.argv[1])
files = sorted((root / "docker").glob("Dockerfile*"))
assert len(files) == 11, len(files)
expected = {
    "alpine:": "alpine:3.24.3@sha256:be2656061b536c1883c4034790e21d0a09c5aaaf51968b89368a3c0371ea2c9a",
    "rust:": "rust:1.100.0-trixie@sha256:e19840879045c951c905d0eaa9887f3439c62000e429963371b376bac8d38c31",
    "debian:": "debian:trixie-20261027-slim@sha256:f6b08b09a05725f9431502d3b5714f5c4576e82c1a84fa1debe0c24a7e0c3ec4",
    "quay.io/": "quay.io/pypa/manylinux_2_28_x86_64:2026.10.31-1@sha256:43f7ce12d79ed98b6dbfa3291d5afe389efd40a7e76de089959ffe99501b6edf",
}
count = 0
for path in files:
    for line in path.read_text().splitlines():
        if line.startswith("FROM "):
            image = line.split()[1]
        elif line.startswith("ARG MANYLINUX_IMAGE="):
            image = line.split("=", 1)[1]
        else:
            continue
        if image in ("scratch", "${MANYLINUX_IMAGE}"):
            continue
        key = next(key for key in expected if image.startswith(key))
        assert image == expected[key], (path.name, image)
        count += 1
assert count == 14, count
print("verified base images: expected=14 actual=14 across 11 Dockerfiles")
PYTHON
}

@test "Docker builders should_match_the_repository_stable_Rust_toolchain" {
	python3 - "$REPO_ROOT" <<'PYTHON'
from pathlib import Path
import sys
import tomllib
root = Path(sys.argv[1])
version = tomllib.loads((root / "rust-toolchain.toml").read_text())["toolchain"]["channel"]
assert version == "1.100.0", version
count = 0
for path in (root / "docker").glob("Dockerfile*"):
    for line in path.read_text().splitlines():
        if line.startswith("ARG RUST_TOOLCHAIN="):
            assert line == f"ARG RUST_TOOLCHAIN={version}", (path.name, line)
            count += 1
assert count == 9, count
print("Rust builders: expected=9 actual=9")
PYTHON
}

@test "Docker build tools should_pin_current_maturin_and_napi_with_supported_Node" {
	python3 - "$REPO_ROOT" <<'PYTHON'
from pathlib import Path
import sys
root = Path(sys.argv[1]) / "docker"
assert "maturin==1.15.0" in (root / "Dockerfile.musl-python").read_text()
for name in ("manylinux-node", "musl-node"):
    assert "@napi-rs/cli@3.10.9 " in (root / f"Dockerfile.{name}").read_text(), name
assert "module enable nodejs:24" in (root / "Dockerfile.manylinux-node").read_text()
print("build tools: expected=4 actual=4")
PYTHON
}

@test "Docker OCR source helpers should_be_copied_and_allowed_in_every_source_builder" {
	python3 - "$REPO_ROOT" <<'PYTHON'
from pathlib import Path
import sys
root = Path(sys.argv[1])
ignore = (root / ".dockerignore").read_text().splitlines()
helpers = ("build-ocr-linux.sh", "verify-ocr-linux.py", "verify-ocr-fixture.sh")
count = 0
for variant in ("core", "full", "manylinux-node", "manylinux-rustler"):
    text = (root / "docker" / f"Dockerfile.{variant}").read_text()
    copies = [line for line in text.splitlines() if line.startswith("COPY scripts/")]
    for helper in helpers:
        source = "scripts/ci/install-system-deps/" + helper
        assert (root / source).is_file(), source
        assert "!" + source in ignore, source
        assert any(source in line for line in copies), (variant, helper)
        count += 1
assert count == 12, count
print("OCR source helper copies: expected=12 actual=12")
PYTHON
}

@test "Docker manylinux builders should_take_English_tessdata_from_AppStream_tesseract_only" {
	python3 - "$REPO_ROOT" <<'PYTHON'
from pathlib import Path
import re
import sys
root = Path(sys.argv[1])
count = 0
for variant in ("manylinux-node", "manylinux-rustler"):
    text = (root / "docker" / f"Dockerfile.{variant}").read_text()
    installs = re.findall(r"dnf -y install \\\n((?:\s+[^\n]*\\\n)+)", text)
    assert len(installs) == 1, (variant, len(installs))
    packages = installs[0].split()
    assert "tesseract" in packages, variant
    assert not any(p.startswith("tesseract-langpack") for p in packages), variant
    assert "OCR_TESSDATA_DIR=/usr/share/tesseract/tessdata" in text, variant
    count += 1
assert count == 2, count
print("manylinux tessdata sources: expected=2 actual=2")
PYTHON
}

@test "Docker builders should_default_to_one_Cargo_job_and_allow_build_arg_overrides" {
	python3 - "$REPO_ROOT" <<'PYTHON'
from pathlib import Path
import sys
root = Path(sys.argv[1])
files = sorted((root / "docker").glob("Dockerfile*"))
assert len(files) == 11, len(files)
count = 0
for path in files:
    text = path.read_text()
    builder = text.split("FROM ", 1)[1].split("\nFROM ", 1)[0]
    assert builder.count("ARG CARGO_BUILD_JOBS=1\n") == 1, path.name
    assert builder.index("ARG CARGO_BUILD_JOBS=1") < builder.index("RUN "), path.name
    assert "ENV CARGO_BUILD_JOBS=" not in builder, path.name
    count += 1
assert count == 11, count
print("Cargo build job defaults: expected=11 actual=11")
PYTHON
}

@test "Docker publish overrides should_use_the_reviewed_manylinux_images_on_both_architectures" {
	python3 - "$REPO_ROOT" <<'PYTHON'
from collections import Counter
from pathlib import Path
import re
import sys
root = Path(sys.argv[1])
text = (root / ".github/workflows/publish.yaml").read_text()
images = re.findall(r"image:\s*(quay\.io/pypa/manylinux[^,\s]+)", text)
expected = {
    "quay.io/pypa/manylinux_2_28_x86_64:2026.10.31-1@sha256:43f7ce12d79ed98b6dbfa3291d5afe389efd40a7e76de089959ffe99501b6edf": 2,
    "quay.io/pypa/manylinux_2_28_aarch64:2026.10.31-1@sha256:724b504247662e1cf122590957c857211ec2d16b113ec134f92c570a2d01250f": 2,
}
assert len(images) == 4, images
assert Counter(images) == expected, images
print("published manylinux image overrides: expected=4 actual=4")
PYTHON
}

verify_published_ocr_notices() {
	python3 - "$REPO_ROOT" "$1" <<'PYTHON'
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tarfile
import tempfile
import textwrap

root = Path(sys.argv[1])
missing = sys.argv[2] == "missing"
workflow = (root / ".github/workflows/publish.yaml").read_text()
notices = (
    "tesseract-5.5.4/LICENSE",
    "tesseract-5.5.4/AUTHORS",
    "leptonica-1.87.0/leptonica-license.txt",
)
steps = (
    ("Stage .node into napi platform dir for npm", "npm"),
    ("Package NIF as RustlerPrecompiled tarball", "elixir"),
)
with tempfile.TemporaryDirectory() as temporary:
    base = Path(temporary)
    for title, kind in steps:
        section = workflow.rsplit("- name: " + title + "\n", 1)[1]
        lines = section.split("        run: |\n", 1)[1].splitlines()
        end = next((i for i, line in enumerate(lines) if line and not line.startswith("          ")), len(lines))
        script = textwrap.dedent("\n".join(lines[:end]))
        script = script.replace("${{ matrix.napi_arch }}", "x64")
        script = script.replace("${{ matrix.target }}", "x86_64-unknown-linux-gnu")
        assert "${{" not in script, title
        work = base / kind
        helper = work / "scripts/ci/package-notices.py"
        helper.parent.mkdir(parents=True)
        shutil.copyfile(root / "scripts/ci/package-notices.py", helper)
        for name in ("LICENSE", "THIRD_PARTY_LICENSES.md"):
            shutil.copyfile(root / name, work / name)
        output = work / "build-output"
        output.mkdir(parents=True)
        (output / ("xberg.node" if kind == "npm" else "libxberg_nif.so")).write_bytes(b"staging fixture")
        (output / "libfixture.so.1").write_bytes(b"closure fixture")
        for name in notices:
            path = output / "licenses/xberg-system-ocr" / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text("notice propagation fixture: " + name + "\n")
        if missing:
            (output / "licenses/xberg-system-ocr" / notices[2]).unlink()
        manifest = work / "crates/xberg-node/npm/linux-x64-gnu/package.json"
        manifest.parent.mkdir(parents=True)
        manifest.write_text(json.dumps({"name": "ocr-notice-propagation-fixture", "version": "0.0.0", "files": ["*.node"]}))
        env = dict(os.environ, TARGET="x86_64-unknown-linux-gnu", NIF_API="2.16", NIF_VERSION="0.0.0")
        result = subprocess.run(["bash", "-euo", "pipefail", "-c", script], cwd=work, env=env, text=True, capture_output=True)
        if missing:
            assert result.returncode != 0, (title, "missing notice accepted")
            assert "leptonica-license.txt" in result.stderr, result.stderr
            continue
        assert result.returncode == 0, (title, result.stdout, result.stderr)
        if kind == "npm":
            package = work / "staging/npm/linux-x64-gnu"
            packed = subprocess.run(["npm", "pack", "--ignore-scripts", "--json", "--cache", str(base / "npm-cache")], cwd=package, text=True, capture_output=True, check=True)
            archive = package / json.loads(packed.stdout)[0]["filename"]
            prefix = "package/licenses/xberg-system-ocr/"
        else:
            archives = list((work / "dist/elixir-natives").glob("*.tar.gz"))
            assert len(archives) == 1, archives
            archive = archives[0]
            prefix = "licenses/xberg-system-ocr/"
        with tarfile.open(archive) as tar:
            members = {member.name.removeprefix("./"): member for member in tar.getmembers() if member.isfile()}
            for name in notices:
                assert prefix + name in members, (kind, name, sorted(members))
                actual = tar.extractfile(members[prefix + name]).read()
                assert actual == ("notice propagation fixture: " + name + "\n").encode(), (kind, name)
        print(f"{kind}: expected notices=3 actual notices=3 in genuine package archive")
    if missing:
        print("expected rejected staging paths=2 actual=2; missing Leptonica notice")
PYTHON
}

@test "Docker packages should_ship_all_OCR_notices_in_npm_and_NIF_archives" {
	verify_published_ocr_notices present
}

@test "Docker packages should_reject_staging_when_an_OCR_notice_is_missing" {
	verify_published_ocr_notices missing
}

@test "Docker source builds should_default_to_one_CMake_job_and_allow_overrides" {
	python3 - "$REPO_ROOT" <<'PYTHON'
from pathlib import Path
import sys
root = Path(sys.argv[1]) / "docker"
names = ("core", "full", "manylinux-node", "manylinux-rustler")
for name in names:
    text = (root / f"Dockerfile.{name}").read_text()
    assert text.count("ARG CMAKE_BUILD_PARALLEL_LEVEL=1\n") == 1, name
    assert 'cmake --build build -j"${CMAKE_BUILD_PARALLEL_LEVEL:-$(nproc)}"' in text, name
print("bounded CMake source builders: expected=4 actual=4")
PYTHON
}

verify_docker_archive_integrity() {
	python3 - "$REPO_ROOT" "$1" <<'PYTHON'
from collections import Counter
from pathlib import Path
import re
import sys

root = Path(sys.argv[1])
verifier = "scripts/ci/actions/setup-onnx-runtime/verify-archive.sh"
nuget_fallback = "scripts/ci/actions/setup-onnx-runtime/nuget-fallback.sh"
manifests = {
    "PDFIUM": ("scripts/pdfium-runtime-checksums.sha256", "pdfium-runtime-checksums.sha256"),
    "ORT": ("scripts/ci/actions/setup-onnx-runtime/checksums.sha256", "onnxruntime-checksums.sha256"),
}
rustup_sha256 = {
    "musl": {
        "x86_64": "1552169aba2836777136ccff7a381e95c17233b5d8250cd5e10ae3cb971bfcfc",
        "aarch64": "956deacdc27ab82ae7ac821d5d3e9e95e4795d476d17efe99743bb8531ca1031",
    },
    "gnu": {
        "x86_64": "b1c8cb14e30889e7e2999e6b69e62d9e9b9ffc0f4df8180564d306d9680e6c55",
        "aarch64": "9f608c142e0e38a78957d0661ba1dfaf0b6fb37184cfaa4b9a0aa57af6184282",
    },
}
archive_sets = {
    "core": ("PDFIUM", "ORT"),
    "full": ("PDFIUM", "ORT"),
    "cli": ("PDFIUM",),
    "musl-build": ("PDFIUM",),
    "manylinux-node": ("ORT",),
    "manylinux-rustler": ("ORT",),
}
variants = {name: (root / "docker" / f"Dockerfile.{name}").read_text() for name in archive_sets}
ignore = (root / ".dockerignore").read_text()
integrity = "/opt/xberg-integrity/"
archive_urls = {
    "PDFIUM": "https://github.com/bblanchon/pdfium-binaries/releases/download/chromium/${PDFIUM_VERSION}/${PDFIUM_ASSET}",
    "ORT": "https://github.com/microsoft/onnxruntime/releases/download/v${ONNXRUNTIME_VERSION}/${ORT_ASSET}",
}
rustup_urls = {
    "musl": "https://static.rust-lang.org/rustup/archive/${RUSTUP_VERSION}/${ARCH}-unknown-linux-musl/rustup-init",
    "gnu": "https://static.rust-lang.org/rustup/archive/${RUSTUP_VERSION}/${TARGET}/rustup-init",
}
alpine_repository_urls = (
    "https://dl-cdn.alpinelinux.org/alpine/edge/community",
    "https://dl-cdn.alpinelinux.org/alpine/edge/main",
)
codec_urls = (
    "https://github.com/strukturag/libde265/releases/download/v1.1.3/libde265-1.1.3.tar.gz",
    "https://storage.googleapis.com/aom-releases/libaom-3.15.2.tar.gz",
    "https://github.com/strukturag/libheif/releases/download/v1.23.7/libheif-1.23.7.tar.gz",
)
epel = "https://dl.fedoraproject.org/pub/epel/epel-release-latest-8.noarch.rpm"
source_label = "https://github.com/xberg-io/xberg"
# Count duplicate literal URLs, including repositories in the standalone musl builder.
external_urls = {
    "core": (*codec_urls, *archive_urls.values(), source_label),
    "full": (*codec_urls, *archive_urls.values(), source_label),
    "cli": (rustup_urls["musl"], archive_urls["PDFIUM"], source_label),
    "musl-build": (*alpine_repository_urls, rustup_urls["musl"], archive_urls["PDFIUM"]),
    "manylinux-node": (epel, *codec_urls, archive_urls["ORT"], rustup_urls["gnu"]),
    "manylinux-rustler": (epel, *codec_urls, archive_urls["ORT"], rustup_urls["gnu"]),
}
literal_url = re.compile(r"""https?://[^\s"'<>`]*""", re.IGNORECASE)
archive_reference = re.compile(r"pdfium|onnxruntime|ORT_(?:ASSET|ARCHIVE)", re.IGNORECASE)
apk_ort_install = re.compile(
    r"apk add --no-cache (?:[\w.+-]+ )*onnxruntime(?:-dev)?(?: [\w.+-]+)* "
    + " ".join(f"--repository={re.escape(url)}" for url in alpine_repository_urls)
)


def is_alpine_package_install(name, instruction, urls):
    if name != "musl-build" or tuple(urls) != alpine_repository_urls:
        return False
    if not instruction.startswith("RUN "):
        return False
    steps = instruction.removeprefix("RUN ").split("&&")
    repository_steps = [step.strip() for step in steps if literal_url.search(step)]
    return len(repository_steps) == 1 and apk_ort_install.fullmatch(repository_steps[0]) is not None


def verify_variant(name, text, allowlist):
    source_lines = [line for line in text.splitlines() if not line.lstrip().startswith("#")]
    commands = re.sub(r"\\\n[ \t]*", "", "\n".join(source_lines))
    instructions = [line.strip() for line in commands.splitlines() if line.strip()]
    runs = [line for line in instructions if line.startswith("RUN ")]
    url_uses = [(line, urls) for line in instructions if (urls := literal_url.findall(line))]
    copies = [line.split()[1:] for line in instructions if line.startswith("COPY scripts/")]
    assert not any("chromium-browser-snapshots" in line for line in instructions), (name, "snapshot endpoint")
    assert not any("sh.rustup.rs" in line for line in instructions), (name, "Rustup endpoint")
    observed_urls = Counter(url for _, urls in url_uses for url in urls)
    expected_urls = Counter(external_urls[name])
    assert observed_urls == expected_urls, (
        name, "external URL inventory", dict(observed_urls - expected_urls), dict(expected_urls - observed_urls)
    )

    def verify_copy(source, target):
        destinations = [copy[-1] for copy in copies if source in copy[:-1]]
        destinations = [dest + Path(source).name if dest.endswith("/") else dest for dest in destinations]
        assert destinations == [integrity + target], (name, source, "COPY source and destination")
        assert (root / source).is_file(), source
        assert allowlist.splitlines().count("!" + source) == 1, (name, source, "Docker allowlist")

    verify_copy(verifier, "verify-archive.sh")
    expected = archive_sets[name]
    assert sum(line.count(integrity + "verify-archive.sh") for line in runs) == 2 * len(expected), (name, "verifier count")
    verified_runs = set()
    for kind, (source, target) in manifests.items():
        archive = f'"${kind}_ARCHIVE"'
        endpoint = archive_urls[kind]
        downloads = [line for line, urls in url_uses if endpoint in urls]
        extracts = [line for line in runs if f"tar -xzf {archive}" in line]
        count = int(kind in expected)
        assert len(downloads) == count, (name, kind, "archive download count")
        assert sum(line.count(f"-o {archive}") for line in runs) == count, (name, kind, "archive output count")
        assert sum(line.count(f"tar -xzf {archive}") for line in extracts) == count, (name, kind, "archive extract count")
        if not count:
            continue
        verify_copy(source, target)
        verify = f"bash {integrity}verify-archive.sh {archive} {integrity}{target}"
        if kind == "ORT":
            verify_copy(nuget_fallback, "nuget-fallback.sh")
            run = downloads[0]
            head, conditional, tail = run.partition("if curl -fL ")
            head_steps = [step.strip() for step in head.split("&&")]
            assert (
                conditional
                and verify + " --known" in head_steps
                and not any(step.startswith(("curl ", "tar ")) or "nuget-fallback" in step for step in head_steps)
            ), (name, kind, "preflight/download/verify/extract order")
            branches = re.fullmatch(
                r'(?P<url>"[^"]+") -o "\$ORT_ARCHIVE"; then (?P<ok>.*?); else (?P<bad>.*?); fi && (?P<rest>.*)', tail
            )
            assert branches is not None, (name, kind, "preflight/download/verify/extract order")
            assert branches["url"] == f'"{endpoint}"', (name, kind, "preflight/download/verify/extract order")
            ok_steps = [step.strip() for step in branches["ok"].split("&&")]
            assert (
                len(ok_steps) == 2
                and ok_steps[0] == verify
                and ok_steps[1].startswith(f"tar -xzf {archive} ")
                and extracts == downloads
            ), (name, kind, "preflight/download/verify/extract order")
            fallback = (
                f'rm -f {archive} && RUNNER_TEMP=/tmp bash {integrity}nuget-fallback.sh "$ONNXRUNTIME_VERSION" '
                f'"$ORT_RID" /build/onnxruntime {integrity}{target}'
            )
            assert branches["bad"].strip() == fallback and run.count("nuget-fallback.sh") == 1, (
                name, kind, "nuget fallback invocation"
            )
            assert "curl " not in branches["rest"] and "nuget-fallback" not in branches["rest"], (
                name, kind, "nuget fallback invocation"
            )
            rid_pairs = re.findall(r'ORT_ARCH="(\w+)"; ORT_RID="([\w-]+)";', run)
            assert sorted(rid_pairs) == [("aarch64", "linux-arm64"), ("x64", "linux-x64")], (
                name, kind, "nuget fallback invocation"
            )
            verified_runs.add(run)
            continue
        steps = [step.strip() for step in downloads[0].split("&&")]
        checks = [step for step in steps if verify in step or step.startswith(("curl ", "tar -xzf "))]
        assert (
            len(checks) == 4
            and checks[0] == verify + " --known"
            and checks[1].startswith("curl ")
            and endpoint in checks[1]
            and checks[1].endswith(f"-o {archive}")
            and checks[2] == verify
            and checks[3].startswith(f"tar -xzf {archive} ")
            and extracts == downloads
        ), (name, kind, "preflight/download/verify/extract order")
        verified_runs.add(downloads[0])
    for instruction, urls in url_uses:
        if archive_reference.search(instruction):
            assert instruction in verified_runs or is_alpine_package_install(name, instruction, urls), (
                name, "archive URL outside verified chain", instruction
            )
    return len(expected)


def rustup_pins(name, text, libc):
    runs = [line for line in re.sub(r"\\\n[ \t]*", "", text).splitlines() if line.startswith("RUN RUSTUP_VERSION=")]
    assert len(runs) == 1 and text.count("https://static.rust-lang.org/rustup/archive/") == 1, (name, "Rustup bootstrap count")
    version = re.findall(r"^RUN RUSTUP_VERSION=(\d+\.\d+\.\d+) &&", text, re.M)
    suffix = "-unknown-linux-gnu" if libc == "gnu" else ""
    pins = re.findall(rf"^\s*(x86_64|aarch64){suffix}\) RUSTUP_SHA256=([0-9a-f]{{64}}) ;;", text, re.M)
    assert version == ["1.29.3"] and len(pins) == 2 and {arch for arch, _ in pins} == {"x86_64", "aarch64"}, (name, "Rustup pin inventory")

    assert dict(pins) == rustup_sha256[libc], (name, "Rustup digest mismatch")

    selector = 'case "$TARGET" in' if libc == "gnu" else 'case "$ARCH" in'
    endpoint = rustup_urls[libc]
    steps = [step.strip() for step in runs[0].split("&&")]
    guard = [step for step in steps if step.startswith(selector)]
    assert len(guard) == 1 and "exit 1 ;;" in guard[0], (name, "Rustup target guard")
    checked = [step for step in steps if step.startswith(("curl ", "printf ", "chmod ", "./rustup-init ", "rm rustup-init"))]
    assert (
        len(checked) == 5
        and checked[0].startswith("curl ")
        and checked[0].endswith(f'"{endpoint}" -o rustup-init')
        and checked[1] == r"""printf '%s  rustup-init\n' "$RUSTUP_SHA256" | sha256sum -c -"""
        and checked[2] == "chmod 0755 rustup-init"
        and checked[3] == './rustup-init -y --default-toolchain "${RUST_TOOLCHAIN}" --component rust-src'
        and checked[4] == "rm rustup-init"
        and steps.index(guard[0]) < steps.index(checked[0])
    ), (name, "Rustup download/verify/execute order")


def verify_rustup(variants):
    for libc, names in (("musl", ("cli", "musl-build")), ("gnu", ("manylinux-node", "manylinux-rustler"))):
        for name in names:
            rustup_pins(name, variants[name], libc)


def swap(text, old, new, count=1):
    assert old in text, ("mutation target missing", old)
    return text.replace(old, new, count)


if sys.argv[2] == "negative":
    core = variants["core"]
    wrong_gnu_digest = {
        name: text.replace("RUSTUP_SHA256=b1c8", "RUSTUP_SHA256=a1c8", 1)
        if name in ("manylinux-node", "manylinux-rustler") else text
        for name, text in variants.items()
    }
    cli = variants["cli"]
    mirror = "https://mirror.invalid/pdfium-linux-x64.tgz"
    unreviewed_url_instructions = (
        "RUN apk add --no-cache onnxruntime " + " ".join(f"--repository={url}" for url in alpine_repository_urls),
        f"ADD {mirror} /tmp/p.tgz",
        f"RUN wget -qO p.tgz {mirror} && tar -xzf p.tgz",
        f"RUN if true; then curl {mirror}; fi",
        f"RUN X=1 curl {mirror}",
        f"RUN (curl {mirror} | tar -xz)",
        "RUN curl https://mirror.invalid/get?id=42 -o pdfium.tgz",
        "RUN curl https://mirror.invalid/get -o /usr/lib/libpdfium.so",
    )
    for instruction in unreviewed_url_instructions:
        try:
            verify_variant("cli", cli + "\n" + instruction + "\n", ignore)
        except AssertionError as error:
            assert "external URL inventory" in str(error), (instruction, error)
        else:
            raise AssertionError(f"mutation passed: {instruction}")
    checked = (
        (lambda: verify_variant("cli", variants["cli"] + "\nRUN curl https://mirror.invalid/pdfium-linux-x64.tgz -o p.tgz && tar -xzf p.tgz\n", ignore), "external URL inventory"),
        (lambda: verify_variant("core", core + "\nRUN true && curl --url=https://mirror.invalid/onnxruntime-linux-x64-1.24.2.tgz -o o.tgz\n", ignore), "external URL inventory"),
        (
            lambda: verify_variant(
                "cli", cli.replace(
                    f'LABEL org.opencontainers.image.source="{source_label}"',
                    f'RUN echo "{source_label}" > /tmp/libpdfium.so',
                ), ignore,
            ),
            "archive URL outside verified chain",
        ),
        (lambda: verify_variant("core", core.replace(" --known &&", " &&", 1), ignore), "preflight/download/verify/extract order"),
        (lambda: verify_variant("core", swap(core, "COPY " + nuget_fallback + " /opt/xberg-integrity/nuget-fallback.sh\n", ""), ignore), "COPY source and destination"),
        (lambda: verify_variant("core", core, swap(ignore, "!" + nuget_fallback, nuget_fallback)), "Docker allowlist"),
        (lambda: verify_variant("core", swap(core, "bash /opt/xberg-integrity/nuget-fallback.sh", "true", 1), ignore), "nuget fallback invocation"),
        (lambda: verify_variant("core", swap(core, '"$ORT_RID" /build/onnxruntime /opt/xberg-integrity/onnxruntime-checksums.sha256', '"$ORT_RID" /build/onnxruntime /opt/xberg-integrity/pdfium-runtime-checksums.sha256', 1), ignore), "nuget fallback invocation"),
        (lambda: verify_variant("core", swap(core, '    else \\\n        rm -f "$ORT_ARCHIVE" &&', '    fi || true; \\\n    true && \\\n        rm -f "$ORT_ARCHIVE" &&', 1), ignore), "preflight/download/verify/extract order"),
        (lambda: verify_variant("core", swap(core, 'fi && \\\n    ORT_ASSET', 'fi && \\\n    RUNNER_TEMP=/tmp bash /opt/xberg-integrity/nuget-fallback.sh "$ONNXRUNTIME_VERSION" "$ORT_RID" /build/onnxruntime /opt/xberg-integrity/onnxruntime-checksums.sha256 && \\\n    ORT_ASSET', 1), ignore), "preflight/download/verify/extract order"),
        (lambda: verify_variant("manylinux-rustler", swap(variants["manylinux-rustler"], '        bash /opt/xberg-integrity/verify-archive.sh "$ORT_ARCHIVE" /opt/xberg-integrity/onnxruntime-checksums.sha256 && \\\n        tar -xzf "$ORT_ARCHIVE" -C /build/onnxruntime --strip-components=1; \\\n', '        tar -xzf "$ORT_ARCHIVE" -C /build/onnxruntime --strip-components=1 && \\\n        bash /opt/xberg-integrity/verify-archive.sh "$ORT_ARCHIVE" /opt/xberg-integrity/onnxruntime-checksums.sha256; \\\n', 1), ignore), "preflight/download/verify/extract order"),
        (lambda: verify_variant("manylinux-node", swap(variants["manylinux-node"], 'ORT_RID="linux-arm64"', 'ORT_RID="linux-x64"', 1), ignore), "nuget fallback invocation"),
        (lambda: verify_variant("core", core.replace(" --known &&", " &&", 1), ignore), "preflight/download/verify/extract order"),
        (lambda: verify_variant("core", core.replace('tar -xzf "$PDFIUM_ARCHIVE"', 'tar -xzf unverified.tgz', 1), ignore), "archive extract count"),
        (lambda: verify_variant("core", core.replace("COPY " + verifier, "COPY scripts/other.sh", 1), ignore), "COPY source and destination"),
        (lambda: verify_variant("core", core, ignore.replace("!" + verifier, verifier)), "Docker allowlist"),
        (lambda: verify_variant("core", core + "\nRUN curl https://chromium-browser-snapshots.invalid\n", ignore), "snapshot endpoint"),
        (lambda: verify_variant("manylinux-node", variants["manylinux-node"] + "\nRUN curl https://sh.rustup.rs\n", ignore), "Rustup endpoint"),
        (lambda: rustup_pins("manylinux-node", variants["manylinux-node"].replace('"$RUSTUP_SHA256" | sha256sum -c -', '"$RUSTUP_SHA256"', 1), "gnu"), "Rustup download/verify/execute order"),
        (lambda: rustup_pins("manylinux-node", variants["manylinux-node"].replace("RUSTUP_SHA256=9", "RUSTUP_SHA256=g", 1), "gnu"), "Rustup pin inventory"),
        (lambda: verify_rustup(wrong_gnu_digest), "Rustup digest mismatch"),
    )
    for check, reason in checked:
        try:
            check()
        except AssertionError as error:
            assert reason in str(error), (reason, error)
        else:
            raise AssertionError(f"mutation passed: {reason}")
    print(f"rejected {len(checked) + len(unreviewed_url_instructions)} expected archive integrity regressions")
else:
    assert sum(verify_variant(name, text, ignore) for name, text in variants.items()) == 8
    verify_rustup(variants)
    assert sum(map(len, external_urls.values())) == 31
    print("six Dockerfiles: 31 reviewed URLs, eight verified archive chains, four verified Rustup bootstraps")
PYTHON
}

@test "Docker builders should verify every pinned PDFium and ONNX Runtime archive in order" {
	verify_docker_archive_integrity stock
}

@test "Docker integrity checks should reject verification, allowlist, endpoint and Rustup pin regressions" {
	verify_docker_archive_integrity negative
}
