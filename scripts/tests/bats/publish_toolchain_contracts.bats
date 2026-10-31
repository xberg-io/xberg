#!/usr/bin/env bats

setup() {
	bats_load_library xberg-bats
	xberg_setup
	REPOSITORY_ROOT="$(cd "$BATS_TEST_DIRNAME/../../.." && pwd -P)"
	cd "$XBERG_WORK"
	unset WASM_ALREADY_PUBLISHED WASM_VERSION
}

@test "Windows GNU toolchain setup should export every required compiler and static-runtime setting" {
	run bash "$REPOSITORY_ROOT/scripts/publish/go/configure-windows-gnu-toolchain.sh"
	xberg_assert_status 0
	xberg_assert_github_env 'CC=gcc
AR=ar
RANLIB=ranlib
CXX=g++
CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER=x86_64-w64-mingw32-gcc
RUSTFLAGS=-C target-feature=+crt-static
CARGO_BUILD_TARGET=x86_64-pc-windows-gnu'
}

@test "cross linker setup should prefer all installed target-specific tools" {
	xberg_stub_exit aarch64-linux-gnu-gcc
	xberg_stub_exit aarch64-linux-gnu-g++
	xberg_stub_exit aarch64-linux-gnu-ar
	run bash "$REPOSITORY_ROOT/scripts/publish/cli/configure-cross-linker.sh"
	xberg_assert_status 0
	xberg_assert_github_env 'CC_aarch64_unknown_linux_gnu=aarch64-linux-gnu-gcc
CXX_aarch64_unknown_linux_gnu=aarch64-linux-gnu-g++
AR_aarch64_unknown_linux_gnu=aarch64-linux-gnu-ar
CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER=aarch64-linux-gnu-gcc'
}

@test "Node dependency install should clear the Rust wrapper before invoking package tools" {
	xberg_stub corepack 'printf "corepack %s|%s\n" "$*" "${RUSTC_WRAPPER-unset}" >>"$XBERG_TRACE"'
	xberg_stub pnpm 'printf "pnpm %s|%s\n" "$*" "${RUSTC_WRAPPER-unset}" >>"$XBERG_TRACE"'
	run env RUSTC_WRAPPER=sccache bash "$REPOSITORY_ROOT/scripts/publish/node/install-deps.sh"
	xberg_assert_status 0
	xberg_assert_trace 'corepack enable|unset
pnpm install -C crates/xberg-node|unset'
}

@test "Node dependency install should stop after package-manager activation fails" {
	xberg_stub_exit corepack 9
	xberg_stub_trace pnpm
	run bash "$REPOSITORY_ROOT/scripts/publish/node/install-deps.sh"
	xberg_assert_status 9
	xberg_assert_trace_empty
}

@test "Ruby Bundler setup should request the pinned version without fallback on success" {
	xberg_stub_trace gem
	xberg_stub_trace bundler
	run bash "$REPOSITORY_ROOT/scripts/ci/ruby/install-bundler.sh"
	xberg_assert_status 0
	xberg_assert_trace 'gem install bundler -v 4.0.3 --no-document
bundler --version'
}

@test "Ruby Bundler setup should fall back only after the pinned install fails" {
	xberg_stub gem \
		'printf "gem %s\n" "$*" >>"$XBERG_TRACE"' \
		'case "$*" in *"-v 4.0.3"*) exit 7 ;; esac'
	xberg_stub_trace bundler
	run bash "$REPOSITORY_ROOT/scripts/ci/ruby/install-bundler.sh"
	xberg_assert_status 0
	xberg_assert_trace 'gem install bundler -v 4.0.3 --no-document
gem install bundler --no-document
bundler --version'
}

@test "WASM summary should distinguish a completed publication from an existing version" {
	run env WASM_VERSION=1.4.0 bash "$REPOSITORY_ROOT/scripts/publish/wasm/write-summary.sh"
	xberg_assert_status 0
	xberg_assert_github_summary 'Successfully published WASM package @xberg-io/xberg-wasm@1.4.0 to npm
Uploaded WASM artifacts to GitHub release'
	: >"$GITHUB_STEP_SUMMARY"
	run env WASM_VERSION=1.4.0 WASM_ALREADY_PUBLISHED=true \
		bash "$REPOSITORY_ROOT/scripts/publish/wasm/write-summary.sh"
	xberg_assert_status 0
	xberg_assert_github_summary 'WASM package @xberg-io/xberg-wasm@1.4.0 was already published; skipped republish.'
}

@test "WASM summary should remain optional outside GitHub Actions" {
	run env -u GITHUB_STEP_SUMMARY bash "$REPOSITORY_ROOT/scripts/publish/wasm/write-summary.sh"
	xberg_assert_status 0
	xberg_assert_output ''
	xberg_assert_github_summary ''
}
