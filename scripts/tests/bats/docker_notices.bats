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
print("Three final runtime images declare both canonical notices")
PYTHON
}

@test "Runtime Docker contexts should include both canonical notices in all three final images" {
	check_notice_contexts
	xberg_assert_status 0
	xberg_assert_output_contains 'Three final runtime images declare both canonical notices'
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

setup_image_verifier() {
	mkdir -p "$XBERG_WORK/bin" "$XBERG_WORK/image"
	cp "$REPOSITORY_ROOT/LICENSE" "$REPOSITORY_ROOT/THIRD_PARTY_LICENSES.md" "$XBERG_WORK/image/"
	# Serve the requested absolute path from the fake image, and fail on a path it lacks. ~keep
	cat >"$XBERG_WORK/bin/docker" <<'SCRIPT'
#!/usr/bin/env bash
set -euo pipefail
for last; do :; done
file="$FAKE_IMAGE_DIR/${last##*/}"
[ -f "$file" ] || { echo "cat: $last: No such file or directory" >&2; exit 1; }
cat "$file"
SCRIPT
	chmod +x "$XBERG_WORK/bin/docker"
	export PATH="$XBERG_WORK/bin:$PATH"
	export FAKE_IMAGE_DIR="$XBERG_WORK/image"
}

verify_image_notices() {
	run python3 "$REPOSITORY_ROOT/scripts/ci/docker/verify_image_notices.py" --image "$1"
}

@test "Image notice verifier should accept both canonical files byte for byte" {
	setup_image_verifier
	verify_image_notices xberg:core
	xberg_assert_status 0
	xberg_assert_output_contains 'xberg:core: packaged notices match the repository byte-for-byte'
}

@test "Image notice verifier should reject a notice missing from the image" {
	setup_image_verifier
	rm "$FAKE_IMAGE_DIR/THIRD_PARTY_LICENSES.md"
	verify_image_notices xberg:full
	xberg_assert_status 1
	xberg_assert_output_contains 'THIRD_PARTY_LICENSES.md is missing from /usr/local/share/licenses/xberg'
}

@test "Image notice verifier should reject a truncated notice" {
	setup_image_verifier
	head -c 10 "$REPOSITORY_ROOT/LICENSE" >"$FAKE_IMAGE_DIR/LICENSE"
	verify_image_notices xberg:cli
	xberg_assert_status 1
	xberg_assert_output_contains 'LICENSE is truncated (10 of'
}

@test "Image notice verifier should reject stale bytes after a matching license" {
	setup_image_verifier
	printf 'corruption\n' >>"$FAKE_IMAGE_DIR/THIRD_PARTY_LICENSES.md"
	verify_image_notices xberg:full
	xberg_assert_status 1
	xberg_assert_output_contains 'THIRD_PARTY_LICENSES.md differs from the repository copy'
}

@test "Image notice verifier should fail when Docker cannot read the image" {
	setup_image_verifier
	printf '#!/usr/bin/env bash\necho "Error: No such image" >&2\nexit 1\n' >"$XBERG_WORK/bin/docker"
	chmod +x "$XBERG_WORK/bin/docker"
	verify_image_notices xberg:core
	xberg_assert_status 1
	xberg_assert_output_contains 'LICENSE is missing from /usr/local/share/licenses/xberg (Error: No such image)'
}
