#!/usr/bin/env bats

setup() {
	bats_load_library xberg-bats
	xberg_setup
	REPOSITORY_ROOT="$(cd "$BATS_TEST_DIRNAME/../../.." && pwd -P)"
	SCRIPT="$REPOSITORY_ROOT/scripts/publish/go/tag-and-push-go-module.sh"
	unset EXISTING_TAG_COMMIT GIT_PUSH_STATUS GH_API_STATUS
	export GITHUB_REPOSITORY=example/project
	xberg_stub git \
		'printf "git %s\n" "$*" >>"$XBERG_TRACE"' \
		'case "$1" in' \
		'  rev-parse)' \
		'    if [ "$2" = "v1.4.0^{commit}" ]; then printf "%s\n" 0123456789abcdef0123456789abcdef01234567; exit 0; fi' \
		'    if [ -n "${EXISTING_TAG_COMMIT:-}" ]; then printf "%s\n" "$EXISTING_TAG_COMMIT"; exit 0; fi' \
		'    exit 1 ;;' \
		'  fetch) exit 0 ;;' \
		'  tag) exit 0 ;;' \
		'  push) exit "${GIT_PUSH_STATUS:-0}" ;;' \
		'  *) echo "unexpected git invocation" >&2; exit 97 ;;' \
		'esac'
	xberg_stub gh \
		'printf "gh %s\n" "$*" >>"$XBERG_TRACE"' \
		'exit "${GH_API_STATUS:-0}"'
}

@test "Go publication should create and push both annotated module tags at the release ref" {
	run bash "$SCRIPT" v1.4.0
	xberg_assert_status 0
	xberg_assert_trace \
		'git rev-parse v1.4.0^{commit}' \
		'git fetch --quiet --force origin refs/tags/packages/go/v1/v1.4.0:refs/tags/packages/go/v1/v1.4.0' \
		'git rev-parse --verify --quiet packages/go/v1/v1.4.0^{commit}' \
		'git tag -a packages/go/v1/v1.4.0 v1.4.0 -m Go module tag packages/go/v1/v1.4.0' \
		'git push origin refs/tags/packages/go/v1/v1.4.0' \
		'git fetch --quiet --force origin refs/tags/packages/go/v1.4.0:refs/tags/packages/go/v1.4.0' \
		'git rev-parse --verify --quiet packages/go/v1.4.0^{commit}' \
		'git tag -a packages/go/v1.4.0 v1.4.0 -m Go module tag packages/go/v1.4.0' \
		'git push origin refs/tags/packages/go/v1.4.0'
}

@test "Go publication should leave module tags already at the release commit untouched" {
	export EXISTING_TAG_COMMIT=0123456789abcdef0123456789abcdef01234567
	run bash "$SCRIPT" v1.4.0
	xberg_assert_status 0
	xberg_assert_trace \
		'git rev-parse v1.4.0^{commit}' \
		'git fetch --quiet --force origin refs/tags/packages/go/v1/v1.4.0:refs/tags/packages/go/v1/v1.4.0' \
		'git rev-parse --verify --quiet packages/go/v1/v1.4.0^{commit}' \
		'git fetch --quiet --force origin refs/tags/packages/go/v1.4.0:refs/tags/packages/go/v1.4.0' \
		'git rev-parse --verify --quiet packages/go/v1.4.0^{commit}'
	[ "$(printf '%s\n' "$output" | grep -c 'already points at 0123456789ab; skipping')" -eq 2 ]
}

@test "Go publication should refuse a module tag that names another commit" {
	export EXISTING_TAG_COMMIT=deadbeefdeadbeefdeadbeefdeadbeefdeadbeef
	run bash "$SCRIPT" v1.4.0
	xberg_assert_status 1
	xberg_assert_output_contains 'points at deadbeefdead, not the release commit 0123456789ab'
	xberg_assert_trace \
		'git rev-parse v1.4.0^{commit}' \
		'git fetch --quiet --force origin refs/tags/packages/go/v1/v1.4.0:refs/tags/packages/go/v1/v1.4.0' \
		'git rev-parse --verify --quiet packages/go/v1/v1.4.0^{commit}'
}

