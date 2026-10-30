#!/usr/bin/env bats

setup() {
	[ -z "${XBERG_EXPECTED_BASH_MAJOR_MINOR:-}" ] || [[ "$BASH_VERSION" == "$XBERG_EXPECTED_BASH_MAJOR_MINOR".* ]]
	bats_load_library xberg-bats
	xberg_setup
	REPOSITORY_ROOT="$(cd "$BATS_TEST_DIRNAME/../../.." && pwd -P)"
	mkdir -p "$XBERG_WORK/scripts/ci" "$XBERG_WORK/stage/lib"
	cp "$REPOSITORY_ROOT/LICENSE" "$REPOSITORY_ROOT/THIRD_PARTY_LICENSES.md" "$XBERG_WORK/"
	if [ -f "$REPOSITORY_ROOT/scripts/ci/package-notices.py" ]; then
		cp "$REPOSITORY_ROOT/scripts/ci/package-notices.py" "$XBERG_WORK/scripts/ci/"
	fi
}

notice_tool() {
	python3 "$XBERG_WORK/scripts/ci/package-notices.py" "$@"
}

@test "Notice staging should retain exact canonical bytes through a nested tar repack" {
	run notice_tool stage "$XBERG_WORK/stage"
	xberg_assert_status 0
	cmp "$XBERG_WORK/LICENSE" "$XBERG_WORK/stage/LICENSE"
	cmp "$XBERG_WORK/THIRD_PARTY_LICENSES.md" "$XBERG_WORK/stage/THIRD_PARTY_LICENSES.md"
	tar -czf "$XBERG_WORK/initial.tar.gz" -C "$XBERG_WORK" stage
	mkdir "$XBERG_WORK/unpacked"
	tar -xzf "$XBERG_WORK/initial.tar.gz" -C "$XBERG_WORK/unpacked"
	printf closure >"$XBERG_WORK/unpacked/stage/lib/libclosure.so"
	tar -czf "$XBERG_WORK/final.tar.gz" -C "$XBERG_WORK/unpacked" .
	run notice_tool verify "$XBERG_WORK/final.tar.gz" stage
	xberg_assert_status 0
	xberg_assert_output_contains 'Verified 2 canonical notice files'
}

@test "Notice staging should add both notices to npm file allowlists without losing binaries" {
	printf '{"files":["binding.node"]}\n' >"$XBERG_WORK/stage/package.json"
	run notice_tool stage "$XBERG_WORK/stage"
	xberg_assert_status 0
	run python3 -c 'import json,sys; assert json.load(open(sys.argv[1]))["files"] == ["LICENSE", "THIRD_PARTY_LICENSES.md", "binding.node"]' "$XBERG_WORK/stage/package.json"
	xberg_assert_status 0
}

@test "Notice verifier should compare both complete files in a ZIP archive" {
	notice_tool stage "$XBERG_WORK/stage"
	python3 - "$XBERG_WORK" <<'PYTHON'
import sys, zipfile
from pathlib import Path
root = Path(sys.argv[1])
with zipfile.ZipFile(root / "package.zip", "w") as archive:
    for notice in ("LICENSE", "THIRD_PARTY_LICENSES.md"):
        archive.write(root / "stage" / notice, notice)
PYTHON
	run notice_tool verify "$XBERG_WORK/package.zip" .
	xberg_assert_status 0
	xberg_assert_output_contains 'Verified 2 canonical notice files'
}

@test "Notice verifier should reject a missing third-party notice" {
	cp "$XBERG_WORK/LICENSE" "$XBERG_WORK/stage/"
	tar -czf "$XBERG_WORK/missing.tar.gz" -C "$XBERG_WORK/stage" .
	run notice_tool verify "$XBERG_WORK/missing.tar.gz" .
	xberg_assert_status 1
	xberg_assert_output_contains 'expected one THIRD_PARTY_LICENSES.md, found 0'
}

@test "Notice verifier should reject corrupt third-party bytes after a valid license" {
	notice_tool stage "$XBERG_WORK/stage"
	printf corruption >>"$XBERG_WORK/stage/THIRD_PARTY_LICENSES.md"
	tar -czf "$XBERG_WORK/corrupt.tar.gz" -C "$XBERG_WORK/stage" .
	run notice_tool verify "$XBERG_WORK/corrupt.tar.gz" .
	xberg_assert_status 1
	xberg_assert_output_contains 'differs from canonical THIRD_PARTY_LICENSES.md'
}

@test "Notice staging should reject an empty canonical source" {
	: >"$XBERG_WORK/THIRD_PARTY_LICENSES.md"
	run notice_tool stage "$XBERG_WORK/stage"
	xberg_assert_status 1
	xberg_assert_output_contains 'empty canonical notice: THIRD_PARTY_LICENSES.md'
}

@test "Notice tool should reject an empty destination inventory" {
	run notice_tool stage
	xberg_assert_status 2
	xberg_assert_output_contains 'required'
}
