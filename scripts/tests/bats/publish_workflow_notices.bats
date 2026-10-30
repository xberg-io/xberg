#!/usr/bin/env bats

setup() {
	[ -z "${XBERG_EXPECTED_BASH_MAJOR_MINOR:-}" ] || [[ "$BASH_VERSION" == "$XBERG_EXPECTED_BASH_MAJOR_MINOR".* ]]
	bats_load_library xberg-bats
	xberg_setup
	REPOSITORY_ROOT="$(cd "$BATS_TEST_DIRNAME/../../.." && pwd -P)"
	cd "$XBERG_WORK" || return
	mkdir -p scripts/ci build-output staging crates/xberg-node/npm/linux-x64-musl crates/xberg-node/npm/linux-x64-gnu
	cp "$REPOSITORY_ROOT/scripts/ci/package-notices.py" scripts/ci/
	cp "$REPOSITORY_ROOT/LICENSE" "$REPOSITORY_ROOT/THIRD_PARTY_LICENSES.md" .
	printf native >build-output/binding.node
	printf native >build-output/libxberg_nif.so
	printf closure >build-output/libonnxruntime.so.1
	for platform in linux-x64-musl linux-x64-gnu; do
		printf '{"files":["binding.node"]}\n' >"crates/xberg-node/npm/$platform/package.json"
	done
	for notice in tesseract-5.5.4/LICENSE tesseract-5.5.4/AUTHORS leptonica-1.87.0/leptonica-license.txt; do
		mkdir -p "build-output/licenses/xberg-system-ocr/$(dirname "$notice")"
		printf OCR >"build-output/licenses/xberg-system-ocr/$notice"
	done
	export TARGET=fixture-target NIF_VERSION=1.4.0 NIF_API=2.16 VERSION=1.4.0
}

extract_run() {
	python3 - "$REPOSITORY_ROOT/.github/workflows/publish.yaml" "$1" "$2" <<'PYTHON' >"$XBERG_WORK/step.sh"
import re, sys
from pathlib import Path
lines = Path(sys.argv[1]).read_text().splitlines()
job = step = ""
blocks = []
for index, line in enumerate(lines):
    if re.match(r"^  [a-z][a-z0-9-]+:$", line):
        job = line.strip()[:-1]
    if line.startswith("      - name: "):
        step = line.removeprefix("      - name: ")
    if job == sys.argv[2] and step.startswith(sys.argv[3]) and line == "        run: |":
        body = []
        for following in lines[index + 1:]:
            if following and not following.startswith("          "):
                break
            body.append(following[10:])
        blocks.append("\n".join(body))
assert len(blocks) == 1, f"expected one packaging step, found {len(blocks)}"
body = blocks[0].replace("${{ matrix.target }}", "fixture-target").replace("${{ matrix.napi_arch }}", "x64")
assert "${{" not in body, body
print(body)
PYTHON
}

verify_archive() {
	run python3 scripts/ci/package-notices.py verify "$1" "$2"
	xberg_assert_status 0
	xberg_assert_output_contains 'Verified 2 canonical notice files'
}

@test "Both Linux npm workflow repacks should retain canonical bytes and npm file allowlists" {
	local count=0 job platform
	for job in node-bindings-musl node-bindings-manylinux; do
		count=$((count + 1))
		platform=linux-x64-musl
		[ "$job" != node-bindings-manylinux ] || platform=linux-x64-gnu
		extract_run "$job" 'Stage .node into napi platform dir for npm'
		run bash -e -o pipefail "$XBERG_WORK/step.sh"
		xberg_assert_status 0
		verify_archive node-bindings-fixture-target.tar.gz "npm/$platform"
		python3 - "$platform" <<'PYTHON'
import json, sys
from pathlib import Path
package = json.loads((Path("staging/npm") / sys.argv[1] / "package.json").read_text())
assert {"LICENSE", "THIRD_PARTY_LICENSES.md", "binding.node", "libonnxruntime.so.1"} <= set(package["files"])
PYTHON
	done
	[ "$count" -eq 2 ]
}

