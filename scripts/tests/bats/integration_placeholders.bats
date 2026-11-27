#!/usr/bin/env bats

setup() {
	bats_load_library xberg-bats
	xberg_setup
	REPOSITORY_ROOT="$(cd "$BATS_TEST_DIRNAME/../../.." && pwd -P)"
	SCRIPT="$REPOSITORY_ROOT/integrations/scripts/publish-placeholders.sh"
	export NPM_CAPTURE="$XBERG_WORK/published"
	export TMPDIR="$XBERG_WORK"
	mkdir -p "$NPM_CAPTURE"
	unset NPM_ALREADY_PUBLISHED NPM_PUBLISH_STATUS
	xberg_stub npm \
		'printf "%s\n" "$*" >>"$XBERG_TRACE"' \
		'case "$1" in' \
		'view) exit "${NPM_ALREADY_PUBLISHED:-1}" ;;' \
		'publish)' \
		'  printf "%s\n" "$PWD" >"$NPM_CAPTURE/directory"' \
		'  cp package.json README.md "$NPM_CAPTURE/"' \
		'  exit "${NPM_PUBLISH_STATUS:-0}" ;;' \
		'*) echo "unexpected npm invocation" >&2; exit 97 ;;' \
		'esac'
}

@test "integration placeholders should skip exactly the two published package names" {
	export NPM_ALREADY_PUBLISHED=0
	run bash "$SCRIPT"
	xberg_assert_status 0
	xberg_assert_trace \
		'view @xberg-io/n8n-nodes-xberg@0.0.1 version' \
		'view @xberg-io/langchain-xberg@0.0.1 version' \
	xberg_assert_file_absent "$NPM_CAPTURE/package.json"
}

@test "integration placeholders should publish only the selected synthetic package" {
	run bash "$SCRIPT" @xberg-io/langchain-xberg
	xberg_assert_status 0
	xberg_assert_trace \
		'view @xberg-io/langchain-xberg@0.0.1 version' \
		'publish --access public'
	python3 - "$NPM_CAPTURE/package.json" <<'PY'
import json
import pathlib
import sys

package = json.loads(pathlib.Path(sys.argv[1]).read_text())
assert package == {
    "name": "@xberg-io/langchain-xberg",
    "version": "0.0.1",
    "description": "LangChain.js document loader for Xberg document extraction (placeholder release).",
    "license": "MIT",
    "homepage": "https://github.com/xberg-io/xberg",
    "publishConfig": {"access": "public"},
}, package
PY
	grep -Fq '**Placeholder release.**' "$NPM_CAPTURE/README.md"
	local published_directory
	published_directory="$(cat "$NPM_CAPTURE/directory")"
	[ -n "$published_directory" ]
	[ "$published_directory" != "$REPOSITORY_ROOT/integrations/langchain" ]
	[ ! -e "$published_directory" ]
}

@test "integration placeholders should reject an unknown package before calling npm" {
	run bash "$SCRIPT" @xberg-io/unknown-integration
	xberg_assert_status 1
	xberg_assert_output_contains 'unknown package: @xberg-io/unknown-integration'
	xberg_assert_trace_empty
}

@test "integration placeholders should propagate a publication failure" {
	export NPM_PUBLISH_STATUS=42
	run bash "$SCRIPT" @xberg-io/langchain-xberg
	xberg_assert_status 42
	xberg_assert_trace \
		'view @xberg-io/langchain-xberg@0.0.1 version' \
		'publish --access public'
	[[ "$output" != *'Done. Configure'* ]]
}
