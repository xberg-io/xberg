#!/usr/bin/env bats

setup() {
	bats_load_library xberg-bats
	xberg_setup
	xberg_setup_workspace
	REPOSITORY_ROOT="$(cd "$BATS_TEST_DIRNAME/../../.." && pwd -P)"
	export RUNNER_TEMP="$BATS_TEST_TMPDIR/runner"
	mkdir -p "$RUNNER_TEMP"
	export RUSTFLAGS='-C debuginfo=1' LD_LIBRARY_PATH=ambient-loader LIBRARY_PATH=ambient-linker
	unset ORT_CHECKSUMS_FILE ORT_LIB_LOCATION ORT_STRATEGY ORT_SKIP_DOWNLOAD ORT_PREFER_DYNAMIC_LINK ORT_DYLIB_PATH
	xberg_stub curl 'printf "%s\n" "$*" >>"$XBERG_TRACE"; exit 93'
}

@test "ORT setup should restore only cache entries from the verified download generation" {
	python3 - "$REPOSITORY_ROOT/.github/actions/setup-onnx-runtime/action.yml" <<'PYTHON'
from pathlib import Path
import re
import sys

action = Path(sys.argv[1]).read_text()
assert action.count("id: cache-onnx") == 1
block = action.split("id: cache-onnx", 1)[1].split("\n    - name:", 1)[0]
key = re.findall(r"(?m)^\s+key: (onnx-v\d+-)", block)
restores = re.findall(r"(?m)^\s+(onnx-v\d+-)", block.split("restore-keys: |", 1)[1])
assert key == ["onnx-v4-"] and restores == ["onnx-v4-", "onnx-v4-"], (key, restores)
PYTHON
}

@test "ORT setup should pin the release archives selected by action callers" {
	python3 - "$REPOSITORY_ROOT" <<'PYTHON'
from collections import Counter
from pathlib import Path
import re
import sys

root = Path(sys.argv[1])
action_path = ".github/actions/setup-onnx-runtime"
scripts = root / "scripts/ci/actions/setup-onnx-runtime"
action = (root / action_path / "action.yml").read_text()
default, = re.findall(r'(?m)^  ort-version:\n(?:    [^\n]*\n)*?    default: "([^"]+)"$', action)
intel, = re.findall(r'(?m)^  ort_version="([^"]+)"$', (scripts / "macos.sh").read_text())
versions = {default}
callers = 0
for workflow in (root / ".github/workflows").glob("*.y*ml"):
    text = workflow.read_text()
    env = re.findall(r"(?m)^\s+ORT_VERSION:[ \t]*([^\n]+)$", text)
    uses = rf"(?m)^\s*(?:- )?uses: \./{re.escape(action_path)}[ \t]*$"
    remaining = len(re.findall(uses, text))
    for step in re.split(r"(?m)^      - ", text):
        if not re.search(uses, step):
            continue
        callers += 1
        remaining -= 1
        pins = re.findall(r"(?m)^\s+ort-version:[ \t]*([^\n]+)$", step)
        for pin in pins:
            pin = pin.strip().strip("'\"")
            if pin == '${{ env.ORT_VERSION }}':
                assert env, workflow
                versions.update(value.strip().strip("'\"") for value in env)
            elif not (pin == intel and "matrix.target == 'x86_64-apple-darwin'" in step):
                versions.add(pin)
    assert remaining == 0, (workflow, remaining)
assert callers > 1, callers
assert all(re.fullmatch(r"[0-9]+\.[0-9]+\.[0-9]+", version) for version in versions), versions

manifest = Counter()
for line in (scripts / "checksums.sha256").read_text().splitlines():
    if line.strip() and not line.lstrip().startswith("#"):
        digest, name = line.split()
        assert re.fullmatch(r"[a-f0-9]{64}", digest), name
        manifest[name] += 1
expected = {f"onnxruntime-osx-x86_64-{intel}.tgz"}
for version in versions:
    for asset in ("linux-x64", "linux-aarch64", "osx-arm64", "win-x64", "win-arm64"):
        ext = "zip" if asset.startswith("win-") else "tgz"
        expected.add(f"onnxruntime-{asset}-{version}.{ext}")
assert all(manifest[name] == 1 for name in expected), {name: manifest[name] for name in expected if manifest[name] != 1}
print(f"{len(expected)} pinned archives checked across {callers} action call sites")
PYTHON
}

runtime_root() {
	printf '%s/onnxruntime/onnxruntime-linux-%s-1.24.2' "$RUNNER_TEMP" "$1"
}

