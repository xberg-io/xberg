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

@test "musl Elixir publish should smoke the checksummed archive with its relative SONAME links" {
	ln -s libonnxruntime.so.1 build-output/libonnxruntime.so
	printf codec >build-output/libheif.so.1
	extract_run elixir-natives-musl 'Package NIF as RustlerPrecompiled tarball'
	run bash "$XBERG_WORK/step.sh"
	xberg_assert_status 0

	cat >scripts/ci/smoke-test-musl-elixir-nif.sh <<'SH'
set -euo pipefail
name=libxberg_nif-v1.4.0-nif-2.16-fixture-target.so.tar.gz
test "$(basename "$1")" = "$name"
test "$(cat "$2")" = "$(sha256sum "$1" | cut -d ' ' -f 1)  $name"
tar -xzf "$1" -C "$XBERG_WORK/staged"
test "$(cat "$XBERG_WORK/staged/${name%.tar.gz}")" = native
test "$(readlink "$XBERG_WORK/staged/libonnxruntime.so")" = libonnxruntime.so.1
test "$(cat "$XBERG_WORK/staged/libonnxruntime.so")" = closure
test "$(cat "$XBERG_WORK/staged/libheif.so.1")" = codec
echo "archive reached smoke"
SH
	mkdir staged
	extract_run elixir-natives-musl 'Smoke-test the staged musl Elixir NIF archive'
	run bash "$XBERG_WORK/step.sh"
	xberg_assert_status 0
	xberg_assert_output_contains 'archive reached smoke'

	printf corruption >>dist/elixir-natives/libxberg_nif-v1.4.0-nif-2.16-fixture-target.so.tar.gz
	run bash "$XBERG_WORK/step.sh"
	xberg_assert_status 1
	xberg_assert_output_contains 'FAILED'
	[[ "$output" != *'archive reached smoke'* ]]
}

@test "musl Elixir dry run should resolve the source package version" {
	mkdir -p packages/elixir
	printf '      version: "4.5.6",\n' >packages/elixir/mix.exs
	export RELEASE_VERSION=0.0.0-dryrun-abcdef DRY_RUN=true
	extract_run elixir-natives-musl 'Resolve Elixir NIF version'
	run bash "$XBERG_WORK/step.sh"
	xberg_assert_status 0
	grep -Fxq 'version=4.5.6' "$GITHUB_OUTPUT"
	: >"$GITHUB_OUTPUT"
	export DRY_RUN=false RELEASE_VERSION=9.8.7
	run bash "$XBERG_WORK/step.sh"
	xberg_assert_status 0
	grep -Fxq 'version=9.8.7' "$GITHUB_OUTPUT"
}

