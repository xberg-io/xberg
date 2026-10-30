#!/usr/bin/env bats

setup() {
	bats_load_library xberg-bats
	xberg_setup
	REPOSITORY_ROOT="$(cd "$BATS_TEST_DIRNAME/../../.." && pwd -P)"
	FIXTURE_ROOT="$XBERG_WORK/repository"
	mkdir -p "$FIXTURE_ROOT/scripts/ci" "$FIXTURE_ROOT/crates/xberg" \
		"$FIXTURE_ROOT/packages/ruby/ext/xberg_rb/native" "$FIXTURE_ROOT/.github/workflows"
	cp "$REPOSITORY_ROOT/scripts/ci/check-ruby-windows-features.sh" "$FIXTURE_ROOT/scripts/ci/"
	cat >"$FIXTURE_ROOT/crates/xberg/Cargo.toml" <<'TOML'
windows-gnu-target = [
    "pdf",
    "ocr",
    "internal-only",
]
TOML
	cat >"$FIXTURE_ROOT/packages/ruby/ext/xberg_rb/native/Cargo.toml" <<'TOML'
[features]
pdf = []
ocr = []
other = []
[dependencies]
TOML
	printf 'ruby_cargo_args: --features pdf,ocr\n' >"$FIXTURE_ROOT/.github/workflows/publish.yaml"
}

@test "Ruby Windows guard should check the intersection of core and binding features" {
	run bash "$FIXTURE_ROOT/scripts/ci/check-ruby-windows-features.sh"
	xberg_assert_status 0
	xberg_assert_output_contains 'bundle=3'
	xberg_assert_output_contains 'binding=3'
	xberg_assert_output_contains 'selected=2'
	xberg_assert_output_contains 'every bundled core feature with a binding counterpart is selected'
}

@test "Ruby Windows guard should reject a missing generated binding counterpart" {
	printf 'ruby_cargo_args: --features pdf\n' >"$FIXTURE_ROOT/.github/workflows/publish.yaml"
	run bash "$FIXTURE_ROOT/scripts/ci/check-ruby-windows-features.sh"
	xberg_assert_status 1
	xberg_assert_output_contains '  - ocr'
	xberg_assert_output_contains 'NOT'
}

@test "Ruby Windows guard should reject empty parsed sets instead of comparing no features" {
	printf '[features]\n' >"$FIXTURE_ROOT/crates/xberg/Cargo.toml"
	run bash "$FIXTURE_ROOT/scripts/ci/check-ruby-windows-features.sh"
	[ "$status" -ne 0 ]
	[[ "$output" != *'ruby windows feature check: ok'* ]]
}

@test "Ruby Windows guard should reject missing manifests before parsing" {
	rm "$FIXTURE_ROOT/packages/ruby/ext/xberg_rb/native/Cargo.toml"
	run bash "$FIXTURE_ROOT/scripts/ci/check-ruby-windows-features.sh"
	xberg_assert_status 1
	xberg_assert_output_contains "error: missing $FIXTURE_ROOT/packages/ruby/ext/xberg_rb/native/Cargo.toml"
}
