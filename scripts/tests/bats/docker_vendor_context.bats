#!/usr/bin/env bats

setup() {
	[ -z "${XBERG_EXPECTED_BASH_MAJOR_MINOR:-}" ] || [[ "$BASH_VERSION" == "$XBERG_EXPECTED_BASH_MAJOR_MINOR".* ]]
	bats_load_library xberg-bats
	xberg_setup
	REPOSITORY_ROOT="$(cd "$BATS_TEST_DIRNAME/../../.." && pwd -P)"
	mkdir -p "$XBERG_WORK/docker" "$XBERG_WORK/vendored"
	cp "$REPOSITORY_ROOT/Cargo.toml" "$REPOSITORY_ROOT/.dockerignore" "$XBERG_WORK/"
	cp "$REPOSITORY_ROOT"/docker/Dockerfile.* "$XBERG_WORK/docker/"
	cp -R "$REPOSITORY_ROOT/vendored/sysinfo" "$XBERG_WORK/vendored/"
}

run_context_check() {
	run python3 - "$XBERG_WORK" <<'PYTHON'
import fnmatch
import shlex
import sys
import tomllib
from pathlib import Path, PurePosixPath

root = Path(sys.argv[1])
manifest = tomllib.loads((root / "Cargo.toml").read_text())
# The existing crate-coverage guard covers workspace self-patches.
members = set(manifest["workspace"]["members"])
patches = [PurePosixPath(value["path"]) for registry in manifest.get("patch", {}).values()
           for value in registry.values() if isinstance(value, dict) and "path" in value and value["path"] not in members]
assert len(patches) == 1, f"expected 1 non-workspace local patch root, found {len(patches)}"
dockerfiles = sorted((root / "docker").glob("Dockerfile.*"))
assert len(dockerfiles) == 11, f"expected 11 Dockerfiles, found {len(dockerfiles)}"
patterns = (root / ".dockerignore").read_text().splitlines()

def included(path):
    allowed = True
    names = [path.as_posix(), *(parent.as_posix() for parent in path.parents if parent != PurePosixPath("."))]
    for line in patterns:
        if not line or line.startswith("#"):
            continue
        pattern = line.removeprefix("!").strip("/")
        if any(fnmatch.fnmatchcase(name, pattern) for name in names):
            allowed = line.startswith("!")
    return allowed

def source_copies(path):
    workdir = PurePosixPath("/")
    copies = []
    manifest_roots = []
    for line in path.read_text().splitlines():
        if not line.startswith(("FROM ", "WORKDIR ", "COPY ")):
            continue
        tokens = shlex.split(line, comments=True)
        if not tokens:
            continue
        if tokens[0] == "FROM":
            workdir = PurePosixPath("/")
            copies = []
        elif tokens[0] == "WORKDIR":
            workdir = workdir / tokens[1]
        elif tokens[0] == "COPY" and not any(token.startswith("--") for token in tokens[1:]):
            destination = workdir / tokens[-1]
            for source in tokens[1:-1]:
                copies.append((PurePosixPath(source), destination))
            if "Cargo.toml" in tokens[1:-1]:
                manifest_roots.append((destination, copies))
    return manifest_roots

checked = 0
for path in dockerfiles:
    stages = source_copies(path)
    assert len(stages) == 1, f"{path.name}: expected 1 root manifest stage, found {len(stages)}"
    manifest_root, copies = stages[0]
    for patch in patches:
        assert (root / patch / "Cargo.toml").is_file(), f"missing patch manifest: {patch}"
        assert any(patch.is_relative_to(source) and destination / patch.relative_to(source) == manifest_root / patch
                   for source, destination in copies), f"{path.name}: local patch {patch} not copied into manifest root"
        files = [file.relative_to(root) for file in (root / patch).rglob("*") if file.is_file()]
        assert len(files) == 144, f"{patch}: expected 142 source files and 2 provenance files, found {len(files)}"
        for file in files:
            assert included(file), f"{path.name}: .dockerignore excludes local patch file {file}"
        checked += 1
assert checked == 11, checked
print(f"Verified {len(patches)} non-workspace local patch root across {len(dockerfiles)} Dockerfiles; {checked} build contexts")
PYTHON
}

@test "Docker contexts should copy every non-workspace local Cargo patch and its full source inventory" {
	run_context_check
	xberg_assert_status 0
	xberg_assert_output_contains 'Verified 1 non-workspace local patch root across 11 Dockerfiles; 11 build contexts'
}

@test "Docker context check should reject a missing vendor COPY in one source builder" {
	sed '/^COPY vendored\//d' "$XBERG_WORK/docker/Dockerfile.core" >"$XBERG_WORK/core.tmp"
	mv "$XBERG_WORK/core.tmp" "$XBERG_WORK/docker/Dockerfile.core"
	run_context_check
	xberg_assert_status 1
	xberg_assert_output_contains 'Dockerfile.core: local patch vendored/sysinfo not copied into manifest root'
}

@test "Docker context check should reject a vendor COPY into the wrong destination" {
	sed 's#COPY vendored/sysinfo/ vendored/sysinfo/#COPY vendored/sysinfo/ wrong/#' \
		"$XBERG_WORK/docker/Dockerfile.cli" >"$XBERG_WORK/cli.tmp"
	mv "$XBERG_WORK/cli.tmp" "$XBERG_WORK/docker/Dockerfile.cli"
	run_context_check
	xberg_assert_status 1
	xberg_assert_output_contains 'Dockerfile.cli: local patch vendored/sysinfo not copied into manifest root'
}

@test "Docker context check should reject a patch manifest excluded by dockerignore" {
	sed '/^!vendored/d' "$XBERG_WORK/.dockerignore" >"$XBERG_WORK/ignore.tmp"
	mv "$XBERG_WORK/ignore.tmp" "$XBERG_WORK/.dockerignore"
	run_context_check
	xberg_assert_status 1
	xberg_assert_output_contains '.dockerignore excludes local patch file vendored/sysinfo/'
}

@test "Docker context check should reject an empty Dockerfile sweep" {
	rm "$XBERG_WORK"/docker/Dockerfile.*
	run_context_check
	xberg_assert_status 1
	xberg_assert_output_contains 'expected 11 Dockerfiles, found 0'
}

@test "Docker context check should reject an empty local patch inventory" {
	sed 's/^\[patch.crates-io\]/[unused.crates-io]/' "$XBERG_WORK/Cargo.toml" >"$XBERG_WORK/manifest.tmp"
	mv "$XBERG_WORK/manifest.tmp" "$XBERG_WORK/Cargo.toml"
	run_context_check
	xberg_assert_status 1
	xberg_assert_output_contains 'expected 1 non-workspace local patch root, found 0'
}