@test "Linux ORT setup should copy cached shared libraries and preserve ambient system flags" {
	root="$(runtime_root x64)"
	mkdir -p "$root/lib"
	printf 'runtime bytes\n' >"$root/lib/libonnxruntime.so.1.24.2"
	run bash "$REPOSITORY_ROOT/scripts/ci/actions/setup-onnx-runtime/linux.sh" 1.24.2 'node output' x64 system
	xberg_assert_status 0
	xberg_assert_file "$GITHUB_WORKSPACE/node output/libonnxruntime.so.1.24.2" 'runtime bytes'
	xberg_assert_github_env "ORT_LIB_LOCATION=$root/lib
ORT_PREFER_DYNAMIC_LINK=1
ORT_SKIP_DOWNLOAD=1
ORT_STRATEGY=system
ORT_DYLIB_PATH=$root/lib/libonnxruntime.so.1.24.2
LD_LIBRARY_PATH=$root/lib:$GITHUB_WORKSPACE/node output:ambient-loader
LIBRARY_PATH=$root/lib:$GITHUB_WORKSPACE/node output:ambient-linker
RUSTFLAGS=-C debuginfo=1 -L $root/lib -C link-arg=-Wl,-rpath,\$ORIGIN"
	[ ! -e "$XBERG_TRACE" ]
}

@test "Linux bundled ORT setup should stage runtime libraries without overriding Cargo strategy or flags" {
	root="$(runtime_root aarch64)"
	mkdir -p "$root/lib"
	printf 'arm runtime\n' >"$root/lib/libonnxruntime.so"
	run bash "$REPOSITORY_ROOT/scripts/ci/actions/setup-onnx-runtime/linux.sh" 1.24.2 bindings arm64 bundled
	xberg_assert_status 0
	xberg_assert_file "$GITHUB_WORKSPACE/bindings/libonnxruntime.so" 'arm runtime'
	xberg_assert_github_env "LD_LIBRARY_PATH=$root/lib:$GITHUB_WORKSPACE/bindings:ambient-loader
LIBRARY_PATH=$root/lib:$GITHUB_WORKSPACE/bindings:ambient-linker"
	[ "$(wc -l <"$GITHUB_ENV" | tr -d '[:space:]')" = 2 ]
	! grep -Eq '^(ORT_|RUSTFLAGS=)' "$GITHUB_ENV"
	[ ! -e "$XBERG_TRACE" ]
}

prepare_ort_archive() {
	local arch="$1" version="$2" checksum
	local name="onnxruntime-linux-$arch-$version"
	mkdir -p "$BATS_TEST_TMPDIR/archive/$name/lib"
	printf 'downloaded runtime\n' >"$BATS_TEST_TMPDIR/archive/$name/lib/libonnxruntime.so"
	export ORT_FIXTURE_ARCHIVE="$BATS_TEST_TMPDIR/runtime.tgz"
	export ORT_CHECKSUMS_FILE="$BATS_TEST_TMPDIR/trusted.sha256"
	tar -czf "$ORT_FIXTURE_ARCHIVE" -C "$BATS_TEST_TMPDIR/archive" "$name"
	checksum="$(shasum -a 256 "$ORT_FIXTURE_ARCHIVE")"
	printf '%s  %s.tgz\n' "${checksum%% *}" "$name" >"$ORT_CHECKSUMS_FILE"
	xberg_stub curl \
		'printf "%s\n" "$*" >>"$XBERG_TRACE"' \
		'while [ "$#" -gt 0 ]; do if [ "$1" = -o ]; then cp "$ORT_FIXTURE_ARCHIVE" "$2"; exit; fi; shift; done' \
		'exit 91'
}

@test "Linux ORT setup should download and extract the requested aarch64 release layout" {
	root="$(runtime_root aarch64)"
	prepare_ort_archive aarch64 1.24.2
	run bash "$REPOSITORY_ROOT/scripts/ci/actions/setup-onnx-runtime/linux.sh" 1.24.2 downloaded arm64
	xberg_assert_status 0
	xberg_assert_file "$GITHUB_WORKSPACE/downloaded/libonnxruntime.so" 'downloaded runtime'
	xberg_assert_file "$root/lib/libonnxruntime.so" 'downloaded runtime'
	xberg_assert_output_contains 'onnxruntime-linux-aarch64-1.24.2.tgz: SHA-256 verified'
	grep -Fq 'releases/download/v1.24.2/onnxruntime-linux-aarch64-1.24.2.tgz' "$XBERG_TRACE"
}

