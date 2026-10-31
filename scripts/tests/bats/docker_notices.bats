#!/usr/bin/env bats

setup() {
	[ -z "${XBERG_EXPECTED_BASH_MAJOR_MINOR:-}" ] || [[ "$BASH_VERSION" == "$XBERG_EXPECTED_BASH_MAJOR_MINOR".* ]]
	bats_load_library xberg-bats
	xberg_setup
	REPOSITORY_ROOT="$(cd "$BATS_TEST_DIRNAME/../../.." && pwd -P)"
	mkdir -p "$XBERG_WORK/docker"
	cp "$REPOSITORY_ROOT"/docker/Dockerfile.* "$XBERG_WORK/docker/"
	cp "$REPOSITORY_ROOT/.dockerignore" "$REPOSITORY_ROOT/LICENSE" "$REPOSITORY_ROOT/THIRD_PARTY_LICENSES.md" "$XBERG_WORK/"
}

check_notice_contexts() {
	run python3 - "$XBERG_WORK" <<'PYTHON'
import fnmatch
import shlex
import sys
from pathlib import Path, PurePosixPath

root = Path(sys.argv[1])
notices = ("LICENSE", "THIRD_PARTY_LICENSES.md")
patterns = (root / ".dockerignore").read_text().splitlines()
for notice in notices:
    assert (root / notice).read_bytes(), f"empty source notice: {notice}"
    allowed = True
    for line in patterns:
        if line and not line.startswith("#") and fnmatch.fnmatchcase(notice, line.removeprefix("!").strip("/")):
            allowed = line.startswith("!")
    assert allowed, f".dockerignore excludes {notice}"

runtime_images = []
for dockerfile in sorted((root / "docker").glob("Dockerfile.*")):
    copies = []
    workdir = PurePosixPath("/")
    for line in dockerfile.read_text().splitlines():
        if not line.startswith(("FROM ", "WORKDIR ", "COPY ")):
            continue
        tokens = shlex.split(line, comments=True)
        if tokens[0] == "FROM":
            copies = []
            workdir = PurePosixPath("/")
        elif tokens[0] == "WORKDIR":
            workdir /= tokens[1]
        else:
            copies.append((tokens[1:-1], workdir / tokens[-1]))
    if not any(destination == PurePosixPath("/usr/local/bin/xberg") for _, destination in copies):
        continue
    runtime_images.append(dockerfile.name)
    for notice in notices:
        matching = [(sources, destination) for sources, destination in copies
                    if not any(source.startswith("--") for source in sources)
                    and notice in sources and destination == PurePosixPath("/usr/local/share/licenses/xberg")]
        assert len(matching) == 1, f"{dockerfile.name}: missing canonical runtime notice {notice}"
assert runtime_images == ["Dockerfile.cli", "Dockerfile.core", "Dockerfile.full"], runtime_images
print("Verified 3 final runtime images and 6 canonical notice copies")
PYTHON
}

@test "Runtime Docker contexts should include both canonical notices in all three final images" {
	check_notice_contexts
	xberg_assert_status 0
	xberg_assert_output_contains 'Verified 3 final runtime images and 6 canonical notice copies'
}

@test "Runtime notice check should reject an excluded canonical notice" {
	printf '\nTHIRD_PARTY_LICENSES.md\n' >>"$XBERG_WORK/.dockerignore"
	check_notice_contexts
	xberg_assert_status 1
	xberg_assert_output_contains '.dockerignore excludes THIRD_PARTY_LICENSES.md'
}

@test "Runtime notice check should reject notices copied only into a builder" {
	sed '/^COPY LICENSE THIRD_PARTY_LICENSES.md /d' "$XBERG_WORK/docker/Dockerfile.core" >"$XBERG_WORK/core.tmp"
	printf '\nCOPY LICENSE THIRD_PARTY_LICENSES.md /usr/local/share/licenses/xberg/\n' >>"$XBERG_WORK/core.tmp"
	# A subsequent stage removes the notices from the final image. ~keep
	printf '\nFROM scratch\nCOPY --from=builder /build/xberg /usr/local/bin/xberg\n' >>"$XBERG_WORK/core.tmp"
	mv "$XBERG_WORK/core.tmp" "$XBERG_WORK/docker/Dockerfile.core"
	check_notice_contexts
	xberg_assert_status 1
	xberg_assert_output_contains 'Dockerfile.core: missing canonical runtime notice LICENSE'
}

@test "Runtime notice check should reject an empty image inventory" {
	rm "$XBERG_WORK"/docker/Dockerfile.*
	check_notice_contexts
	xberg_assert_status 1
	xberg_assert_output_contains 'AssertionError: []'
}

@test "Runtime notice check should reject an empty source notice" {
	: >"$XBERG_WORK/THIRD_PARTY_LICENSES.md"
	check_notice_contexts
	xberg_assert_status 1
	xberg_assert_output_contains 'empty source notice: THIRD_PARTY_LICENSES.md'
}

load_runtime_notice_test() {
	eval "$(sed -n '/^test_runtime_notices() {$/,/^}$/p' "$REPOSITORY_ROOT/scripts/test/test-docker-config-local.sh")"
	export DOCKER_DIR="$XBERG_WORK/docker"
	export TEST_TEMP_DIR="$XBERG_WORK"
	start_test() { :; }
	get_image_name() { printf 'xberg:%s\n' "$1"; }
	pass_test() { printf 'notices match\n'; }
	fail_test() { printf '%s\n' "$1"; }
}

@test "Image notice smoke test should compare both complete canonical files" {
	load_runtime_notice_test
	# The fake image maps the requested absolute path to its canonical fixture. ~keep
	# shellcheck disable=SC2329 # Invoked by the loaded image smoke test.
	docker() {
		local path="${!#}"
		cat "$XBERG_WORK/${path##*/}"
	}
	run test_runtime_notices core
	xberg_assert_status 0
	xberg_assert_output_contains 'notices match'
}

@test "Image notice smoke test should reject different bytes after a matching license" {
	load_runtime_notice_test
	# shellcheck disable=SC2329 # Invoked by the loaded image smoke test.
	docker() {
		local path="${!#}"
		cat "$XBERG_WORK/${path##*/}"
		[[ "$path" != */THIRD_PARTY_LICENSES.md ]] || printf 'corruption\n'
		return 0
	}
	run test_runtime_notices full
	xberg_assert_status 1
	xberg_assert_output_contains 'Runtime notice differs from canonical source: THIRD_PARTY_LICENSES.md'
}

@test "Image notice smoke test should reject an unreadable notice" {
	load_runtime_notice_test
	docker() { return 1; }
	run test_runtime_notices core
	xberg_assert_status 1
	xberg_assert_output_contains 'Could not read runtime notice: LICENSE'
}
