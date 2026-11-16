#!/usr/bin/env bats

setup() {
	bats_load_library xberg-bats
	xberg_setup
	REPOSITORY_ROOT="$(cd "$BATS_TEST_DIRNAME/../../.." && pwd -P)"
	SCRIPT="$REPOSITORY_ROOT/scripts/publish/go/tag-and-push-go-module.sh"
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
	export GITHUB_REPOSITORY=fixture/fixture
}

remote_tag_commit() {
	git -C "$CLONE" ls-remote origin "refs/tags/$1^{}" | cut -f1
}

@test "go module tagging should create both tags on the release commit" {
	run bash "$SCRIPT" v1.2.3
	xberg_assert_status 0
	release="$(git rev-parse 'v1.2.3^{commit}')"
	[ "$(remote_tag_commit packages/go/v1/v1.2.3)" = "$release" ]
	[ "$(remote_tag_commit packages/go/v1.2.3)" = "$release" ]
}

@test "go module tagging should be a no-op when the tags already name the release commit" {
	bash "$SCRIPT" v1.2.3
	git tag -d packages/go/v1/v1.2.3 packages/go/v1.2.3 >/dev/null
	run bash "$SCRIPT" v1.2.3
	xberg_assert_status 0
	xberg_assert_output_contains 'already points at'
}

@test "go module tagging should fail when a tag names a different commit" {
	git tag -a packages/go/v1/v1.2.3 HEAD~1 -m stale
	git push --quiet origin refs/tags/packages/go/v1/v1.2.3
	git tag -d packages/go/v1/v1.2.3 >/dev/null
	run bash "$SCRIPT" v1.2.3
	xberg_assert_status 1
	xberg_assert_output_contains 'not the release commit'
}
