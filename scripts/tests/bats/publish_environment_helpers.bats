#!/usr/bin/env bats

setup() {
	bats_load_library xberg-bats
	xberg_setup
	REPOSITORY_ROOT="$(cd "$BATS_TEST_DIRNAME/../../.." && pwd -P)"
	cd "$XBERG_WORK" || return
	mkdir home
	unset CLI_TARGET SUDO_STATUS DOCKER_STATUS DF_STATUS
	xberg_stub sudo 'printf "sudo %s\n" "$*" >>"$XBERG_TRACE"' 'exit "${SUDO_STATUS:-0}"'
	xberg_stub docker 'printf "docker %s\n" "$*" >>"$XBERG_TRACE"' 'exit "${DOCKER_STATUS:-0}"'
	xberg_stub df 'printf "df %s\n" "$*" >>"$XBERG_TRACE"' 'exit "${DF_STATUS:-0}"'
}

@test "Maven GPG patch should combine legacy arguments across whitespace without changing surrounding XML" {
	printf '<args>\n<arg>--pinentry-mode</arg>\n  <arg>loopback</arg>\n</args>\n' >pom.xml
	run "$BASH" "$REPOSITORY_ROOT/scripts/publish/patch-maven-gpg-args.sh" pom.xml
	xberg_assert_status 0
	xberg_assert_file pom.xml '<args>
<arg>--pinentry-mode=loopback</arg>
</args>'
}

@test "Maven GPG patch should preserve already-modern arguments exactly" {
	printf '<arg>--pinentry-mode=loopback</arg>\n' >pom.xml
	run "$BASH" "$REPOSITORY_ROOT/scripts/publish/patch-maven-gpg-args.sh" pom.xml
	xberg_assert_status 0
	xberg_assert_file pom.xml '<arg>--pinentry-mode=loopback</arg>'
}

@test "Maven GPG patch should reject missing input files" {
	run "$BASH" "$REPOSITORY_ROOT/scripts/publish/patch-maven-gpg-args.sh" missing.xml
	xberg_assert_status 1
	xberg_assert_output_contains 'pom.xml not found: missing.xml'
}

@test "GPG preference should create a forwarding shim only in the disposable home" {
	xberg_stub gpg2 'printf "gpg2\n" >>"$XBERG_TRACE"; printf "<%s>\n" "$@" >>"$XBERG_TRACE"'
	run env HOME="$XBERG_WORK/home" "$BASH" "$REPOSITORY_ROOT/scripts/publish/prefer-gpg2-binary.sh"
	xberg_assert_status 0
	xberg_assert_github_path "$XBERG_WORK/home/.local/bin"
	[ -x home/.local/bin/gpg ]
	run "$BASH" "$XBERG_WORK/home/.local/bin/gpg" --sign 'file with spaces'
	xberg_assert_status 0
	xberg_assert_trace 'gpg2
<--sign>
<file with spaces>'
}

@test "GPG preference should leave workflow paths untouched when gpg2 is absent" {
	xberg_shadow_system_path_without gpg2
	run env HOME="$XBERG_WORK/home" PATH="$(xberg_isolated_path)" "$BASH" "$REPOSITORY_ROOT/scripts/publish/prefer-gpg2-binary.sh"
	xberg_assert_status 0
	xberg_assert_output_contains 'gpg2 not found'
	xberg_assert_github_path ''
	xberg_assert_github_env ''
	[ ! -e home/.local ]
}

@test "CLI build dependency setup should install ARM cross compilers only for the selected ARM target" {
	run env CLI_TARGET=aarch64-unknown-linux-gnu "$BASH" "$REPOSITORY_ROOT/scripts/publish/cli/install-build-deps.sh"
	xberg_assert_status 0
	xberg_assert_trace 'sudo apt-get update
sudo apt-get install -y gcc-aarch64-linux-gnu g++-aarch64-linux-gnu'
}

@test "CLI build dependency setup should not install ARM tools for a native target" {
	run env CLI_TARGET=x86_64-unknown-linux-gnu "$BASH" "$REPOSITORY_ROOT/scripts/publish/cli/install-build-deps.sh"
	xberg_assert_status 0
	xberg_assert_trace 'sudo apt-get update'
}

@test "CLI build dependency setup should stop before installation when repository refresh fails" {
	export SUDO_STATUS=9
	run env CLI_TARGET=aarch64-unknown-linux-gnu "$BASH" "$REPOSITORY_ROOT/scripts/publish/cli/install-build-deps.sh"
	xberg_assert_status 9
	xberg_assert_trace 'sudo apt-get update'
}

@test "Docker cleanup should attempt both best-effort prunes and still inspect disk space after failures" {
	export DOCKER_STATUS=7
	run "$BASH" "$REPOSITORY_ROOT/scripts/publish/ubuntu/docker-prune.sh"
	xberg_assert_status 0
	xberg_assert_trace 'docker system prune -af --volumes
docker builder prune -af
df -h /'
}

@test "Docker cleanup should preserve failure of the final disk-space inspection" {
	export DF_STATUS=9
	run "$BASH" "$REPOSITORY_ROOT/scripts/publish/ubuntu/docker-prune.sh"
	xberg_assert_status 9
}
