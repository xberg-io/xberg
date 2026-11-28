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
assert key == ["onnx-v5-"] and restores == ["onnx-v5-", "onnx-v5-"], (key, restores)
PYTHON
}

@test "ORT setup should pin a NuGet package and a PyPI wheel for every runtime callers select" {
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
inline = 0
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
            elif split := re.fullmatch(r"\$\{\{ matrix\.target == 'x86_64-apple-darwin' && '([0-9.]+)' \|\| '([0-9.]+)' \}\}", pin):
                assert split[1] == intel, (workflow, pin)
                versions.add(split[2])
            elif not (pin == intel and "matrix.target == 'x86_64-apple-darwin'" in step):
                versions.add(pin)
    assert remaining == 0, (workflow, remaining)
    for pin in re.findall(r"(?m)^\s+ort_version=([^\s]+)$", text):
        inline += 1
        versions.add(pin)
for dockerfile in (root / "docker").glob("Dockerfile*"):
    versions.update(re.findall(r"(?m)^ARG ONNXRUNTIME_VERSION=([^\s]+)$", dockerfile.read_text()))
assert callers > 1 and inline == 2, (callers, inline)
assert all(re.fullmatch(r"[0-9]+\.[0-9]+\.[0-9]+", version) for version in versions), versions

names = Counter()
wheels = []
for line in (scripts / "checksums.sha256").read_text().splitlines():
    if not line.strip() or line.lstrip().startswith("#"):
        continue
    fields = line.split()
    assert len(fields) in (2, 4), line
    assert re.fullmatch(r"[a-f0-9]{64}", fields[0]), line
    names[fields[1]] += 1
    if len(fields) == 4:
        _, name, rid, url = fields
        assert name.startswith("onnxruntime-") and name.endswith(".whl"), line
        assert url.startswith("https://files.pythonhosted.org/packages/") and url.endswith("/" + name), line
        wheels.append((name, rid))
    else:
        assert re.fullmatch(r"microsoft\.ml\.onnxruntime\.[0-9.]+\.nupkg", fields[1]), line
assert all(count == 1 for count in names.values()), names
for version in versions | {intel}:
    assert names[f"microsoft.ml.onnxruntime.{version}.nupkg"] == 1, version
wanted = [(version, rid) for version in versions for rid in ("linux-x64", "linux-arm64", "osx-arm64")]
wanted.append((intel, "osx-x64"))
for version, rid in wanted:
    matching = [name for name, row_rid in wheels if row_rid == rid and name.startswith(f"onnxruntime-{version}-")]
    assert len(matching) == 1, (version, rid, matching)
print(f"{len(names)} pins checked across {callers} action call sites and {inline} inline stages")
PYTHON
}

runtime_root() {
	printf '%s/onnxruntime/onnxruntime-linux-%s-1.32.0' "$RUNNER_TEMP" "$1"
}

@test "Linux ORT setup should copy cached shared libraries and preserve ambient system flags" {
	root="$(runtime_root x64)"
	mkdir -p "$root/lib"
	printf 'runtime bytes\n' >"$root/lib/libonnxruntime.so.1.32.0"
	ln -s libonnxruntime.so.1.32.0 "$root/lib/libonnxruntime.so.1"
	ln -s libonnxruntime.so.1 "$root/lib/libonnxruntime.so"
	run bash "$REPOSITORY_ROOT/scripts/ci/actions/setup-onnx-runtime/linux.sh" 1.32.0 'node output' x64 system
	xberg_assert_status 0
	xberg_assert_file "$GITHUB_WORKSPACE/node output/libonnxruntime.so.1.32.0" 'runtime bytes'
	xberg_assert_github_env "ORT_LIB_LOCATION=$root/lib
ORT_PREFER_DYNAMIC_LINK=1
ORT_SKIP_DOWNLOAD=1
ORT_STRATEGY=system
ORT_DYLIB_PATH=$root/lib/libonnxruntime.so
LD_LIBRARY_PATH=$root/lib:$GITHUB_WORKSPACE/node output:ambient-loader
LIBRARY_PATH=$root/lib:$GITHUB_WORKSPACE/node output:ambient-linker
RUSTFLAGS=-C debuginfo=1 -L $root/lib -C link-arg=-Wl,-rpath,\$ORIGIN"
	[ ! -e "$XBERG_TRACE" ]
}

