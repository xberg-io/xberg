setup() {
	repo="$BATS_TEST_DIRNAME/../../.."
	bootstrap="$repo/docker/install-rustup-musl.sh"
}

@test "the four musl language builders use one bootstrap with the existing musl Rustup pins" {
	python3 - "$repo" <<'PY'
from pathlib import Path
import re
import sys

docker = Path(sys.argv[1]) / "docker"
source = (docker / "install-rustup-musl.sh").read_text()
pins = dict(re.findall(r"^[ \t]*(x86_64|aarch64)\) digest=([0-9a-f]{64}) ;{2}$", source, re.M))
assert "version=1.29.3" in source and len(pins) == 2
for previous in ("Dockerfile.binstall-musl", "Dockerfile.cli", "Dockerfile.musl-build"):
    reference = (docker / previous).read_text()
    assert "RUSTUP_VERSION=1.29.3" in reference
    old_pins = dict(re.findall(r"^        (x86_64|aarch64)\) RUSTUP_SHA256=([0-9a-f]{64}) ;{2}", reference, re.M))
    assert old_pins == pins, previous

copy = "COPY docker/install-rustup-musl.sh /tmp/install-rustup-musl.sh"
install = 'RUN sh /tmp/install-rustup-musl.sh "${RUST_TOOLCHAIN}"'
for builder in ("node", "ffi", "rustler", "python"):
    text = (docker / f"Dockerfile.musl-{builder}").read_text()
    assert all(text.count(step) == 1 for step in (copy, install)), builder
    assert text.index(copy) < text.index(install), builder
    assert "sh.rustup.rs" not in text, builder
assert "!docker/" in (docker.parent / ".dockerignore").read_text().splitlines()
PY
}

make_inert_bootstrap() {
	local fixture_sha
	mkdir "$BATS_TEST_TMPDIR/bin"
	cat >"$BATS_TEST_TMPDIR/fixture" <<'FIXTURE'
#!/bin/sh
printf '%s\n' "$@" > "$XBERG_FIXTURE_DIR/executed"
FIXTURE
	cat >"$BATS_TEST_TMPDIR/bin/uname" <<'UNAME'
#!/bin/sh
[ "$1" = -m ] || exit 2
printf '%s\n' "$XBERG_FIXTURE_ARCH"
UNAME
	cat >"$BATS_TEST_TMPDIR/bin/curl" <<'CURL'
#!/bin/sh
expected="https://static.rust-lang.org/rustup/archive/1.29.3/$XBERG_FIXTURE_ARCH-unknown-linux-musl/rustup-init"
if [ "$#" -ne 9 ] || [ "$1" != --proto ] || [ "$2" != '=https' ] ||
    [ "$3" != --proto-redir ] || [ "$4" != '=https' ] || [ "$5" != --tlsv1.2 ] ||
    [ "$6" != -fsSL ] || [ "$7" != "$expected" ] || [ "$8" != -o ] || [ "$9" != rustup-init ]; then
    printf 'Unexpected curl arguments:\n' >&2
    printf '  <%s>\n' "$@" >&2
    exit 2
fi
printf '%s\n' "$7" > "$XBERG_FIXTURE_DIR/downloaded"
printf '%s\n' "$PWD" > "$XBERG_FIXTURE_DIR/scratch"
cp "$XBERG_FIXTURE_DIR/fixture" rustup-init || exit 2
if [ "$XBERG_FIXTURE_TAMPERED" = yes ]; then
    printf '# changed bytes\n' >> rustup-init
fi
CURL
	chmod +x "$BATS_TEST_TMPDIR/bin/uname" "$BATS_TEST_TMPDIR/bin/curl"
	fixture_sha=$(sha256sum "$BATS_TEST_TMPDIR/fixture")
	sed -E "s/(digest=)[0-9a-f]{64}/\\1${fixture_sha%% *}/g" \
		"$bootstrap" >"$BATS_TEST_TMPDIR/bootstrap"
}

run_inert_bootstrap() {
	PATH="$BATS_TEST_TMPDIR/bin:$PATH" XBERG_FIXTURE_DIR="$BATS_TEST_TMPDIR" \
		XBERG_FIXTURE_ARCH="$1" XBERG_FIXTURE_TAMPERED="$2" \
		sh "$BATS_TEST_TMPDIR/$3" fixture-toolchain
}

@test "musl language builders execute verified bytes on x86_64 and aarch64 and clean up" {
	make_inert_bootstrap
	for arch in x86_64 aarch64; do
		run run_inert_bootstrap "$arch" no bootstrap
		[ "$status" -eq 0 ]
		[ "$(cat "$BATS_TEST_TMPDIR/executed")" = "$(printf '%s\n' -y --default-toolchain fixture-toolchain --component rust-src)" ]
		[ ! -e "$(cat "$BATS_TEST_TMPDIR/scratch")" ]
		rm "$BATS_TEST_TMPDIR/executed"
	done
}

@test "musl language builders reject mismatched bytes before execution on either architecture" {
	make_inert_bootstrap
	for arch in x86_64 aarch64; do
		run run_inert_bootstrap "$arch" yes bootstrap
		[ "$status" -ne 0 ]
		[[ "$output" == *'rustup-init: FAILED'* ]]
		[ ! -e "$BATS_TEST_TMPDIR/executed" ]
		[ ! -e "$(cat "$BATS_TEST_TMPDIR/scratch")" ]
	done
	sed '/^sha256sum -c rustup-init.sha256$/d' "$BATS_TEST_TMPDIR/bootstrap" >"$BATS_TEST_TMPDIR/unverified"
	run run_inert_bootstrap x86_64 yes unverified
	[ "$status" -eq 0 ]
	[ -f "$BATS_TEST_TMPDIR/executed" ]
}

@test "musl language builders reject unsupported architecture before downloading" {
	make_inert_bootstrap
	run run_inert_bootstrap riscv64 no bootstrap
	[ "$status" -ne 0 ]
	[[ "$output" == *'Unsupported Rustup architecture: riscv64'* ]]
	[ ! -e "$BATS_TEST_TMPDIR/downloaded" ]
	[ ! -e "$BATS_TEST_TMPDIR/executed" ]
}