@test "Linux ORT setup should reject an unpinned fresh version before curl" {
	run bash "$REPOSITORY_ROOT/scripts/ci/actions/setup-onnx-runtime/linux.sh" 9.9.9 output x64
	xberg_assert_status 1
	xberg_assert_output_contains 'Expected exactly one pinned SHA-256 for onnxruntime-linux-x64-9.9.9.tgz'
	xberg_assert_trace_empty
	xberg_assert_github_env ''
	[ ! -e "$RUNNER_TEMP/onnxruntime" ]
	[ ! -e "$GITHUB_WORKSPACE/output" ]
}

@test "Linux ORT setup should reject tampered fresh bytes without extraction in either cache mode" {
	prepare_ort_archive x64 1.24.2
	printf 'tampered runtime\n' >"$ORT_FIXTURE_ARCHIVE"
	xberg_stub_exit tar 94
	for mode in false true; do
		run bash "$REPOSITORY_ROOT/scripts/ci/actions/setup-onnx-runtime/linux.sh" 1.24.2 output x64 system "$mode"
		xberg_assert_status 1
		xberg_assert_output_contains 'SHA-256 mismatch for onnxruntime-linux-x64-1.24.2.tgz'
		xberg_assert_github_env ''
		[ ! -e "$RUNNER_TEMP/onnxruntime" ]
		[ ! -e "$GITHUB_WORKSPACE/output" ]
	done
}

@test "Linux ORT verified archive mode should use a trusted custom pin and recheck cache hits" {
	prepare_ort_archive x64 9.9.9
	cached_archive="$RUNNER_TEMP/onnxruntime/onnxruntime-linux-x64-9.9.9.tgz"
	run bash "$REPOSITORY_ROOT/scripts/ci/actions/setup-onnx-runtime/linux.sh" 9.9.9 output x64 system true
	xberg_assert_status 0
	xberg_assert_output_contains 'onnxruntime-linux-x64-9.9.9.tgz: SHA-256 verified'
	xberg_assert_file "$GITHUB_WORKSPACE/output/libonnxruntime.so" 'downloaded runtime'
	cmp "$ORT_FIXTURE_ARCHIVE" "$cached_archive"
	grep -Fq 'releases/download/v9.9.9/onnxruntime-linux-x64-9.9.9.tgz' "$XBERG_TRACE"
	first_root="$(sed -n 's/^ORT_LIB_LOCATION=//p' "$GITHUB_ENV")"
	[[ "$first_root" == "$RUNNER_TEMP"/onnxruntime-verified.*/onnxruntime-linux-x64-9.9.9/lib ]]

	: >"$GITHUB_ENV"
	rm "$XBERG_TRACE"
	run bash "$REPOSITORY_ROOT/scripts/ci/actions/setup-onnx-runtime/linux.sh" 9.9.9 output x64 system true
	xberg_assert_status 0
	xberg_assert_output_contains 'onnxruntime-linux-x64-9.9.9.tgz: SHA-256 verified'
	second_root="$(sed -n 's/^ORT_LIB_LOCATION=//p' "$GITHUB_ENV")"
	[[ "$second_root" == "$RUNNER_TEMP"/onnxruntime-verified.*/onnxruntime-linux-x64-9.9.9/lib ]]
	[ "$second_root" != "$first_root" ]
	xberg_assert_trace_empty
}

@test "Linux ORT verified archive mode should replace a corrupt cache with freshly verified bytes" {
	prepare_ort_archive x64 9.9.9
	cached_archive="$RUNNER_TEMP/onnxruntime/onnxruntime-linux-x64-9.9.9.tgz"
	mkdir -p "$(dirname "$cached_archive")"
	printf 'corrupt cache\n' >"$cached_archive"
	run bash "$REPOSITORY_ROOT/scripts/ci/actions/setup-onnx-runtime/linux.sh" 9.9.9 output x64 system true
	xberg_assert_status 0
	xberg_assert_output_contains 'SHA-256 mismatch for onnxruntime-linux-x64-9.9.9.tgz'
	xberg_assert_output_contains 'onnxruntime-linux-x64-9.9.9.tgz: SHA-256 verified'
	[ "$(wc -l <"$XBERG_TRACE" | tr -d '[:space:]')" = 1 ]
	grep -Fq 'releases/download/v9.9.9/onnxruntime-linux-x64-9.9.9.tgz' "$XBERG_TRACE"
	cmp "$ORT_FIXTURE_ARCHIVE" "$cached_archive"
	xberg_assert_file "$GITHUB_WORKSPACE/output/libonnxruntime.so" 'downloaded runtime'

	printf 'corrupt cache\n' >"$cached_archive"
	printf 'tampered download\n' >"$ORT_FIXTURE_ARCHIVE"
	xberg_stub_exit tar 94
	: >"$GITHUB_ENV"
	rm "$XBERG_TRACE"
	run bash "$REPOSITORY_ROOT/scripts/ci/actions/setup-onnx-runtime/linux.sh" 9.9.9 rejected x64 system true
	xberg_assert_status 1
	[ "$(wc -l <"$XBERG_TRACE" | tr -d '[:space:]')" = 1 ]
	[ "$(grep -Fc 'SHA-256 mismatch for onnxruntime-linux-x64-9.9.9.tgz' <<<"$output")" = 2 ]
	xberg_assert_github_env ''
	[ ! -e "$cached_archive" ]
	[ ! -e "$GITHUB_WORKSPACE/rejected" ]
}