@test "Linux ORT setup should point ORT_DYLIB_PATH at the runtime, never the providers library" {
	root="$(runtime_root x64)"
	mkdir -p "$root/lib"
	printf 'providers bytes\n' >"$root/lib/libonnxruntime_providers_shared.so"
	printf 'runtime bytes\n' >"$root/lib/libonnxruntime.so"
	run bash "$REPOSITORY_ROOT/scripts/ci/actions/setup-onnx-runtime/linux.sh" 1.32.0 output x64 system
	xberg_assert_status 0
	[ "$(sed -n 's/^ORT_DYLIB_PATH=//p' "$GITHUB_ENV")" = "$root/lib/libonnxruntime.so" ]
}

@test "Linux ORT setup should fail without exports when the runtime has no unversioned name" {
	root="$(runtime_root x64)"
	mkdir -p "$root/lib"
	printf 'providers bytes\n' >"$root/lib/libonnxruntime_providers_shared.so"
	printf 'runtime bytes\n' >"$root/lib/libonnxruntime.so.1.32.0"
	run bash "$REPOSITORY_ROOT/scripts/ci/actions/setup-onnx-runtime/linux.sh" 1.32.0 output x64 system
	xberg_assert_status 1
	xberg_assert_output_contains 'libonnxruntime.so missing from the staged ONNX Runtime'
	xberg_assert_github_env ''
}

@test "Linux bundled ORT setup should stage runtime libraries without overriding Cargo strategy or flags" {
	root="$(runtime_root aarch64)"
	mkdir -p "$root/lib"
	printf 'arm runtime\n' >"$root/lib/libonnxruntime.so"
	run bash "$REPOSITORY_ROOT/scripts/ci/actions/setup-onnx-runtime/linux.sh" 1.32.0 bindings arm64 bundled
	xberg_assert_status 0
	xberg_assert_file "$GITHUB_WORKSPACE/bindings/libonnxruntime.so" 'arm runtime'
	xberg_assert_github_env "LD_LIBRARY_PATH=$root/lib:$GITHUB_WORKSPACE/bindings:ambient-loader
LIBRARY_PATH=$root/lib:$GITHUB_WORKSPACE/bindings:ambient-linker"
	[ "$(wc -l <"$GITHUB_ENV" | tr -d '[:space:]')" = 2 ]
	! grep -Eq '^(ORT_|RUSTFLAGS=)' "$GITHUB_ENV"
	[ ! -e "$XBERG_TRACE" ]
}

@test "Linux ORT setup should reject unsupported architecture before downloading or exporting" {
	run bash "$REPOSITORY_ROOT/scripts/ci/actions/setup-onnx-runtime/linux.sh" 1.32.0 output riscv64
	xberg_assert_status 1
	xberg_assert_output_contains 'Unsupported Linux arch-id: riscv64'
	xberg_assert_github_env ''
	[ ! -e "$XBERG_TRACE" ]
	[ ! -e "$GITHUB_WORKSPACE/output" ]
}

@test "Linux ORT setup should reject a cached archive directory with no lib directory" {
	mkdir -p "$(runtime_root x64)"
	run bash "$REPOSITORY_ROOT/scripts/ci/actions/setup-onnx-runtime/linux.sh" 1.32.0 output x64
	xberg_assert_status 1
	xberg_assert_output_contains 'ONNX Runtime lib directory missing'
	xberg_assert_github_env ''
	[ ! -e "$XBERG_TRACE" ]
}

@test "Linux ORT setup should reject a cached lib directory containing no shared libraries" {
	mkdir -p "$(runtime_root x64)/lib"
	printf 'unrelated\n' >"$(runtime_root x64)/lib/README"
	run bash "$REPOSITORY_ROOT/scripts/ci/actions/setup-onnx-runtime/linux.sh" 1.32.0 output x64
	xberg_assert_status 1
	xberg_assert_output_contains 'No ONNX Runtime libraries found'
	xberg_assert_github_env ''
	[ ! -e "$GITHUB_WORKSPACE/output" ]
}

