#!/usr/bin/env bats

setup() {
	bats_load_library xberg-bats
	xberg_setup
	REPOSITORY_ROOT="$(cd "$BATS_TEST_DIRNAME/../../.." && pwd -P)"
	cd "$XBERG_WORK" || return
	xberg_stub apt-cache 'printf "apt-cache %s\n" "$*" >>"$XBERG_TRACE"' 'printf "%s\n" "${APT_POLICY:-  Candidate: 1:5.5.4-2ubuntu1}"; exit "${APT_STATUS:-0}"'
	xberg_stub brew 'printf "brew %s\n" "$*" >>"$XBERG_TRACE"' 'case "$*" in "info --json=v2 tesseract") printf "%s\n" "${BREW_JSON:-}";; "info tesseract") printf "%s\n" "${BREW_TEXT:-tesseract: stable 5.5.4 (bottled)}";; *) exit 97;; esac'
	unset APT_POLICY APT_STATUS BREW_JSON BREW_TEXT
}

@test "Linux detector should parse candidate version and omit epoch and distribution suffix" {
	export APT_POLICY
	APT_POLICY="$(printf '  Installed: 5.3.0-1\n  Candidate: 1:5.5.4-2ubuntu1\n')"
	run "$BASH" "$REPOSITORY_ROOT/scripts/ci/install-system-deps/detect-tesseract-linux.sh"
	xberg_assert_status 0
	xberg_assert_github_output version=5.5.4
	xberg_assert_trace 'apt-cache policy tesseract-ocr'
}

@test "Linux detector should emit unknown on unavailable apt metadata" {
	export APT_POLICY='  Candidate: (none)' APT_STATUS=7
	run "$BASH" "$REPOSITORY_ROOT/scripts/ci/install-system-deps/detect-tesseract-linux.sh"
	xberg_assert_status 0
	xberg_assert_github_output version=unknown
	xberg_assert_trace 'apt-cache policy tesseract-ocr'
}

@test "macOS detector should use JSON stable version without requesting text fallback" {
	export BREW_JSON='{"formulae":[{"versions":{"stable":"5.5.4_2"}}]}'
	run "$BASH" "$REPOSITORY_ROOT/scripts/ci/install-system-deps/detect-tesseract-macos.sh"
	xberg_assert_status 0
	xberg_assert_github_output version=5.5.4
	xberg_assert_trace 'brew info --json=v2 tesseract'
}

@test "macOS detector should parse text fallback after malformed JSON" {
	export BREW_JSON='not json' BREW_TEXT='tesseract: stable 5.4.1 (bottled)'
	run "$BASH" "$REPOSITORY_ROOT/scripts/ci/install-system-deps/detect-tesseract-macos.sh"
	xberg_assert_status 0
	xberg_assert_github_output version=5.4.1
	xberg_assert_trace 'brew info --json=v2 tesseract' 'brew info tesseract'
}

@test "macOS detector should emit unknown when both metadata forms lack a version" {
	export BREW_JSON='{"formulae":[]}' BREW_TEXT=unavailable
	run "$BASH" "$REPOSITORY_ROOT/scripts/ci/install-system-deps/detect-tesseract-macos.sh"
	xberg_assert_status 0
	xberg_assert_github_output version=unknown
	xberg_assert_trace 'brew info --json=v2 tesseract' 'brew info tesseract'
}

prepare_macos_install() {
	mkdir -p scripts/ci/install-system-deps scripts/lib
	cp "$REPOSITORY_ROOT/scripts/ci/install-system-deps/install-macos.sh" scripts/ci/install-system-deps/
	cp "$REPOSITORY_ROOT/scripts/lib/retry.sh" scripts/lib/
	printf 'printf "verify-ocr\\n" >>"$XBERG_TRACE"\nexit "${VERIFY_STATUS:-0}"\n' >scripts/ci/install-system-deps/verify-ocr-macos.sh
	cat >macos-runner.sh <<'SH'
set -eu
brew() {
 printf 'brew %s\n' "$*" >>"$XBERG_TRACE"
 case "$1" in
 list) [ "${ALREADY_INSTALLED:-false}" = true ] ;;
 install|upgrade) [ "$2" != "${FAIL_FORMULA:-none}" ] ;;
 update|link) return 0 ;;
 *) return 97 ;;
 esac
}
cmake() { printf 'cmake %s\n' "$*" >>"$XBERG_TRACE"; printf 'cmake version 3.30.0\n'; }
tesseract() { printf 'tesseract %s\n' "$*" >>"$XBERG_TRACE"; printf 'tesseract 5.5.4\n'; }
php() { printf 'php %s\n' "$*" >>"$XBERG_TRACE"; printf 'PHP 8.4.0\n'; }
pkg-config() { printf 'pkg-config %s\n' "$*" >>"$XBERG_TRACE"; printf '2.3.0\n'; }
sleep() { printf 'sleep %s\n' "$*" >>"$XBERG_TRACE"; }
export -f brew cmake tesseract php pkg-config sleep
export REPO_ROOT="$XBERG_WORK"
source scripts/ci/install-system-deps/install-macos.sh
SH
}

@test "macOS installer should install exact required formulas while preserving active PHP" {
	prepare_macos_install
	run "$BASH" macos-runner.sh
	xberg_assert_status 0
	grep '^brew install ' "$XBERG_TRACE" >installs
	xberg_assert_file installs "$(printf 'brew install %s\n' cmake tesseract tesseract-lang libmagic libheif boost pkg-config)"
	[ "$(grep -c '^brew update$' "$XBERG_TRACE")" -eq 1 ]
	[ "$(grep -c '^verify-ocr$' "$XBERG_TRACE")" -eq 1 ]
	[ "$(grep -c '^php --version$' "$XBERG_TRACE")" -eq 2 ]
	xberg_assert_github_env CMAKE=cmake
}

@test "macOS installer should upgrade all eight existing formulas without installing packages" {
	prepare_macos_install
	run env ALREADY_INSTALLED=true "$BASH" macos-runner.sh
	xberg_assert_status 0
	grep '^brew upgrade ' "$XBERG_TRACE" >upgrades
	xberg_assert_file upgrades "$(printf 'brew upgrade %s\n' cmake tesseract leptonica tesseract-lang libmagic libheif boost pkg-config)"
	run grep -q '^brew install ' "$XBERG_TRACE"
	xberg_assert_status 1
}

@test "macOS installer should exhaust CMake retries before later package installations" {
	prepare_macos_install
	run env FAIL_FORMULA=cmake "$BASH" macos-runner.sh
	xberg_assert_status 1
	[ "$(grep -c '^brew install cmake$' "$XBERG_TRACE")" -eq 3 ]
	[ "$(grep -c '^sleep ' "$XBERG_TRACE")" -eq 2 ]
	xberg_assert_output_contains 'Failed to install CMake after retries'
	run grep -q '^brew install tesseract$' "$XBERG_TRACE"
	xberg_assert_status 1
}

@test "macOS installer should propagate the nested OCR verification failure" {
	prepare_macos_install
	run env VERIFY_STATUS=7 "$BASH" macos-runner.sh
	xberg_assert_status 7
	[ "$(grep -c '^verify-ocr$' "$XBERG_TRACE")" -eq 1 ]
	run grep -q '^pkg-config --version$' "$XBERG_TRACE"
	xberg_assert_status 1
}