@test "Go publication should fall back to the API with the exact resolved commit when push fails" {
	export GIT_PUSH_STATUS=42
	run bash "$SCRIPT" v1.4.0
	xberg_assert_status 0
	grep -Fxq 'gh api repos/example/project/git/refs -f ref=refs/tags/packages/go/v1/v1.4.0 -f sha=0123456789abcdef0123456789abcdef01234567 --silent' "$XBERG_TRACE"
	grep -Fxq 'gh api repos/example/project/git/refs -f ref=refs/tags/packages/go/v1.4.0 -f sha=0123456789abcdef0123456789abcdef01234567 --silent' "$XBERG_TRACE"
	[ "$(grep -c '^gh api ' "$XBERG_TRACE")" -eq 2 ]
}

@test "Go publication should stop before the legacy tag if its API fallback fails" {
	export GIT_PUSH_STATUS=1 GH_API_STATUS=42
	run bash "$SCRIPT" v1.4.0
	xberg_assert_status 42
	grep -Fxq 'gh api repos/example/project/git/refs -f ref=refs/tags/packages/go/v1/v1.4.0 -f sha=0123456789abcdef0123456789abcdef01234567 --silent' "$XBERG_TRACE"
	[ "$(grep -c '^gh api ' "$XBERG_TRACE")" -eq 1 ]
	run grep -Fq 'packages/go/v1.4.0:refs' "$XBERG_TRACE"
	xberg_assert_status 1
}

@test "Go publication should reject a missing release tag before invoking git or GitHub" {
	run bash "$SCRIPT"
	[ "$status" -ne 0 ]
	xberg_assert_output_contains 'Release tag argument required'
	xberg_assert_trace_empty
}

# The stubbed tests above pin the call sequence; these run the script against real git, which
# is what proves the fetch refspec and the annotated-tag peel actually behave as assumed. ~keep
real_git_release() {
	rm "$XBERG_STUB_BIN/git"
	ORIGIN="$XBERG_WORK/origin.git"
	CLONE="$XBERG_WORK/clone"
	git init --quiet --bare "$ORIGIN"
	git clone --quiet "$ORIGIN" "$CLONE"
	cd "$CLONE"
	git config user.name fixture
	git config user.email fixture@example.invalid
	git commit --quiet --allow-empty -m first
	git commit --quiet --allow-empty -m second
	git tag -a v1.2.3 -m release
	git push --quiet origin HEAD:refs/heads/main refs/tags/v1.2.3
}

remote_tag_commit() {
	git -C "$CLONE" ls-remote origin "refs/tags/$1^{}" | cut -f1
}

@test "Go publication against real git should tag the release commit and re-run as a no-op" {
	real_git_release
	run bash "$SCRIPT" v1.2.3
	xberg_assert_status 0
	release="$(git rev-parse 'v1.2.3^{commit}')"
	[ "$(remote_tag_commit packages/go/v1/v1.2.3)" = "$release" ]
	[ "$(remote_tag_commit packages/go/v1.2.3)" = "$release" ]
	git tag -d packages/go/v1/v1.2.3 packages/go/v1.2.3 >/dev/null
	run bash "$SCRIPT" v1.2.3
	xberg_assert_status 0
	[ "$(printf '%s\n' "$output" | grep -c 'already points at')" -eq 2 ]
}

@test "Go publication against real git should fail when a remote tag names another commit" {
	real_git_release
	git tag -a packages/go/v1/v1.2.3 HEAD~1 -m stale
	git push --quiet origin refs/tags/packages/go/v1/v1.2.3
	git tag -d packages/go/v1/v1.2.3 >/dev/null
	run bash "$SCRIPT" v1.2.3
	xberg_assert_status 1
	xberg_assert_output_contains 'not the release commit'
}