# Writes a NuGet package and a PyPI wheel fixture, a manifest pinning both, and a curl that serves
# them. ORT_NUGET_MODE and ORT_PYPI_MODE pick ok, fail (exit 22) or tamper for each source.
prepare_sources() {
	local rid="$1" version="$2" platform="$3" package_sum wheel_sum
	export ORT_FIXTURE_PACKAGE="$BATS_TEST_TMPDIR/package.nupkg"
	export ORT_FIXTURE_WHEEL="$BATS_TEST_TMPDIR/runtime.whl"
	export ORT_CHECKSUMS_FILE="$BATS_TEST_TMPDIR/trusted.sha256"
	export ORT_NUGET_MODE=ok ORT_PYPI_MODE=ok
	ORT_WHEEL="onnxruntime-$version-cp312-cp312-manylinux_2_28_$platform.whl"
	python3 - "$ORT_FIXTURE_PACKAGE" "$ORT_FIXTURE_WHEEL" "$rid" "$version" <<'PYTHON'
import sys
import zipfile

package, wheel, rid, version = sys.argv[1:]
with zipfile.ZipFile(package, "w") as archive:
    archive.writestr(f"runtimes/{rid}/native/libonnxruntime.so", "nuget runtime\n")
    archive.writestr(f"runtimes/{rid}/native/libonnxruntime_providers_shared.so", "nuget providers\n")
    archive.writestr("runtimes/linux-other/native/libonnxruntime.so", "wrong runtime\n")
    archive.writestr("build/native/include/onnxruntime_c_api.h", "header\n")
with zipfile.ZipFile(wheel, "w") as archive:
    archive.writestr(f"onnxruntime/capi/libonnxruntime.so.{version}", "pypi runtime\n")
    archive.writestr("onnxruntime/capi/libonnxruntime_providers_shared.so", "pypi providers\n")
    archive.writestr("onnxruntime/capi/onnxruntime_pybind11_state.so", "python extension\n")
PYTHON
	package_sum="$(shasum -a 256 "$ORT_FIXTURE_PACKAGE")"
	wheel_sum="$(shasum -a 256 "$ORT_FIXTURE_WHEEL")"
	{
		printf '%s  microsoft.ml.onnxruntime.%s.nupkg\n' "${package_sum%% *}" "$version"
		printf '%s  %s  %s  https://files.pythonhosted.org/packages/aa/bb/%s\n' "${wheel_sum%% *}" "$ORT_WHEEL" "$rid" "$ORT_WHEEL"
	} >"$ORT_CHECKSUMS_FILE"
	xberg_stub curl \
		'printf "%s\n" "$*" >>"$XBERG_TRACE"' \
		'case "$*" in *--version*) exit 91 ;; *api.nuget.org*) mode="$ORT_NUGET_MODE" fixture="$ORT_FIXTURE_PACKAGE" ;; *files.pythonhosted.org*) mode="$ORT_PYPI_MODE" fixture="$ORT_FIXTURE_WHEEL" ;; *) exit 92 ;; esac' \
		'[ "$mode" != fail ] || exit 22' \
		'while [ "$#" -gt 0 ]; do if [ "$1" = -o ]; then if [ "$mode" = tamper ]; then printf "tampered\n" >"$2"; else cp "$fixture" "$2"; fi; exit; fi; shift; done' \
		'exit 91'
}

downloads() {
	grep -v -e '--version' "$XBERG_TRACE"
}

@test "Linux ORT setup should stage the verified NuGet package as the release layout" {
	root="$(runtime_root aarch64)"
	prepare_sources linux-arm64 1.32.0 aarch64
	run bash "$REPOSITORY_ROOT/scripts/ci/actions/setup-onnx-runtime/linux.sh" 1.32.0 output arm64
	xberg_assert_status 0
	xberg_assert_output_contains 'microsoft.ml.onnxruntime.1.32.0.nupkg: SHA-256 verified'
	xberg_assert_output_contains 'Staged ONNX Runtime 1.32.0 (linux-arm64) from NuGet'
	xberg_assert_file "$root/lib/libonnxruntime.so.1.32.0" 'nuget runtime'
	[ "$(readlink "$root/lib/libonnxruntime.so.1")" = libonnxruntime.so.1.32.0 ]
	[ "$(readlink "$root/lib/libonnxruntime.so")" = libonnxruntime.so.1 ]
	xberg_assert_file "$root/include/onnxruntime_c_api.h" 'header'
	xberg_assert_file "$GITHUB_WORKSPACE/output/libonnxruntime.so.1" 'nuget runtime'
	xberg_assert_file "$GITHUB_WORKSPACE/output/libonnxruntime_providers_shared.so" 'nuget providers'
	grep -Fxq "ORT_LIB_LOCATION=$root/lib" "$GITHUB_ENV"
	[ "$(downloads | wc -l | tr -d '[:space:]')" = 1 ]
	downloads | grep -Fq 'api.nuget.org/v3-flatcontainer/microsoft.ml.onnxruntime/1.32.0/microsoft.ml.onnxruntime.1.32.0.nupkg'
}

