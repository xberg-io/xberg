#!/usr/bin/env bats
# Negative controls for scripts/ci/check-licenses.sh.
#
# Every graph is a scratch manifest created inside the repository: cargo-deny resolves deny.toml
# by walking up from the manifest, so a manifest under /tmp would silently fall back to the
# default policy and pass for the wrong reason. The graph table and the config path are injected
# through the checker's environment hooks so nothing tracked is edited while these run.

setup() {
	bats_load_library xberg-bats
	xberg_setup
	REPOSITORY_ROOT="$(cd "$BATS_TEST_DIRNAME/../../.." && pwd -P)"
	SCRATCH="$(mktemp -d "$REPOSITORY_ROOT/.bats-license-control.XXXXXX")"
	export XBERG_NO_LICENSE_FIELD_OK=""
}

teardown() {
	rm -rf "$SCRATCH"
}

make_package() {
	local name=$1 license=$2
	mkdir -p "$SCRATCH/$name/src"
	{
		printf '[package]\n'
		printf 'name = "%s"\n' "$name"
		printf 'version = "0.0.0"\n'
		printf 'edition = "2021"\n'
		printf 'license = "%s"\n' "$license"
	} >"$SCRATCH/$name/Cargo.toml"
	printf 'pub fn control() {}\n' >"$SCRATCH/$name/src/lib.rs"
}

make_root() {
	local dir=$1 dependency=$2
	mkdir -p "$dir/src"
	{
		printf '[workspace]\n\n'
		printf '[package]\n'
		printf 'name = "license-control"\n'
		printf 'version = "0.0.0"\n'
		printf 'edition = "2021"\n'
		printf 'license = "MIT"\n'
		printf 'publish = false\n\n'
		printf '[dependencies]\n'
		printf '%s = { path = "../%s" }\n' "$dependency" "$dependency"
	} >"$dir/Cargo.toml"
	printf 'pub fn control() {}\n' >"$dir/src/lib.rs"
	cargo generate-lockfile --manifest-path "$dir/Cargo.toml" --offline
}

@test "license gate should reject a dependency whose license is not allowed" {
	make_package gpl-only GPL-3.0-only
	make_root "$SCRATCH/gpl" gpl-only
	export XBERG_LICENSE_GRAPHS="gpl control|$SCRATCH/gpl/Cargo.toml||"

	run bash "$REPOSITORY_ROOT/scripts/ci/check-licenses.sh"

	xberg_assert_status 1
	xberg_assert_output_contains "FAIL gpl control"
	xberg_assert_output_contains "rejected: license is not explicitly allowed"
	xberg_assert_output_contains "GPL-3.0-only"
}

@test "license gate should reject an exception that matches nothing in any graph" {
	make_package mit-only MIT
	make_root "$SCRATCH/mit" mit-only
	cat >"$SCRATCH/deny.toml" <<'EOF'
[licenses]
allow = ["MIT"]
exceptions = [{ allow = ["MPL-2.0"], crate = "not-a-real-crate" }]
EOF
	export XBERG_DENY_CONFIG="$SCRATCH/deny.toml"
	export XBERG_LICENSE_GRAPHS="mit control|$SCRATCH/mit/Cargo.toml||"

	run bash "$REPOSITORY_ROOT/scripts/ci/check-licenses.sh"

	xberg_assert_status 1
	xberg_assert_output_contains "license exceptions match nothing in any checked graph"
	xberg_assert_output_contains "not-a-real-crate"
}

@test "license gate should reject a local exceptions file that cargo-deny would merge" {
	make_package gpl-only GPL-3.0-only
	make_root "$SCRATCH/gpl" gpl-only
	printf '[licenses]\nallow = ["MIT"]\n' >"$SCRATCH/deny.toml"
	printf 'exceptions = [{ allow = ["GPL-3.0-only"], crate = "gpl-only" }]\n' \
		>"$SCRATCH/gpl/deny.exceptions.toml"

	# The claim the guard exists for: passing --config does not stop cargo-deny from merging the
	# project-local exceptions file, and the merged entry turns the rejected GPL dependency into
	# a pass. If this assertion ever fails, cargo-deny stopped discovering these files.
	run cargo deny --config "$SCRATCH/deny.toml" --manifest-path "$SCRATCH/gpl/Cargo.toml" --locked check licenses
	xberg_assert_status 0

	export XBERG_DENY_CONFIG="$SCRATCH/deny.toml"
	export XBERG_LICENSE_GRAPHS="gpl control|$SCRATCH/gpl/Cargo.toml||"

	run bash "$REPOSITORY_ROOT/scripts/ci/check-licenses.sh"

	xberg_assert_status 1
	xberg_assert_output_contains "gpl control: cargo-deny would load local license exceptions"
	xberg_assert_output_contains "deny.exceptions.toml"
}

@test "license gate should pass a clean graph that declares no exceptions" {
	make_package mit-only MIT
	make_root "$SCRATCH/mit" mit-only
	cat >"$SCRATCH/deny.toml" <<'EOF'
[licenses]
allow = ["MIT"]
EOF
	export XBERG_DENY_CONFIG="$SCRATCH/deny.toml"
	export XBERG_LICENSE_GRAPHS="mit control|$SCRATCH/mit/Cargo.toml||"

	run bash "$REPOSITORY_ROOT/scripts/ci/check-licenses.sh"

	xberg_assert_status 0
	xberg_assert_output_contains "no license exceptions declared"
}