@test "Elixir rebuild upload should reject failed legs and a missing current-run archive" {
	extract_run prepare 'List expected Elixir NIF assets'
	bash "$XBERG_WORK/step.sh"
	export NIF_ASSETS
	NIF_ASSETS="$(sed -n 's/^names=//p' "$GITHUB_OUTPUT")"
	# Failed legs are rejected by xberg-io/actions/require-job-results (tested in that repo);
	# here the gate must be the job's first step and require every NIF build job. ~keep
	run python3 - "$REPOSITORY_ROOT/.github/workflows/publish.yaml" <<'PYTHON'
import re
import sys
text = open(sys.argv[1]).read()
job = re.search(r"(?ms)^  upload-elixir-release:\n(.*?)(?=^  [a-z][a-z0-9-]+:\n)", text).group(1)
needs = re.search(r"(?m)^    needs: \[(.*)\]$", job).group(1).split(", ")
gate = re.split(r"(?m)^(?=      - )", job.split("    steps:\n", 1)[1])[1]
assert "        uses: xberg-io/actions/require-job-results@v1\n" in gate, gate
assert "          needs: ${{ toJSON(needs) }}\n" in gate, gate
assert "succeeded-or-skipped" not in gate, gate
required = gate.split("          succeeded: |\n", 1)[1].split()
expected = ["elixir-natives", "elixir-natives-musl", "elixir-natives-manylinux"]
assert required == expected, required
assert set(expected) <= set(needs), needs
print("gate requires", " ".join(required))
PYTHON
	xberg_assert_status 0
	xberg_assert_output_contains 'gate requires elixir-natives elixir-natives-musl elixir-natives-manylinux'

	extract_run upload-elixir-release 'Verify current-run Elixir archives'
	mkdir -p dist/elixir
	local -a names
	IFS=, read -ra names <<<"$NIF_ASSETS"
	for name in "${names[@]}"; do
		printf archive >"dist/elixir/$name"
	done
	run bash "$XBERG_WORK/step.sh"
	xberg_assert_status 0
	rm "dist/elixir/${names[5]}"
	run bash "$XBERG_WORK/step.sh"
	xberg_assert_status 1
	xberg_assert_output_contains "${names[5]}"
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

@test "Elixir release inventory should require every loader archive by its exact name" {
	extract_run prepare 'List expected Elixir NIF assets'
	bash "$XBERG_WORK/step.sh"
	python3 - "$GITHUB_OUTPUT" "$REPOSITORY_ROOT/packages/elixir/lib/xberg/native.ex" <<'PYTHON'
import re, sys
from pathlib import Path
csv, header, *names, end = Path(sys.argv[1]).read_text().splitlines()
assert header == "multiline<<EOF" and end == "EOF"
assert csv == "names=" + ",".join(names)
source = Path(sys.argv[2]).read_text()
targets = re.search(r"targets: \[(.*?)\]", source, re.S).group(1)
expected = set()
for target in re.findall(r'"([^"]+)"', targets):
    prefix, ext = ("", "dll") if "windows" in target else ("lib", "so")
    expected.add(f"{prefix}xberg_nif-v1.4.0-nif-2.16-{target}.{ext}.tar.gz")
assert len(expected) == len(names) == 6
assert set(names) == expected
PYTHON
}

prepare_cli_notice_gate() {
	mkdir -p .notice-tools/scripts/ci .release-notices dist/cli
	# The verifier comes from the workflow revision, the canonical bytes from the release
	# checkout; keeping them apart proves the gate reads the root it is given. ~keep
	cp scripts/ci/package-notices.py .notice-tools/scripts/ci/
	cp LICENSE THIRD_PARTY_LICENSES.md .release-notices/
	export NOTICE_ROOT="$XBERG_WORK/.release-notices"
}

build_cli_archive() {
	local asset="$1" defect="${2:-}" stage
	stage="${asset%.tar.gz}"
	stage="${stage%.zip}"
	mkdir -p "cli-stage/$stage"
	cp LICENSE THIRD_PARTY_LICENSES.md "cli-stage/$stage/"
	printf native >"cli-stage/$stage/xberg"
	case "$defect" in
	missing) rm "cli-stage/$stage/THIRD_PARTY_LICENSES.md" ;;
	stale) printf 'corrupt\n' >"cli-stage/$stage/THIRD_PARTY_LICENSES.md" ;;
	esac
	case "$asset" in
	*.zip)
		python3 - "dist/cli/$asset" "$stage" <<'PYTHON'
import sys, zipfile
from pathlib import Path
asset, stage = sys.argv[1], sys.argv[2]
with zipfile.ZipFile(asset, "w") as archive:
    for path in sorted((Path("cli-stage") / stage).iterdir()):
        archive.write(path, f"{stage}/{path.name}")
PYTHON
		;;
	*)
		tar -czf "dist/cli/$asset" -C cli-stage "$stage"
		;;
	esac
}

run_cli_notice_gate() {
	extract_run upload-cli-release 'Verify notice bytes in every CLI archive'
	export CLI_ASSETS="$1"
	run bash -e -o pipefail "$XBERG_WORK/step.sh"
}

@test "CLI release upload should verify notice bytes inside every collected archive" {
	local count=0 asset
	local -a assets=(
		xberg-cli-aarch64-apple-darwin.tar.gz
		xberg-cli-x86_64-unknown-linux-gnu.tar.gz
		xberg-cli-x86_64-pc-windows-msvc.zip
		xberg-cli-x86_64-pc-windows-msvc-no-gpl-lgpl.zip
		xberg-cli-binstall-x86_64-unknown-linux-musl.tar.gz
	)
	prepare_cli_notice_gate
	local assets_multiline=""
	for asset in "${assets[@]}"; do
		count=$((count + 1))
		build_cli_archive "$asset"
		assets_multiline+="$asset"$'\n'
	done
	# The remaining five prepare names are absent on purpose: a leg that failed to build leaves
	# no archive, and the ones that did build must still be byte-checked here. ~keep
	run_cli_notice_gate "$assets_multiline"
	xberg_assert_status 0
	xberg_assert_output_contains "Verified notices in $count CLI archives"
	xberg_assert_output_contains "Verified 2 canonical notice files in dist/cli/xberg-cli-x86_64-pc-windows-msvc-no-gpl-lgpl.zip"
	xberg_assert_output_contains "Verified 2 canonical notice files in dist/cli/xberg-cli-x86_64-unknown-linux-gnu.tar.gz"
}

