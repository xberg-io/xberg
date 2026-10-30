#!/usr/bin/env bats

setup() {
	bats_load_library xberg-bats
	xberg_setup
	xberg_setup_workspace
	REPOSITORY_ROOT="$(cd "$BATS_TEST_DIRNAME/../../.." && pwd -P)"
	export RUNNER_TEMP="$BATS_TEST_TMPDIR/runner"
	mkdir -p "$RUNNER_TEMP"
	export RUSTFLAGS='-C debuginfo=1' LD_LIBRARY_PATH=ambient-loader LIBRARY_PATH=ambient-linker
	unset ORT_LIB_LOCATION ORT_STRATEGY ORT_SKIP_DOWNLOAD ORT_PREFER_DYNAMIC_LINK ORT_DYLIB_PATH
	xberg_stub curl 'printf "%s\n" "$*" >>"$XBERG_TRACE"; exit 93'
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

@test "Linux ORT setup should download and extract the requested aarch64 release layout" {
	root="$(runtime_root aarch64)"
	mkdir -p "$BATS_TEST_TMPDIR/archive/onnxruntime-linux-aarch64-1.24.2/lib"
	printf 'downloaded runtime\n' >"$BATS_TEST_TMPDIR/archive/onnxruntime-linux-aarch64-1.24.2/lib/libonnxruntime.so"
	export ORT_FIXTURE_ARCHIVE="$BATS_TEST_TMPDIR/runtime.tgz"
	tar -czf "$ORT_FIXTURE_ARCHIVE" -C "$BATS_TEST_TMPDIR/archive" onnxruntime-linux-aarch64-1.24.2
	xberg_stub curl \
		'printf "%s\n" "$*" >>"$XBERG_TRACE"' \
		'while [ "$#" -gt 0 ]; do if [ "$1" = -o ]; then cp "$ORT_FIXTURE_ARCHIVE" "$2"; exit; fi; shift; done' \
		'exit 91'
	run bash "$REPOSITORY_ROOT/scripts/ci/actions/setup-onnx-runtime/linux.sh" 1.24.2 downloaded arm64
	xberg_assert_status 0
	xberg_assert_file "$GITHUB_WORKSPACE/downloaded/libonnxruntime.so" 'downloaded runtime'
	xberg_assert_file "$root/lib/libonnxruntime.so" 'downloaded runtime'
	grep -Fq 'releases/download/v1.24.2/onnxruntime-linux-aarch64-1.24.2.tgz' "$XBERG_TRACE"
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
