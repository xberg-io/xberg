#!/usr/bin/env bats

setup() {
	bats_load_library xberg-bats
	xberg_setup
	REPOSITORY_ROOT="$(cd "$BATS_TEST_DIRNAME/../../.." && pwd -P)"
	cd "$XBERG_WORK" || return
	mkdir -p scripts/ci target crates/xberg-ffi/include output
	cp "$REPOSITORY_ROOT/scripts/package_cli_artifact.sh" scripts/
	cp "$REPOSITORY_ROOT/scripts/ci/package-notices.py" scripts/ci/
	printf "third-party bytes\n" >THIRD_PARTY_LICENSES.md
	printf 'license bytes\n' >LICENSE
	printf 'usage bytes\n' >README.md
	printf 'public header\n' >crates/xberg-ffi/include/xberg.h
	unset TARGET GLIBC_FLOOR CARGO_STATUS ALEF_STATUS OMIT_ARCHIVE OMIT_METADATA DOCKER_STATUS EMPTY_DOCKER_OUTPUT
	export GO_TARGET=aarch64-apple-darwin GO_PLATFORM=macos-arm64 GO_SHARED=libxberg_ffi.dylib GO_STATIC=libxberg_ffi.a
	xberg_stub cargo \
		'printf "cargo %s\n" "$*" >>"$XBERG_TRACE"' \
		'[ "${CARGO_STATUS:-0}" = 0 ] || exit "$CARGO_STATUS"' \
		'mkdir -p "target/$GO_TARGET/release"' \
		'printf "floor shared\\n" >"target/$GO_TARGET/release/$GO_SHARED"' \
		'printf "floor static\\n" >"target/$GO_TARGET/release/$GO_STATIC"'
	xberg_stub alef \
		'printf "alef %s\n" "$*" >>"$XBERG_TRACE"' \
		'[ "${ALEF_STATUS:-0}" = 0 ] || exit "$ALEF_STATUS"' \
		'if [ "$2" = build ]; then mkdir -p "target/$GO_TARGET/release"; printf "generic shared\\n" >"target/$GO_TARGET/release/$GO_SHARED"; printf "generic static\\n" >"target/$GO_TARGET/release/$GO_STATIC"; exit 0; fi' \
		'[ "${OMIT_ARCHIVE:-false}" != true ] || exit 0' \
		'mkdir -p "alef-stage/$GO_PLATFORM/lib"' \
		'cp "target/$GO_TARGET/release/$GO_SHARED" "target/$GO_TARGET/release/$GO_STATIC" "alef-stage/$GO_PLATFORM/lib/"' \
		'if [ "${OMIT_METADATA:-false}" != true ]; then printf -- "-lpthread\\n" >"alef-stage/$GO_PLATFORM/lib/native-static-libs.txt"; fi' \
		'tar -czf "output/xberg-go-v1.4.0-$GO_PLATFORM.tar.gz" -C alef-stage "$GO_PLATFORM"'
	xberg_stub git 'printf "revision123\\n"'
	xberg_stub docker \
		'printf "docker %s\n" "$*" >>"$XBERG_TRACE"' \
		'[ "${DOCKER_STATUS:-0}" = 0 ] || exit "$DOCKER_STATUS"' \
		'for argument in "$@"; do case "$argument" in type=local,dest=*) destination="${argument#type=local,dest=}" ;; esac; done' \
		'[ "${EMPTY_DOCKER_OUTPUT:-false}" != true ] || exit 0' \
		'printf "musl shared\\n" >"$destination/libxberg_ffi.so"' \
		'printf "musl dependency\\n" >"$destination/libonnxruntime.so.1"'
}

@test "CLI archive packaging should preserve the explicit target binary and both optional PDFium runtimes" {
	export TARGET=wrong-target
	mkdir -p target/selected-target/release
	printf 'selected CLI\n' >target/selected-target/release/xberg
	printf 'mac PDFium\n' >target/selected-target/release/libpdfium.dylib
	printf 'Linux PDFium\n' >target/selected-target/release/libpdfium.so
	run "$BASH" "$XBERG_WORK/scripts/package_cli_artifact.sh" selected-target
	xberg_assert_status 0
	mkdir extracted
	tar -xzf xberg-cli-selected-target.tar.gz -C extracted
	xberg_assert_file extracted/xberg-cli-selected-target/xberg 'selected CLI'
	xberg_assert_file extracted/xberg-cli-selected-target/LICENSE 'license bytes'
	cmp THIRD_PARTY_LICENSES.md extracted/xberg-cli-selected-target/THIRD_PARTY_LICENSES.md
	xberg_assert_file extracted/xberg-cli-selected-target/README.md 'usage bytes'
	xberg_assert_file extracted/xberg-cli-selected-target/libpdfium.dylib 'mac PDFium'
	xberg_assert_file extracted/xberg-cli-selected-target/libpdfium.so 'Linux PDFium'
	[ "$(find extracted -type f | wc -l | tr -d '[:space:]')" -eq 6 ]
	[ ! -e xberg-cli-selected-target ]
}