@test "NAPI declaration repack should retain canonical notices for every populated platform" {
	extract_run node-bindings 'Ship the alef-generated index.d.ts'
	cp build-output/binding.node crates/xberg-node/npm/linux-x64-gnu/
	xberg_stub git 'exit 0'
	run bash -e -o pipefail "$XBERG_WORK/step.sh"
	xberg_assert_status 0
	verify_archive node-bindings-fixture-target.tar.gz npm/linux-x64-gnu
}

@test "C FFI workflow staging should ship exact canonical notices beside its native libraries" {
	extract_run c-ffi-libraries 'Stage C FFI distribution'
	export PLATFORM=fixture FFI_SHARED_LIBRARY="$PWD/build-output/libxberg_nif.so" FFI_STATIC_LIBRARY="$PWD/static.a" FFI_NATIVE_STATIC_LIBS="$PWD/native-libs.txt"
	printf static >static.a
	printf metadata >native-libs.txt
	mkdir -p crates/xberg-ffi/include
	printf header >crates/xberg-ffi/include/xberg.h
	run bash -e -o pipefail "$XBERG_WORK/step.sh"
	xberg_assert_status 0
	verify_archive dist/c-ffi/xberg-ffi-fixture.tar.gz xberg-ffi-fixture
}

@test "Both alternate NIF workflow archives should retain canonical bytes before checksums" {
	local count=0 job archive
	for job in elixir-natives-musl elixir-natives-manylinux; do
		count=$((count + 1))
		extract_run "$job" 'Package NIF as RustlerPrecompiled tarball'
		run bash -e -o pipefail "$XBERG_WORK/step.sh"
		xberg_assert_status 0
		archive=dist/elixir-natives/libxberg_nif-v1.4.0-nif-2.16-fixture-target.so.tar.gz
		verify_archive "$archive" .
		(cd dist/elixir-natives && shasum -a 256 -c "$(basename "$archive").sha256")
	done
	[ "$count" -eq 2 ]
}

@test "Dart desktop workflow should retain canonical bytes in the final checksummed download" {
	extract_run upload-dart-release 'Package server natives as tarballs'
	mkdir -p dart-server-artifacts/dart-server-linux-x64
	cp build-output/libxberg_nif.so dart-server-artifacts/dart-server-linux-x64/
	run bash -e -o pipefail "$XBERG_WORK/step.sh"
	xberg_assert_status 0
	verify_archive dist/dart-release/xberg-dart-v1.4.0-linux-x64.tar.gz .
	(cd dist/dart-release && shasum -a 256 -c xberg-dart-v1.4.0-linux-x64.tar.gz.sha256)
}

@test "Final npm pack should retain canonical notices in the consumer tarball" {
	extract_run publish-node 'Stage platform packages and main package'
	export GITHUB_WORKSPACE="$PWD"
	mkdir -p node-artifacts artifact/npm/linux-x64-gnu
	printf '{"name":"@xberg-io/notice-fixture","version":"1.4.0","files":["binding.node"]}\n' >artifact/npm/linux-x64-gnu/package.json
	printf native >artifact/npm/linux-x64-gnu/binding.node
	tar -czf node-artifacts/platform.tar.gz -C artifact npm
	printf '{"name":"main-fixture"}\n' >crates/xberg-node/package.json
	run bash -e -o pipefail "$XBERG_WORK/step.sh"
	xberg_assert_status 0
	verify_archive crates/xberg-node/npm/xberg-io-notice-fixture-1.4.0.tgz package
}

@test "NAPI workflow should reject a zero-platform sweep" {
	extract_run node-bindings 'Ship the alef-generated index.d.ts'
	xberg_stub git 'exit 0'
	run bash -e -o pipefail "$XBERG_WORK/step.sh"
	xberg_assert_status 1
	xberg_assert_output_contains 'no populated NAPI platform packages'
	[ ! -e node-bindings-fixture-target.tar.gz ]
}