@test "CLI notice gate should reject an archive that dropped a notice, naming archive and file" {
	prepare_cli_notice_gate
	build_cli_archive xberg-cli-x86_64-unknown-linux-gnu.tar.gz
	build_cli_archive xberg-cli-x86_64-pc-windows-msvc-no-gpl-lgpl.zip missing
	run_cli_notice_gate "$(printf '%s\n' \
		xberg-cli-x86_64-unknown-linux-gnu.tar.gz \
		xberg-cli-x86_64-pc-windows-msvc-no-gpl-lgpl.zip)"
	xberg_assert_status 1
	xberg_assert_output_contains 'dist/cli/xberg-cli-x86_64-pc-windows-msvc-no-gpl-lgpl.zip: expected one xberg-cli-x86_64-pc-windows-msvc-no-gpl-lgpl/THIRD_PARTY_LICENSES.md, found 0'
	[[ "$output" != *"Verified notices in"* ]]
}

@test "CLI notice gate should reject truncated notice bytes rather than a present file name" {
	prepare_cli_notice_gate
	build_cli_archive xberg-cli-aarch64-unknown-linux-musl.tar.gz stale
	run_cli_notice_gate $'xberg-cli-aarch64-unknown-linux-musl.tar.gz\n'
	xberg_assert_status 1
	xberg_assert_output_contains 'dist/cli/xberg-cli-aarch64-unknown-linux-musl.tar.gz: xberg-cli-aarch64-unknown-linux-musl/THIRD_PARTY_LICENSES.md differs from canonical THIRD_PARTY_LICENSES.md'
	[[ "$output" != *"Verified notices in"* ]]
}

@test "CLI notice gate should fail closed on an empty archive, an empty set or a fully skipped build" {
	prepare_cli_notice_gate
	run_cli_notice_gate ''
	xberg_assert_status 1
	xberg_assert_output_contains 'no expected CLI archives to verify'

	run_cli_notice_gate $'xberg-cli-x86_64-apple-darwin.tar.gz\n'
	xberg_assert_status 1
	xberg_assert_output_contains 'no staged CLI archive was verified'

	: >dist/cli/xberg-cli-x86_64-apple-darwin.tar.gz
	run_cli_notice_gate $'xberg-cli-x86_64-apple-darwin.tar.gz\n'
	xberg_assert_status 1
	xberg_assert_output_contains 'empty CLI archive: dist/cli/xberg-cli-x86_64-apple-darwin.tar.gz'
}

prepare_android_aar_notice_step() {
	mkdir -p .release-tools/scripts/ci packages/kotlin-android/build/outputs/aar
	cp "$REPOSITORY_ROOT/scripts/ci/android-aar-notices.py" \
		"$REPOSITORY_ROOT/scripts/ci/verify-android-jni.py" \
		"$REPOSITORY_ROOT/scripts/ci/package-notices.py" \
		.release-tools/scripts/ci/
	cp "$REPOSITORY_ROOT/scripts/ci/android-aar-notices.py" scripts/ci/
	xberg_stub gradle 'exit 0'
	extract_run publish-kotlin-android 'Inject canonical notices into the release AAR'
}

