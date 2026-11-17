setup() {
	DOCKERFILE="$BATS_TEST_DIRNAME/../../../docker/Dockerfile.binstall-musl"
}

checked_bootstrap() {
	python3 - "$DOCKERFILE" <<'PYTHON'
from pathlib import Path
import re
import sys

text = Path(sys.argv[1]).read_text()
logical_lines = re.sub(r"\\\n[ \t]*", "", text).splitlines()
runs = [line.removeprefix("RUN ") for line in logical_lines if line.startswith("RUN RUSTUP_VERSION=")]
assert len(runs) == 1 and text.count("https://static.rust-lang.org/rustup/archive/") == 1
assert "sh.rustup.rs" not in text
steps = runs[0].split(" && ")
assert steps[:2] == ["RUSTUP_VERSION=1.29.3", "ARCH=$(uname -m)"]
case = (
    'case "$ARCH" in '
    "x86_64) RUSTUP_SHA256=1552169aba2836777136ccff7a381e95c17233b5d8250cd5e10ae3cb971bfcfc ;; "
    "aarch64) RUSTUP_SHA256=956deacdc27ab82ae7ac821d5d3e9e95e4795d476d17efe99743bb8531ca1031 ;; "
    r"""*) printf 'Unsupported Rustup architecture: %s\n' "$ARCH" >&2; exit 1 ;; esac"""
)
assert steps[2:-1] == [
    case,
    """curl --proto '=https' --proto-redir '=https' --tlsv1.2 -fsSL """
    '"https://static.rust-lang.org/rustup/archive/${RUSTUP_VERSION}/${ARCH}-unknown-linux-musl/rustup-init" -o rustup-init',
    r"""printf '%s  rustup-init\n' "$RUSTUP_SHA256" | sha256sum -c -""",
    "chmod 0755 rustup-init",
    './rustup-init -y --default-toolchain "${RUST_TOOLCHAIN}" --component rust-src',
    "rm rustup-init",
]
assert steps[-1] == 'echo "Rust host: $(~/.cargo/bin/rustc -vV | grep host)"'
assert 'RUN rustup target add "${TARGET}"' in logical_lines
assert 'SHELL ["/bin/ash", "-o", "pipefail", "-c"]' in logical_lines
print(" &&\n".join(steps[:-1]))
PYTHON
}

@test "binstall musl uses two pinned host installers and verifies before execution" {
	run checked_bootstrap
	[ "$status" -eq 0 ]
	[ "$(printf '%s\n' "$output" | wc -l)" -eq 8 ]
}

make_inert_bootstrap() {
	local fixture_sha
	checked_bootstrap > "$BATS_TEST_TMPDIR/bootstrap"
	mkdir "$BATS_TEST_TMPDIR/bin"
	cat > "$BATS_TEST_TMPDIR/fixture" <<'FIXTURE'
#!/bin/sh
printf '%s\n' "$@" > "$XBERG_FIXTURE_DIR/executed"
FIXTURE
	cat > "$BATS_TEST_TMPDIR/bin/uname" <<'UNAME'
#!/bin/sh
test "$1" = -m || exit 2
printf '%s\n' "$XBERG_FIXTURE_ARCH"
UNAME
	cat > "$BATS_TEST_TMPDIR/bin/curl" <<'CURL'
#!/bin/sh
test "$#" = 10 || exit 2
test "$7" = -fsSL || exit 2
test "$8" = "https://static.rust-lang.org/rustup/archive/1.29.3/${XBERG_FIXTURE_ARCH}-unknown-linux-musl/rustup-init" || exit 2
test "$9" = -o && test "${10}" = rustup-init || exit 2
printf '%s\n' "$8" > "$XBERG_FIXTURE_DIR/downloaded"
cp "$XBERG_FIXTURE_DIR/fixture" rustup-init || exit 2
if test "$XBERG_FIXTURE_TAMPERED" = yes; then
	printf '# modified bytes\n' >> rustup-init
fi
CURL
	chmod +x "$BATS_TEST_TMPDIR/bin/uname" "$BATS_TEST_TMPDIR/bin/curl"
	fixture_sha="$(sha256sum "$BATS_TEST_TMPDIR/fixture")"
	sed -E "s/(RUSTUP_SHA256=)[0-9a-f]{64}/\\1${fixture_sha%% *}/g" \
		"$BATS_TEST_TMPDIR/bootstrap" > "$BATS_TEST_TMPDIR/bootstrap-inert"
}

run_inert_bootstrap() (
	cd "$BATS_TEST_TMPDIR" || exit
	PATH="$BATS_TEST_TMPDIR/bin:$PATH" XBERG_FIXTURE_DIR="$BATS_TEST_TMPDIR" \
		XBERG_FIXTURE_ARCH="$1" XBERG_FIXTURE_TAMPERED="$2" RUST_TOOLCHAIN=fixture-toolchain \
		bash -o pipefail "$3"
)

@test "binstall musl executes verified bytes and stops tampered bytes before exec" {
	make_inert_bootstrap
	for arch in x86_64 aarch64; do
		run run_inert_bootstrap "$arch" no bootstrap-inert
		[ "$status" -eq 0 ]
		[ "$(cat "$BATS_TEST_TMPDIR/executed")" = "$(printf '%s\n' -y --default-toolchain fixture-toolchain --component rust-src)" ]
		[ ! -e "$BATS_TEST_TMPDIR/rustup-init" ]
		rm "$BATS_TEST_TMPDIR/executed"
	done

	run run_inert_bootstrap aarch64 yes bootstrap-inert
	[ "$status" -ne 0 ]
	[[ "$output" == *'rustup-init: FAILED'* ]]
	[ ! -e "$BATS_TEST_TMPDIR/executed" ]
	[ ! -x "$BATS_TEST_TMPDIR/rustup-init" ]

	sed '/sha256sum -c - &&$/d' "$BATS_TEST_TMPDIR/bootstrap-inert" > "$BATS_TEST_TMPDIR/bootstrap-unverified"
	run run_inert_bootstrap aarch64 yes bootstrap-unverified
	[ "$status" -eq 0 ]
	[ -f "$BATS_TEST_TMPDIR/executed" ]
}

@test "binstall musl rejects an unsupported host before downloading" {
	make_inert_bootstrap
	run run_inert_bootstrap riscv64 no bootstrap-inert
	[ "$status" -ne 0 ]
	[[ "$output" == *'Unsupported Rustup architecture: riscv64'* ]]
	[ ! -e "$BATS_TEST_TMPDIR/downloaded" ]
	[ ! -e "$BATS_TEST_TMPDIR/executed" ]
}