@test "NIF workflow should reject missing canonical source before creating its checksum" {
	extract_run elixir-natives-musl 'Package NIF as RustlerPrecompiled tarball'
	rm THIRD_PARTY_LICENSES.md
	run bash -e -o pipefail "$XBERG_WORK/step.sh"
	xberg_assert_status 1
	xberg_assert_output_contains 'THIRD_PARTY_LICENSES.md'
	[ ! -e dist/elixir-natives/libxberg_nif-v1.4.0-nif-2.16-fixture-target.so.tar.gz ]
}

@test "NIF workflow should reject corrupted staged notice bytes before writing a checksum" {
	extract_run elixir-natives-musl 'Package NIF as RustlerPrecompiled tarball'
	python3 - <<'PYTHON'
from pathlib import Path
helper = Path("scripts/ci/package-notices.py")
source = helper.read_text()
needle = "            stage(args.destinations, notices)"
assert source.count(needle) == 1
helper.write_text(source.replace(needle, needle + '\n            (Path(args.destinations[0]) / "THIRD_PARTY_LICENSES.md").write_bytes(b"corrupt")'))
PYTHON
	run bash -e -o pipefail "$XBERG_WORK/step.sh"
	xberg_assert_status 1
	xberg_assert_output_contains 'differs from canonical THIRD_PARTY_LICENSES.md'
	[ ! -e dist/elixir-natives/libxberg_nif-v1.4.0-nif-2.16-fixture-target.so.tar.gz.sha256 ]
}

@test "macOS NAPI closure repack should retain complete canonical notices and added dylibs" {
	extract_run node-bindings 'Vendor macOS dylib closure'
	mkdir -p crates/xberg-node/npm/darwin-arm64
	printf '{"files":["binding.node"]}\n' >crates/xberg-node/npm/darwin-arm64/package.json
	cp build-output/binding.node crates/xberg-node/npm/darwin-arm64/
	cat >scripts/ci/vendor-macos-node-dylibs.sh <<'SHELL'
printf closure >"$1/libclosure.dylib"
SHELL
	run bash -e -o pipefail "$XBERG_WORK/step.sh"
	xberg_assert_status 0
	verify_archive node-bindings-fixture-target.tar.gz npm/darwin-arm64
	[ "$(tar -xOzf node-bindings-fixture-target.tar.gz npm/darwin-arm64/libclosure.dylib)" = closure ]
}

prepare_c_ffi_closure_fixture() {
	extract_run c-ffi-libraries 'Ship + verify ONNX Runtime (glibc Linux)'
	export GLIBC_FLOOR=2.28
	mkdir -p dist/c-ffi closure/xberg-ffi-fixture/lib
	python3 scripts/ci/package-notices.py stage closure/xberg-ffi-fixture
	tar -czf dist/c-ffi/xberg-ffi-fixture.tar.gz -C closure xberg-ffi-fixture
	cat >scripts/ci/vendor-native-closure.sh <<'SHELL'
set -euo pipefail
work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT
tar -xzf "$1" -C "$work"
printf closure >"$work/xberg-ffi-fixture/lib/libclosure.so"
if [ "${DROP_NOTICE:-false}" = true ]; then
  rm "$work/xberg-ffi-fixture/THIRD_PARTY_LICENSES.md"
fi
tar -czf "$1" -C "$work" .
SHELL
	printf 'exit 0\n' >scripts/ci/verify-glibc-floor.sh
}

