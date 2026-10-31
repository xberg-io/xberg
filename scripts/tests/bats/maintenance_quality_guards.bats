#!/usr/bin/env bats

setup() {
	[ -z "${XBERG_EXPECTED_BASH_MAJOR_MINOR:-}" ] || [[ "$BASH_VERSION" == "$XBERG_EXPECTED_BASH_MAJOR_MINOR".* ]]
	bats_load_library xberg-bats
	xberg_setup
	REPOSITORY_ROOT="$(cd "$BATS_TEST_DIRNAME/../../.." && pwd -P)"
	cd "$XBERG_WORK" || return
	unset DF_H_STATUS DF_BYTES_STATUS GIT_STATUS FORMAT_STATUS LINT_STATUS
	export SHELL_FILES='scripts/one.sh
scripts/with space.sh'
	xberg_stub git 'printf "git %s\n" "$*" >>"$XBERG_TRACE"' 'printf "%s\n" "$SHELL_FILES"' 'exit "${GIT_STATUS:-0}"'
	xberg_stub shfmt 'printf "shfmt\n" >>"$XBERG_TRACE"' 'printf "<%s>\n" "$@" >>"$XBERG_TRACE"' 'exit "${FORMAT_STATUS:-0}"'
	xberg_stub shellcheck 'printf "shellcheck\n" >>"$XBERG_TRACE"' 'printf "<%s>\n" "$@" >>"$XBERG_TRACE"' 'exit "${LINT_STATUS:-0}"'
	xberg_stub df 'printf "df %s\n" "$*" >>"$XBERG_TRACE"' \
		'case "$1" in -h) printf "human disk report\n"; exit "${DF_H_STATUS:-0}" ;; -B1) printf "bytes disk report\n"; exit "${DF_BYTES_STATUS:-0}" ;; *) exit 92 ;; esac'
}

@test "Disk diagnostics should print the selected label and query both human and byte reports" {
	run "$BASH" "$REPOSITORY_ROOT/scripts/ci/validate/show-disk-space.sh" 'Before link'
	xberg_assert_status 0
	xberg_assert_output_contains '=== Before link ==='
	xberg_assert_output_contains 'human disk report'
	xberg_assert_output_contains 'bytes disk report'
	xberg_assert_trace 'df -h /
df -B1 /'
}

@test "Disk diagnostics should accept unsupported optional byte formatting while retaining the primary report" {
	export DF_BYTES_STATUS=21
	run "$BASH" "$REPOSITORY_ROOT/scripts/ci/validate/show-disk-space.sh"
	xberg_assert_status 0
	xberg_assert_output_contains '=== Disk space ==='
	xberg_assert_output_contains 'human disk report'
	xberg_assert_trace 'df -h /
df -B1 /'
}

@test "Disk diagnostics should propagate a failed primary query without claiming another report" {
	export DF_H_STATUS=22
	run "$BASH" "$REPOSITORY_ROOT/scripts/ci/validate/show-disk-space.sh"
	xberg_assert_status 22
	xberg_assert_trace 'df -h /'
	[[ "$output" != *'Disk info:'* ]]
}

@test "Shell quality guard should check every tracked script while preserving paths containing spaces" {
	run "$BASH" "$REPOSITORY_ROOT/scripts/task/shell-lint.sh" check
	xberg_assert_status 0
	xberg_assert_trace 'git ls-files *.sh
shfmt
<-d>
<scripts/one.sh>
<scripts/with space.sh>
shellcheck
<scripts/one.sh>
<scripts/with space.sh>'
}

@test "Shell quality guard should select formatting fix mode by default before linting" {
	export SHELL_FILES=scripts/one.sh
	run "$BASH" "$REPOSITORY_ROOT/scripts/task/shell-lint.sh"
	xberg_assert_status 0
	xberg_assert_trace 'git ls-files *.sh
shfmt
<-w>
<scripts/one.sh>
shellcheck
<scripts/one.sh>'
}

@test "Shell quality guard should reject a zero-file sweep" {
	export SHELL_FILES=''
	run "$BASH" "$REPOSITORY_ROOT/scripts/task/shell-lint.sh" check
	xberg_assert_status 1
	xberg_assert_output_contains 'no tracked shell scripts found'
	xberg_assert_trace 'git ls-files *.sh'
}

@test "Shell quality guard should preserve a failed Git discovery instead of treating it as an empty success" {
	export GIT_STATUS=23 SHELL_FILES=''
	run "$BASH" "$REPOSITORY_ROOT/scripts/task/shell-lint.sh" check
	xberg_assert_status 23
	xberg_assert_trace 'git ls-files *.sh'
}

@test "Shell quality guard should reject invalid modes before discovering files" {
	export SHELL_FILES=''
	run "$BASH" "$REPOSITORY_ROOT/scripts/task/shell-lint.sh" unknown
	xberg_assert_status 2
	xberg_assert_output_contains 'Usage:'
	xberg_assert_trace_empty
}

@test "Shell quality guard should stop before linting after formatting fails" {
	export FORMAT_STATUS=24 SHELL_FILES=scripts/one.sh
	run "$BASH" "$REPOSITORY_ROOT/scripts/task/shell-lint.sh" check
	xberg_assert_status 24
	xberg_assert_trace 'git ls-files *.sh
shfmt
<-d>
<scripts/one.sh>'
}

@test "Shell quality guard should propagate actual lint failures" {
	export LINT_STATUS=25 SHELL_FILES=scripts/one.sh
	run "$BASH" "$REPOSITORY_ROOT/scripts/task/shell-lint.sh" check
	xberg_assert_status 25
	grep -Fxq 'shellcheck' "$XBERG_TRACE"
}