@test "CLI archive packaging should accept its environment target without requiring optional runtimes" {
	export TARGET=environment-target
	mkdir -p "target/$TARGET/release"
	printf 'environment CLI\n' >"target/$TARGET/release/xberg"
	run "$BASH" "$XBERG_WORK/scripts/package_cli_artifact.sh"
	xberg_assert_status 0
	[ "$(tar -tzf xberg-cli-environment-target.tar.gz | grep -Ec '/[^/]+$')" -eq 4 ]
}

@test "CLI archive packaging should reject a missing target before creating staging directories" {
	run "$BASH" "$XBERG_WORK/scripts/package_cli_artifact.sh"
	xberg_assert_status 1
	xberg_assert_output_contains 'Usage:'
	[ -z "$(find . -maxdepth 1 -name 'xberg-cli-*' -print -quit)" ]
}

@test "CLI archive packaging should reject missing compiled binaries without emitting an archive" {
	run "$BASH" "$XBERG_WORK/scripts/package_cli_artifact.sh" missing-target
	xberg_assert_status 1
	[ ! -e xberg-cli-missing-target.tar.gz ]
}

@test "Go release packaging should produce a complete macOS archive after exactly one build and package call" {
	run "$BASH" "$REPOSITORY_ROOT/scripts/ci/package-go-release.sh" "$GO_TARGET" 1.4.0 "$XBERG_WORK/output"
	xberg_assert_status 0
	xberg_assert_trace "alef publish build --lang go --target $GO_TARGET
alef publish package --lang go --target $GO_TARGET --version 1.4.0 --output $XBERG_WORK/output"
	mkdir extracted
	tar -xzf "output/xberg-go-v1.4.0-$GO_PLATFORM.tar.gz" -C extracted
	xberg_assert_file "extracted/$GO_PLATFORM/lib/$GO_SHARED" 'generic shared'
	xberg_assert_file "extracted/$GO_PLATFORM/lib/$GO_STATIC" 'generic static'
	xberg_assert_file "extracted/$GO_PLATFORM/lib/native-static-libs.txt" '-lpthread'
	[ "$(find extracted -type f | wc -l | tr -d '[:space:]')" -eq 3 ]
}

@test "Go release packaging should restore both glibc-floor libraries after the generic build overwrites them" {
	export GO_TARGET=x86_64-unknown-linux-gnu GO_PLATFORM=linux-x86_64 GO_SHARED=libxberg_ffi.so GLIBC_FLOOR=2.28
	run "$BASH" "$REPOSITORY_ROOT/scripts/ci/package-go-release.sh" "$GO_TARGET" 1.4.0 "$XBERG_WORK/output"
	xberg_assert_status 0
	xberg_assert_trace "cargo zigbuild --locked -p xberg-ffi --release --target $GO_TARGET.2.28
alef publish build --lang go --target $GO_TARGET
alef publish package --lang go --target $GO_TARGET --version 1.4.0 --output $XBERG_WORK/output"
	mkdir extracted
	tar -xzf "output/xberg-go-v1.4.0-$GO_PLATFORM.tar.gz" -C extracted
	xberg_assert_file "extracted/$GO_PLATFORM/lib/$GO_SHARED" 'floor shared'
	xberg_assert_file "extracted/$GO_PLATFORM/lib/$GO_STATIC" 'floor static'
}

@test "Go release packaging should reject unsupported targets before invoking build tools" {
	run "$BASH" "$REPOSITORY_ROOT/scripts/ci/package-go-release.sh" unknown-target 1.4.0 "$XBERG_WORK/output"
	xberg_assert_status 2
	xberg_assert_output_contains 'unsupported Go release target'
	xberg_assert_trace_empty
}

@test "Go release packaging should preserve a failed Alef build before invoking packaging" {
	export ALEF_STATUS=9
	run "$BASH" "$REPOSITORY_ROOT/scripts/ci/package-go-release.sh" "$GO_TARGET" 1.4.0 "$XBERG_WORK/output"
	xberg_assert_status 9
	xberg_assert_trace "alef publish build --lang go --target $GO_TARGET"
	[ -z "$(find output -type f -print -quit)" ]
}

@test "Go release packaging should reject a success-looking package command that emitted no archive" {
	export OMIT_ARCHIVE=true
	run "$BASH" "$REPOSITORY_ROOT/scripts/ci/package-go-release.sh" "$GO_TARGET" 1.4.0 "$XBERG_WORK/output"
	xberg_assert_status 1
	xberg_assert_output_contains 'Go release archive not found'
}