@test "Linux ORT setup should reject unsupported architecture before downloading or exporting" {
	run bash "$REPOSITORY_ROOT/scripts/ci/actions/setup-onnx-runtime/linux.sh" 1.24.2 output riscv64
	xberg_assert_status 1
	xberg_assert_output_contains 'Unsupported Linux arch-id: riscv64'
	xberg_assert_github_env ''
	[ ! -e "$XBERG_TRACE" ]
	[ ! -e "$GITHUB_WORKSPACE/output" ]
}

@test "Linux ORT setup should reject a cached archive directory with no lib directory" {
	mkdir -p "$(runtime_root x64)"
	run bash "$REPOSITORY_ROOT/scripts/ci/actions/setup-onnx-runtime/linux.sh" 1.24.2 output x64
	xberg_assert_status 1
	xberg_assert_output_contains 'ONNX Runtime lib directory missing'
	xberg_assert_github_env ''
	[ ! -e "$XBERG_TRACE" ]
}

@test "Linux ORT setup should reject a cached lib directory containing no shared libraries" {
	mkdir -p "$(runtime_root x64)/lib"
	printf 'unrelated\n' >"$(runtime_root x64)/lib/README"
	run bash "$REPOSITORY_ROOT/scripts/ci/actions/setup-onnx-runtime/linux.sh" 1.24.2 output x64
	xberg_assert_status 1
	xberg_assert_output_contains 'No ONNX Runtime libraries found'
	xberg_assert_github_env ''
	[ ! -e "$GITHUB_WORKSPACE/output" ]
}

@test "Linux ORT setup should preserve a failed download without reporting runtime success" {
	run bash "$REPOSITORY_ROOT/scripts/ci/actions/setup-onnx-runtime/linux.sh" 1.24.2 output x64
	xberg_assert_status 93
	xberg_assert_github_env ''
	[ ! -e "$GITHUB_WORKSPACE/output" ]
	[ "$(wc -l <"$XBERG_TRACE" | tr -d '[:space:]')" = 1 ]
}

@test "prebuilt macOS ORT setup should export only an existing target-specific static library" {
	for target in aarch64-apple-darwin x86_64-apple-darwin; do
		: >"$GITHUB_ENV"
		root="$GITHUB_WORKSPACE/target/onnxruntime/$target/onnxruntime/lib"
		mkdir -p "$root"
		printf 'static runtime\n' >"$root/libonnxruntime.a"
		run bash "$REPOSITORY_ROOT/scripts/ci/actions/setup-prebuilt-onnx/prepare.sh" "$target"
		xberg_assert_status 0
		xberg_assert_github_env "ORT_STRATEGY=system
ORT_LIB_LOCATION=$root
ORT_SKIP_DOWNLOAD=1
ORT_PREFER_DYNAMIC_LINK=1"
		[ "$(wc -l <"$GITHUB_ENV" | tr -d '[:space:]')" = 4 ]
	done
	[ ! -e "$XBERG_TRACE" ]
}

@test "prebuilt ORT setup should leave compiler provisioning available after an unavailable download" {
	run bash "$REPOSITORY_ROOT/scripts/ci/actions/setup-prebuilt-onnx/prepare.sh" aarch64-apple-darwin
	xberg_assert_status 0
	xberg_assert_output_contains 'Will download and build ONNX Runtime during compilation'
	xberg_assert_github_env ''
	[ "$(wc -l <"$XBERG_TRACE" | tr -d '[:space:]')" = 1 ]
}

@test "prebuilt ORT setup should reject an unsupported target before touching downloads" {
	run bash "$REPOSITORY_ROOT/scripts/ci/actions/setup-prebuilt-onnx/prepare.sh" x86_64-unknown-linux-gnu
	xberg_assert_status 1
	xberg_assert_output_contains 'does not support target x86_64-unknown-linux-gnu'
	xberg_assert_github_env ''
	[ ! -e "$XBERG_TRACE" ]
}