@test "Linux ORT setup should fall back to the verified PyPI wheel when the NuGet download fails" {
	root="$(runtime_root x64)"
	prepare_sources linux-x64 1.32.0 x86_64
	export ORT_NUGET_MODE=fail
	run bash "$REPOSITORY_ROOT/scripts/ci/actions/setup-onnx-runtime/linux.sh" 1.32.0 output x64
	xberg_assert_status 0
	xberg_assert_output_contains 'NuGet download of microsoft.ml.onnxruntime.1.32.0.nupkg failed; using the PyPI wheel'
	xberg_assert_output_contains "$ORT_WHEEL: SHA-256 verified"
	xberg_assert_output_contains 'Staged ONNX Runtime 1.32.0 (linux-x64) from PyPI'
	xberg_assert_file "$root/lib/libonnxruntime.so.1.32.0" 'pypi runtime'
	xberg_assert_file "$root/lib/libonnxruntime_providers_shared.so" 'pypi providers'
	[ "$(readlink "$root/lib/libonnxruntime.so.1")" = libonnxruntime.so.1.32.0 ]
	[ "$(readlink "$root/lib/libonnxruntime.so")" = libonnxruntime.so.1 ]
	[ ! -e "$root/lib/onnxruntime_pybind11_state.so" ]
	xberg_assert_file "$GITHUB_WORKSPACE/output/libonnxruntime.so" 'pypi runtime'
	[ "$(sed -n 's/^ORT_DYLIB_PATH=//p' "$GITHUB_ENV")" = "$root/lib/libonnxruntime.so" ]
	[ "$(downloads | wc -l | tr -d '[:space:]')" = 2 ]
	downloads | sed -n 1p | grep -Fq 'api.nuget.org/'
	downloads | sed -n 2p | grep -Fq "https://files.pythonhosted.org/packages/aa/bb/$ORT_WHEEL"
}

@test "Linux ORT setup should fall back to the PyPI wheel when the NuGet package fails verification" {
	root="$(runtime_root x64)"
	prepare_sources linux-x64 1.32.0 x86_64
	export ORT_NUGET_MODE=tamper
	run bash "$REPOSITORY_ROOT/scripts/ci/actions/setup-onnx-runtime/linux.sh" 1.32.0 output x64
	xberg_assert_status 0
	xberg_assert_output_contains 'SHA-256 mismatch for microsoft.ml.onnxruntime.1.32.0.nupkg'
	xberg_assert_output_contains 'failed verification; using the PyPI wheel'
	xberg_assert_file "$root/lib/libonnxruntime.so.1.32.0" 'pypi runtime'
	[ "$(downloads | wc -l | tr -d '[:space:]')" = 2 ]
}

@test "Linux ORT setup should fail without staging when the PyPI wheel fails verification in either cache mode" {
	prepare_sources linux-x64 1.32.0 x86_64
	export ORT_NUGET_MODE=fail ORT_PYPI_MODE=tamper
	for mode in false true; do
		run bash "$REPOSITORY_ROOT/scripts/ci/actions/setup-onnx-runtime/linux.sh" 1.32.0 output x64 system "$mode"
		xberg_assert_status 1
		xberg_assert_output_contains "SHA-256 mismatch for $ORT_WHEEL"
		xberg_assert_github_env ''
		[ ! -e "$(runtime_root x64)" ]
		[ ! -e "$GITHUB_WORKSPACE/output" ]
		[ -z "$(compgen -G "$RUNNER_TEMP/onnxruntime-verified.*" || true)" ]
	done
}