@test "Go release packaging should reject a real archive missing static link metadata" {
	export OMIT_METADATA=true
	run "$BASH" "$REPOSITORY_ROOT/scripts/ci/package-go-release.sh" "$GO_TARGET" 1.4.0 "$XBERG_WORK/output"
	xberg_assert_status 1
	[ "$(tar -tzf "output/xberg-go-v1.4.0-$GO_PLATFORM.tar.gz" | grep -Ec '/lib/[^/]+$')" -eq 2 ]
}

@test "Go musl packaging should include actual Docker output libraries and the public header exactly once" {
	run "$BASH" "$REPOSITORY_ROOT/scripts/ci/package-go-musl-release.sh" x86_64-unknown-linux-musl linux-x86_64-musl 1.4.0 "$XBERG_WORK/output"
	xberg_assert_status 0
	[ "$(wc -l <"$XBERG_TRACE" | tr -d '[:space:]')" -eq 1 ]
	grep -Fq 'docker build -f docker/Dockerfile.musl-ffi --build-arg XBERG_BUILD_ID=revision123 --output type=local,dest=' "$XBERG_TRACE"
	mkdir extracted
	tar -xzf output/xberg-go-v1.4.0-linux-x86_64-musl.tar.gz -C extracted
	xberg_assert_file extracted/xberg-go-v1.4.0-linux-x86_64-musl/lib/libxberg_ffi.so 'musl shared'
	xberg_assert_file extracted/xberg-go-v1.4.0-linux-x86_64-musl/lib/libonnxruntime.so.1 'musl dependency'
	xberg_assert_file extracted/xberg-go-v1.4.0-linux-x86_64-musl/include/xberg.h 'public header'
	cmp "$REPOSITORY_ROOT/LICENSE" extracted/xberg-go-v1.4.0-linux-x86_64-musl/LICENSE
	cmp "$REPOSITORY_ROOT/THIRD_PARTY_LICENSES.md" extracted/xberg-go-v1.4.0-linux-x86_64-musl/THIRD_PARTY_LICENSES.md
	[ "$(find extracted -type f | wc -l | tr -d '[:space:]')" -eq 5 ]
	[ ! -e output/xberg-go-v1.4.0-linux-x86_64-musl ]
}

@test "Go musl packaging should reject mismatched target and platform before starting Docker" {
	run "$BASH" "$REPOSITORY_ROOT/scripts/ci/package-go-musl-release.sh" aarch64-unknown-linux-musl linux-x86_64-musl 1.4.0 "$XBERG_WORK/output"
	xberg_assert_status 2
	xberg_assert_output_contains 'unsupported Go musl release target/platform'
	xberg_assert_trace_empty
}

@test "Go musl packaging should propagate Docker build failure without creating an archive" {
	export DOCKER_STATUS=7
	run "$BASH" "$REPOSITORY_ROOT/scripts/ci/package-go-musl-release.sh" x86_64-unknown-linux-musl linux-x86_64-musl 1.4.0 "$XBERG_WORK/output"
	xberg_assert_status 7
	[ -z "$(find output -type f -print -quit)" ]
}

@test "Go musl packaging should reject successful Docker output containing zero shared libraries" {
	export EMPTY_DOCKER_OUTPUT=true
	run "$BASH" "$REPOSITORY_ROOT/scripts/ci/package-go-musl-release.sh" x86_64-unknown-linux-musl linux-x86_64-musl 1.4.0 "$XBERG_WORK/output"
	xberg_assert_status 1
	[ -z "$(find output -type f -print -quit)" ]
}

@test "Go installation README generation should use the requested output and keep shell variables literal" {
	run "$BASH" "$REPOSITORY_ROOT/scripts/ci/go/generate-install-readme.sh" "$XBERG_WORK/INSTALL.md"
	xberg_assert_status 0
	grep -Fxq 'export PKG_CONFIG_PATH="$HOME/.local/share/pkgconfig:$PKG_CONFIG_PATH"' INSTALL.md
	grep -Fxq 'go get github.com/xberg-io/xberg/packages/go@latest' INSTALL.md
	[ "$(find . -maxdepth 1 -name INSTALL.md | wc -l | tr -d '[:space:]')" -eq 1 ]
}

@test "Go installation README generation should fail when its output directory is absent" {
	run "$BASH" "$REPOSITORY_ROOT/scripts/ci/go/generate-install-readme.sh" "$XBERG_WORK/absent/INSTALL.md"
	xberg_assert_status 1
	[[ "$output" != *'README generated'* ]]
}