@test "C FFI workflow should verify complete notice bytes after a nested closure archive repack" {
	prepare_c_ffi_closure_fixture
	run bash -e -o pipefail "$XBERG_WORK/step.sh"
	xberg_assert_status 0
	xberg_assert_output_contains 'Verified 2 canonical notice files'
	verify_archive dist/c-ffi/xberg-ffi-fixture.tar.gz xberg-ffi-fixture
	[ "$(tar -xOzf dist/c-ffi/xberg-ffi-fixture.tar.gz ./xberg-ffi-fixture/lib/libclosure.so)" = closure ]
}

@test "C FFI workflow should reject a nested closure repack that drops a canonical notice" {
	prepare_c_ffi_closure_fixture
	export DROP_NOTICE=true
	run bash -e -o pipefail "$XBERG_WORK/step.sh"
	xberg_assert_status 1
	xberg_assert_output_contains 'expected one xberg-ffi-fixture/THIRD_PARTY_LICENSES.md, found 0'
}

check_dart_release_prerequisites() {
	run python3 - "${1:-$REPOSITORY_ROOT/.github/workflows/publish.yaml}" <<'PYTHON'
import re, sys
from pathlib import Path

source = Path(sys.argv[1]).read_text()

def job(name):
    matches = re.findall(rf"^  {name}:\n(.*?)(?=^  [a-z][a-z0-9-]+:|\Z)", source, re.M | re.S)
    assert len(matches) == 1, f"expected one {name} job, found {len(matches)}"
    return matches[0]

upload = job("upload-dart-release")
steps = re.findall(r"^      - .*?(?=^      - |\Z)", upload, re.M | re.S)
checkouts = [(index, step) for index, step in enumerate(steps) if "uses: actions/checkout@" in step]
assert len(checkouts) == 1, f"Dart upload needs one source checkout, found {len(checkouts)}"
packaging = [index for index, step in enumerate(steps) if "name: Package server natives as tarballs" in step]
assert len(packaging) == 1, f"expected one Dart archive packaging step, found {len(packaging)}"
index, checkout = checkouts[0]
assert index < packaging[0], "Dart source checkout must precede packaging"
producer = job("dart-server-natives")
pin = re.findall(r"uses: (actions/checkout@\S+)", producer)
assert len(pin) == 1 and f"uses: {pin[0]}" in checkout, "Dart checkout must match its producer's action pin"
for setting in ("ref: ${{ needs.prepare.outputs.checkout_ref }}", "fetch-depth: 0", "submodules: false", "persist-credentials: false"):
    assert setting in checkout, f"Dart source checkout missing {setting}"
for excluded in ("if:", "path:", "sparse-checkout:"):
    assert not re.search(rf"^\s+{excluded}", checkout, re.M), f"Dart checkout must provide unconditional root source: {excluded}"
assert "python3 scripts/ci/package-notices.py stage" in steps[packaging[0]], "Dart packaging helper was not examined"
print("Verified 1 Dart release source checkout before 1 archive packaging step")
PYTHON
}

@test "Dart upload job should check out its prepared release source before using canonical notices" {
	check_dart_release_prerequisites
	xberg_assert_status 0
	xberg_assert_output_contains 'Verified 1 Dart release source checkout before 1 archive packaging step'
}

@test "Dart prerequisite guard should reject removing its source checkout on a fresh runner" {
	python3 - "$REPOSITORY_ROOT/.github/workflows/publish.yaml" "$XBERG_WORK/no-checkout.yaml" <<'PYTHON'
import re, sys
from pathlib import Path
source = Path(sys.argv[1]).read_text()
start = source.index("  upload-dart-release:\n")
end = source.index("\n  swift-target:", start)
job = source[start:end]
job = re.sub(r"^      - uses: actions/checkout@.*?(?=^      - |\Z)", "", job, flags=re.M | re.S)
Path(sys.argv[2]).write_text(source[:start] + job + source[end:])
PYTHON
	check_dart_release_prerequisites "$XBERG_WORK/no-checkout.yaml"
	xberg_assert_status 1
	xberg_assert_output_contains 'Dart upload needs one source checkout, found 0'
}
