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
    "rust:": "rust:1.99.1-trixie@sha256:09027f873d45b8ecd1dc86f1fa13b7a62b875c961f017a0e331267b350430137",
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
assert version == "1.99.1", version
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
