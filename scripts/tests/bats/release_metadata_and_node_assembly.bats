#!/usr/bin/env bats

setup() {
	bats_load_library xberg-bats
	xberg_setup
	REPOSITORY_ROOT="$(cd "$BATS_TEST_DIRNAME/../../.." && pwd -P)"
	cd "$XBERG_WORK" || return
	mkdir -p scripts/publish artifacts definitions
	cp "$REPOSITORY_ROOT/scripts/publish/validate-and-compute-release-metadata.sh" scripts/publish/
	unset GITHUB_EVENT_NAME GITHUB_REF_NAME EVENT_RELEASE_TAG INPUT_TAG INPUT_REF INPUT_TARGETS INPUT_DRY_RUN INPUT_FORCE_REPUBLISH
}

assert_metadata() {
	printf '%s\n' "$output" >metadata.json
	python3 - "$@" <<'PY'
import json,sys
value=json.load(open('metadata.json'))
assert len(value)==27, len(value)
for entry in sys.argv[1:]:
 key,expected=entry.split('=',1)
 actual=str(value[key]).lower() if isinstance(value[key],bool) else value[key]
 assert actual==expected,(key,actual,expected)
PY
}

@test "release metadata should select every target for an official stable release" {
	run env GITHUB_EVENT_NAME=release EVENT_RELEASE_TAG=v1.4.0 "$BASH" "$REPOSITORY_ROOT/scripts/publish/validate-and-compute-release-metadata.sh"
	xberg_assert_status 0
	assert_metadata tag=v1.4.0 version=1.4.0 npm_tag=latest release_targets=all release_any=true is_tag=true matrix_ref=v1.4.0
}

@test "release metadata should normalize aliases and make Homebrew imply the CLI target" {
	run env GITHUB_EVENT_NAME=workflow_dispatch INPUT_TAG=v1.4.0-rc.2 INPUT_REF=topic INPUT_TARGETS='homebrew, CS, cffi' INPUT_DRY_RUN=true "$BASH" "$REPOSITORY_ROOT/scripts/publish/validate-and-compute-release-metadata.sh"
	xberg_assert_status 0
	assert_metadata npm_tag=next is_prerelease=true release_targets=cli,homebrew,csharp,c-ffi release_node=false checkout_ref=refs/heads/topic matrix_ref=topic dry_run=true
}

@test "release metadata should keep an explicit none selection empty without enabling default targets" {
	run env GITHUB_EVENT_NAME=workflow_dispatch INPUT_TAG=v1.4.0 INPUT_TARGETS=none "$BASH" "$REPOSITORY_ROOT/scripts/publish/validate-and-compute-release-metadata.sh"
	xberg_assert_status 0
	assert_metadata release_targets=none release_any=false release_cli=false release_python=false
}

@test "release metadata should preserve an exact commit while resolving the release matrix from main" {
	run env GITHUB_EVENT_NAME=workflow_dispatch INPUT_TAG=v1.4.0 INPUT_REF=1111111111111111111111111111111111111111 INPUT_TARGETS=node "$BASH" "$REPOSITORY_ROOT/scripts/publish/validate-and-compute-release-metadata.sh"
	xberg_assert_status 0
	assert_metadata target_sha=1111111111111111111111111111111111111111 checkout_ref=refs/heads/main matrix_ref=main is_tag=false
}

@test "release metadata should reject unknown targets and missing tags" {
	run env GITHUB_EVENT_NAME=workflow_dispatch INPUT_TAG=v1.4.0 INPUT_TARGETS=invalid "$BASH" "$REPOSITORY_ROOT/scripts/publish/validate-and-compute-release-metadata.sh"
	xberg_assert_status 1
	xberg_assert_output_contains 'Unknown release target'
	run env GITHUB_EVENT_NAME=release "$BASH" "$REPOSITORY_ROOT/scripts/publish/validate-and-compute-release-metadata.sh"
	xberg_assert_status 1
	xberg_assert_output_contains 'Release tag could not be determined'
}

@test "release metadata preparation should export every computed field exactly once" {
	run env GITHUB_EVENT_NAME=release EVENT_RELEASE_TAG=v1.4.0 "$BASH" "$REPOSITORY_ROOT/scripts/publish/prepare-release-metadata.sh"
	xberg_assert_status 0
	python3 - "$GITHUB_OUTPUT" <<'PY'
import json,sys
metadata=json.load(open('release-metadata.json'))
rows=open(sys.argv[1]).read().splitlines()
assert len(metadata)==len(rows)==27,(len(metadata),len(rows))
actual=dict(row.split('=',1) for row in rows)
expected={k:str(v).lower() if isinstance(v,bool) else str(v) for k,v in metadata.items()}
assert actual==expected,(actual,expected)
PY
}

@test "release metadata preparation should reject failed JSON extraction without exporting empty fields" {
	xberg_stub_exit jq 7
	run env GITHUB_EVENT_NAME=release EVENT_RELEASE_TAG=v1.4.0 "$BASH" "$REPOSITORY_ROOT/scripts/publish/prepare-release-metadata.sh"
	xberg_assert_status 7
	xberg_assert_github_output ''
}

make_node_archive() {
	mkdir -p package/linux-arm64
	printf '{"name":"platform-package"}\n' >package/linux-arm64/package.json
	printf 'native module bytes\n' >package/linux-arm64/addon.node
	tar -czf artifacts/platform.tar.gz -C package linux-arm64
	printf 'reviewed loader\n' >definitions/index.js
	printf 'type declarations\n' >definitions/index.d.ts
}

@test "Node artifact assembly should copy actual platform bytes and both wrapper files" {
	make_node_archive
	run "$BASH" "$REPOSITORY_ROOT/scripts/publish/prepare-node-artifacts.sh" artifacts definitions destination
	xberg_assert_status 0
	xberg_assert_file destination/npm/linux-arm64/addon.node 'native module bytes'
	xberg_assert_file destination/index.js 'reviewed loader'
	xberg_assert_file destination/index.d.ts 'type declarations'
	[ "$(find destination -type f | wc -l | tr -d '[:space:]')" -eq 4 ]
}

@test "Node artifact assembly should reject an empty artifact directory instead of a zero-package success" {
	run "$BASH" "$REPOSITORY_ROOT/scripts/publish/prepare-node-artifacts.sh" artifacts definitions destination
	xberg_assert_status 1
	xberg_assert_output_contains 'No platform packages were prepared'
}

@test "Node artifact assembly should reject archives containing only empty platform directories" {
	mkdir -p package/empty
	tar -czf artifacts/empty.tar.gz -C package empty
	run "$BASH" "$REPOSITORY_ROOT/scripts/publish/prepare-node-artifacts.sh" artifacts definitions destination
	xberg_assert_status 1
	xberg_assert_output_contains 'No platform packages were prepared'
}

@test "Node artifact assembly should propagate missing wrapper files rather than claiming they merged" {
	make_node_archive
	rm definitions/index.d.ts
	run "$BASH" "$REPOSITORY_ROOT/scripts/publish/prepare-node-artifacts.sh" artifacts definitions destination
	xberg_assert_status 1
	[[ "$output" != *'TypeScript definitions merged'* ]]
}

@test "Node artifact assembly should preserve extraction failures without claiming completion" {
	printf 'not an archive\n' >artifacts/broken.tar.gz
	run "$BASH" "$REPOSITORY_ROOT/scripts/publish/prepare-node-artifacts.sh" artifacts definitions destination
	[ "$status" -ne 0 ]
	[[ "$output" != *'prepared successfully'* ]]
}