build_android_aar_fixture() {
	local destination="${1:-packages/kotlin-android/build/outputs/aar/xberg-android-release.aar}"
	python3 - "$destination" <<'PYTHON'
import struct
import sys
import zipfile
from pathlib import Path


def elf(machine):
    header = bytearray(64)
    header[:7] = b"\x7fELF\x02\x01\x01"
    struct.pack_into("<HH", header, 16, 3, machine)
    struct.pack_into("<I", header, 20, 1)
    struct.pack_into("<Q", header, 32, 64)
    struct.pack_into("<3H", header, 52, 64, 56, 1)
    segment = struct.pack("<II6Q", 1, 5, 120, 120, 0, 8, 8, 8)
    return bytes(header) + segment + bytes(8)


path = Path(sys.argv[1])
path.parent.mkdir(parents=True, exist_ok=True)
with zipfile.ZipFile(path, "w") as archive:
    archive.writestr("AndroidManifest.xml", '<manifest package="io.xberg"/>')
    archive.writestr("classes.jar", b"classes")
    archive.writestr("jni/arm64-v8a/libxberg_jni.so", elf(183))
    archive.writestr("jni/x86_64/libxberg_jni.so", elf(62))
    archive.writestr("R.txt", b"")
PYTHON
}

@test "Android AAR workflow step should inject canonical notices and keep the archive valid" {
	prepare_android_aar_notice_step
	build_android_aar_fixture
	run bash -e -o pipefail "$XBERG_WORK/step.sh"
	xberg_assert_status 0
	xberg_assert_output_contains 'Injected canonical notices into packages/kotlin-android/build/outputs/aar/xberg-android-release.aar'
	python3 - <<'PYTHON'
import zipfile
from pathlib import Path

aar = Path("packages/kotlin-android/build/outputs/aar/xberg-android-release.aar")
with zipfile.ZipFile(aar) as archive:
    names = [entry.filename for entry in archive.infolist()]
    assert len(names) == len(set(names)), names
    assert archive.read("META-INF/LICENSE") == Path("LICENSE").read_bytes()
    assert archive.read("META-INF/THIRD_PARTY_LICENSES.md") == Path("THIRD_PARTY_LICENSES.md").read_bytes()
    assert archive.read("AndroidManifest.xml") == b'<manifest package="io.xberg"/>'
    assert archive.read("classes.jar") == b"classes"
    assert archive.read("R.txt") == b""
    for abi in ("arm64-v8a", "x86_64"):
        assert archive.read(f"jni/{abi}/libxberg_jni.so")[:4] == b"\x7fELF"
print("archive kept its entries and gained both notices")
PYTHON
}

@test "Android AAR notice gate should reject a missing notice, naming archive and file" {
	prepare_android_aar_notice_step
	build_android_aar_fixture
	run python3 scripts/ci/android-aar-notices.py verify --canonical-root . \
		packages/kotlin-android/build/outputs/aar/xberg-android-release.aar
	xberg_assert_status 1
	xberg_assert_output_contains \
		'packages/kotlin-android/build/outputs/aar/xberg-android-release.aar: expected one META-INF/LICENSE, found 0'
	[[ "$output" != *"Verified"* ]]
}

@test "Android AAR notice gate should reject truncated notice bytes, naming archive and file" {
	prepare_android_aar_notice_step
	build_android_aar_fixture
	bash -e -o pipefail "$XBERG_WORK/step.sh"
	python3 - <<'PYTHON'
import zipfile
from pathlib import Path

aar = Path("packages/kotlin-android/build/outputs/aar/xberg-android-release.aar")
payload = {}
with zipfile.ZipFile(aar) as archive:
    for entry in archive.infolist():
        payload[entry.filename] = archive.read(entry)
payload["META-INF/THIRD_PARTY_LICENSES.md"] = b"truncated"
with zipfile.ZipFile(aar, "w") as archive:
    for name, content in payload.items():
        archive.writestr(name, content)
PYTHON
	run python3 scripts/ci/android-aar-notices.py verify --canonical-root . \
		packages/kotlin-android/build/outputs/aar/xberg-android-release.aar
	xberg_assert_status 1
	xberg_assert_output_contains \
		'packages/kotlin-android/build/outputs/aar/xberg-android-release.aar: META-INF/THIRD_PARTY_LICENSES.md differs from canonical THIRD_PARTY_LICENSES.md'
	[[ "$output" != *"Verified"* ]]
}

@test "Android AAR workflow step should fail closed when the build produced no archive" {
	prepare_android_aar_notice_step
	run bash -e -o pipefail "$XBERG_WORK/step.sh"
	xberg_assert_status 1
	xberg_assert_output_contains 'expected one release AAR, found 0'
	[[ "$output" != *"Injected canonical notices"* ]]
}