@test "Linux ORT setup should fail without exports when both downloads fail" {
	prepare_sources linux-x64 1.32.0 x86_64
	export ORT_NUGET_MODE=fail ORT_PYPI_MODE=fail
	run bash "$REPOSITORY_ROOT/scripts/ci/actions/setup-onnx-runtime/linux.sh" 1.32.0 output x64
	xberg_assert_status 22
	xberg_assert_github_env ''
	[ ! -e "$GITHUB_WORKSPACE/output" ]
	[ ! -e "$(runtime_root x64)" ]
	[ "$(downloads | wc -l | tr -d '[:space:]')" = 2 ]
	[ "$(downloads | grep -c 'github.com' || true)" = 0 ]
}

@test "Linux ORT setup should refuse a version without NuGet or PyPI pins before any download" {
	run bash "$REPOSITORY_ROOT/scripts/ci/actions/setup-onnx-runtime/linux.sh" 9.9.9 output x64
	xberg_assert_status 1
	xberg_assert_output_contains 'Expected exactly one pinned SHA-256 for microsoft.ml.onnxruntime.9.9.9.nupkg'
	xberg_assert_trace_empty
	xberg_assert_github_env ''
	[ ! -e "$RUNNER_TEMP/onnxruntime" ]
	[ ! -e "$GITHUB_WORKSPACE/output" ]
}

@test "Linux ORT setup should refuse a pinned NuGet package without a pinned wheel before any download" {
	prepare_sources linux-x64 1.32.0 x86_64
	sed -i.bak '/\.whl/d' "$ORT_CHECKSUMS_FILE"
	run bash "$REPOSITORY_ROOT/scripts/ci/actions/setup-onnx-runtime/linux.sh" 1.32.0 output x64
	xberg_assert_status 1
	xberg_assert_output_contains 'Expected exactly one pinned PyPI wheel for ONNX Runtime 1.32.0 (linux-x64)'
	xberg_assert_trace_empty
	[ ! -e "$RUNNER_TEMP/onnxruntime" ]
}

@test "Linux ORT setup should refuse a pinned wheel without a pinned NuGet package before any download" {
	prepare_sources linux-x64 1.32.0 x86_64
	sed -i.bak '/\.nupkg/d' "$ORT_CHECKSUMS_FILE"
	run bash "$REPOSITORY_ROOT/scripts/ci/actions/setup-onnx-runtime/linux.sh" 1.32.0 output x64
	xberg_assert_status 1
	xberg_assert_output_contains 'Expected exactly one pinned SHA-256 for microsoft.ml.onnxruntime.1.32.0.nupkg'
	xberg_assert_trace_empty
}

@test "Linux ORT setup should refuse a wheel pinned to a URL outside PyPI before any download" {
	prepare_sources linux-x64 1.32.0 x86_64
	sed -i.bak "s#https://files.pythonhosted.org/packages/aa/bb/#https://mirror.invalid/#" "$ORT_CHECKSUMS_FILE"
	run bash "$REPOSITORY_ROOT/scripts/ci/actions/setup-onnx-runtime/linux.sh" 1.32.0 output x64
	xberg_assert_status 1
	xberg_assert_output_contains "Pinned PyPI wheel $ORT_WHEEL has an unexpected URL"
	xberg_assert_trace_empty
}

@test "Linux ORT verified mode should ignore the extracted-tree cache and stage fresh verified bytes" {
	root="$(runtime_root x64)"
	mkdir -p "$root/lib"
	printf 'stale cache\n' >"$root/lib/libonnxruntime.so"
	prepare_sources linux-x64 1.32.0 x86_64
	run bash "$REPOSITORY_ROOT/scripts/ci/actions/setup-onnx-runtime/linux.sh" 1.32.0 output x64 system true
	xberg_assert_status 0
	xberg_assert_file "$GITHUB_WORKSPACE/output/libonnxruntime.so" 'nuget runtime'
	first_root="$(sed -n 's/^ORT_LIB_LOCATION=//p' "$GITHUB_ENV")"
	[[ "$first_root" == "$RUNNER_TEMP"/onnxruntime-verified.*/onnxruntime-linux-x64-1.32.0/lib ]]

	: >"$GITHUB_ENV"
	run bash "$REPOSITORY_ROOT/scripts/ci/actions/setup-onnx-runtime/linux.sh" 1.32.0 output x64 system true
	xberg_assert_status 0
	second_root="$(sed -n 's/^ORT_LIB_LOCATION=//p' "$GITHUB_ENV")"
	[[ "$second_root" == "$RUNNER_TEMP"/onnxruntime-verified.*/onnxruntime-linux-x64-1.32.0/lib ]]
	[ "$second_root" != "$first_root" ]
	[ "$(downloads | wc -l | tr -d '[:space:]')" = 2 ]
	xberg_assert_file "$root/lib/libonnxruntime.so" 'stale cache'
}

