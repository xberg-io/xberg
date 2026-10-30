#!/usr/bin/env bats

setup() {
	bats_load_library xberg-bats
	xberg_setup
	REPOSITORY_ROOT="$(cd "$BATS_TEST_DIRNAME/../../.." && pwd -P)"
	export REAL_GIT
	REAL_GIT="$(command -v git)"
	export GIT_CONFIG_NOSYSTEM=1 GIT_CONFIG_GLOBAL=/dev/null
	export GIT_AUTHOR_NAME=Fixture GIT_AUTHOR_EMAIL=fixture@example.invalid
	export GIT_COMMITTER_NAME=Fixture GIT_COMMITTER_EMAIL=fixture@example.invalid
	export SUBMODULE_GITLINK_TIMEOUT=3
	cd "$XBERG_WORK"
	git -c init.defaultBranch=main init -q remote
	printf 'published fixture\n' >remote/fixture
	git -C remote add fixture
	git -C remote -c commit.gpgsign=false -c core.hooksPath=/dev/null commit -qm fixture
	PUBLISHED_SHA="$(git -C remote rev-parse HEAD)"
	git -c init.defaultBranch=main init -q super
	cd super
	git config -f .gitmodules submodule.corpus.path corpus
	git config -f .gitmodules submodule.corpus.url "$XBERG_WORK/remote"
	git add .gitmodules
	git update-index --add --cacheinfo "160000,$PUBLISHED_SHA,corpus"
	git -c commit.gpgsign=false -c core.hooksPath=/dev/null commit -qm superproject
}

@test "submodule gate should fetch the actual committed gitlink without a local checkout" {
	[ ! -d corpus ]
	run bash "$REPOSITORY_ROOT/scripts/ci/check-submodule-gitlinks.sh"
	xberg_assert_status 0
	xberg_assert_output_contains "gitlink $PUBLISHED_SHA is reachable on $XBERG_WORK/remote"
}

@test "submodule gate should reject an unpublished gitlink even in network-warning mode" {
	missing_sha=1111111111111111111111111111111111111111
	git update-index --cacheinfo "160000,$missing_sha,corpus"
	git -c commit.gpgsign=false -c core.hooksPath=/dev/null commit -qm unpublished
	run bash "$REPOSITORY_ROOT/scripts/ci/check-submodule-gitlinks.sh" --on-fetch-failure=warn
	xberg_assert_status 1
	xberg_assert_output_contains "points at $missing_sha, which is NOT reachable on its remote"
}

@test "submodule gate should use HEAD rather than an uncommitted index gitlink" {
	git update-index --cacheinfo 160000,1111111111111111111111111111111111111111,corpus
	run bash "$REPOSITORY_ROOT/scripts/ci/check-submodule-gitlinks.sh"
	xberg_assert_status 0
	xberg_assert_output_contains "gitlink $PUBLISHED_SHA is reachable"
}

@test "submodule gate should distinguish unreachable remotes from missing commits" {
	xberg_stub git \
		'if [ "${1:-}" = ls-remote ]; then exit 7; fi' \
		'exec "$REAL_GIT" "$@"'
	run bash "$REPOSITORY_ROOT/scripts/ci/check-submodule-gitlinks.sh"
	xberg_assert_status 1
	xberg_assert_output_contains 'error: could not reach remote'
	run bash "$REPOSITORY_ROOT/scripts/ci/check-submodule-gitlinks.sh" --on-fetch-failure=warn
	xberg_assert_status 0
	xberg_assert_output_contains 'warning: could not reach remote'
}

@test "submodule gate should reject an unknown leniency argument" {
	run bash "$REPOSITORY_ROOT/scripts/ci/check-submodule-gitlinks.sh" --on-fetch-failure=ignore
	xberg_assert_status 2
	xberg_assert_output_contains "unrecognized argument '--on-fetch-failure=ignore'"
}