run_stage_runtime() {
	run bash "$REPOSITORY_ROOT/scripts/ci/actions/setup-onnx-runtime/stage-runtime.sh" \
		1.32.0 linux-x64 "$BATS_TEST_TMPDIR/ort-root" "$ORT_CHECKSUMS_FILE"
}

@test "ORT staging should force HTTPS and omit --retry-all-errors when curl predates it" {
	prepare_sources linux-x64 1.32.0 x86_64
	xberg_stub curl \
		'printf "%s\n" "$*" >>"$XBERG_TRACE"' \
		'case "$*" in *--version*) exit 2 ;; esac' \
		'while [ "$#" -gt 0 ]; do if [ "$1" = -o ]; then cp "$ORT_FIXTURE_PACKAGE" "$2"; exit; fi; shift; done' \
		'exit 91'
	run_stage_runtime
	xberg_assert_status 0
	download="$(downloads)"
	[[ "$download" == '--proto =https --proto-redir =https --retry 5 --retry-delay 5 -fsSL -o '* ]]
	[[ "$download" != *--retry-all-errors* ]]
}

@test "ORT staging should retry all errors when curl supports it" {
	prepare_sources linux-x64 1.32.0 x86_64
	xberg_stub curl \
		'printf "%s\n" "$*" >>"$XBERG_TRACE"' \
		'case "$*" in *--version*) exit 0 ;; esac' \
		'while [ "$#" -gt 0 ]; do if [ "$1" = -o ]; then cp "$ORT_FIXTURE_PACKAGE" "$2"; exit; fi; shift; done' \
		'exit 91'
	run_stage_runtime
	xberg_assert_status 0
	downloads | grep -Fq -e '--proto =https --proto-redir =https --retry 5 --retry-delay 5 --retry-all-errors -fsSL'
}

@test "ORT staging should extract the NuGet package and the wheel with python3 when unzip is missing" {
	prepare_sources linux-x64 1.32.0 x86_64
	bin="$BATS_TEST_TMPDIR/no-unzip-bin"
	mkdir -p "$bin"
	for tool in bash env dirname mktemp mkdir cp mv ln rm cat sed grep awk tr cut head tail wc basename sort uname shasum sha256sum python3 chmod readlink; do
		path="$(command -v "$tool" || true)"
		[ -z "$path" ] || ln -s "$path" "$bin/$tool"
	done
	cp "$XBERG_STUB_BIN/curl" "$bin/curl"
	PATH="$bin"
	[ -z "$(command -v unzip || true)" ]
	run_stage_runtime
	xberg_assert_status 0
	xberg_assert_file "$BATS_TEST_TMPDIR/ort-root/lib/libonnxruntime.so.1.32.0" 'nuget runtime'
	xberg_assert_file "$BATS_TEST_TMPDIR/ort-root/include/onnxruntime_c_api.h" 'header'
	xberg_assert_file "$BATS_TEST_TMPDIR/ort-root/lib/libonnxruntime_providers_shared.so" 'nuget providers'

	export ORT_NUGET_MODE=fail
	run_stage_runtime
	xberg_assert_status 0
	xberg_assert_file "$BATS_TEST_TMPDIR/ort-root/lib/libonnxruntime.so.1.32.0" 'pypi runtime'
	xberg_assert_file "$BATS_TEST_TMPDIR/ort-root/lib/libonnxruntime_providers_shared.so" 'pypi providers'
	[ ! -e "$BATS_TEST_TMPDIR/ort-root/include" ]
}
