#!/usr/bin/env bats
#
# Contract tests for plugin/scripts/mcp-launch.sh.
#
# This launcher pins the release it ships with (PINNED_VERSION) and runs the exact pinned npm
# package through npx before any download, so the suite is no longer byte-identical with the
# sibling repositories' copies.
#
# The suite tests strictly through the process boundary: it runs the script and asserts the
# exit status, which launcher was exec'd, and that argv arrives intact. Internal names never
# appear, which is what lets one file cover five divergent implementations. Every parameter it
# needs is read out of the script in setup_file, so it names no product.
#
# The script exec()s, so the launcher it chooses BECOMES the process -- a stub that prints a
# marker is therefore direct evidence of the branch taken, not an inference from log text. ~keep

setup_file() {
	bats_load_library xberg-bats

	REPO_ROOT="$(cd "$BATS_TEST_DIRNAME/../../.." && pwd -P)"
	SCRIPT="$REPO_ROOT/plugin/scripts/mcp-launch.sh"
	[ -f "$SCRIPT" ] || {
		echo "no plugin/scripts/mcp-launch.sh under $REPO_ROOT" >&2
		return 1
	}

	# Read the per-product parameters out of the script itself. Deriving them is what keeps
	# this file identical across repos; an empty derivation would silently turn most assertions
	# below into comparisons between two empty strings, so each one is checked. ~keep
	BINARY_NAME="$(sed -n 's/^BINARY_NAME="\(.*\)"$/\1/p' "$SCRIPT" | head -n 1)"
	PINNED_VERSION="$(sed -n 's/^PINNED_VERSION="\(.*\)"[[:space:]]*\(#.*\)\{0,1\}$/\1/p' "$SCRIPT" | head -n 1)"
	LAUNCHER_VAR="$(sed -n 's/^LAUNCHER="\${\([A-Z_]*\):-auto}"$/\1/p' "$SCRIPT" | head -n 1)"

	local name
	for name in BINARY_NAME PINNED_VERSION LAUNCHER_VAR; do
		[ -n "${!name}" ] || {
			echo "could not read ${name} out of ${SCRIPT}; the suite would assert nothing" >&2
			return 1
		}
	done

	# The launchers must be genuinely absent, not merely shadowed by a stub: the script decides
	# what to do by probing `command -v brew`, so a prepended stub directory cannot express "this
	# machine has no brew" and the probe would find the developer's real copy. The product binary
	# is excluded for the same reason -- a developer with it installed would otherwise take the
	# PATH branch in every test. ~keep
	xberg_shadow_system_path_without brew curl wget node npx "$BINARY_NAME"

	export REPO_ROOT SCRIPT BINARY_NAME PINNED_VERSION LAUNCHER_VAR
}

setup() {
	bats_load_library xberg-bats
	xberg_setup_isolated

	PLUGIN_ROOT="$XBERG_WORK/plugin"
	mkdir -p "$PLUGIN_ROOT"
	LAUNCH_MODE="auto"
}

# Run the script in a clean environment with the isolated PATH.
#
# `env -i` rather than merely overriding PATH: the launcher reads CLAUDE_PLUGIN_ROOT and its own
# mode variable, and a developer who happens to export either would otherwise change the result
# of the suite on their machine only.
# XBERG_TRACE is forwarded explicitly. `env -i` strips it otherwise, and a trace stub whose
# output file is unset fails silently and writes nothing -- which makes xberg_assert_trace_empty
# pass whether or not the command was called. That is a test that cannot fail, so the variable is
# part of the contract of this helper rather than something a caller remembers. ~keep
launch() {
	run env -i \
		PATH="$(xberg_isolated_path)" \
		HOME="$XBERG_WORK/home" \
		XBERG_TRACE="$XBERG_TRACE" \
		CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" \
		"${LAUNCHER_VAR}=${LAUNCH_MODE}" \
		"$SCRIPT" "$@"
}

# An executable that reports a version, announces itself, and echoes argv one line per argument.
#
# The unconditional `<marker>:ran` line answers "which binary was exec'd" even when the script is
# launched with no arguments; the per-argument lines answer "what argv arrived". Keeping them
# separate matters because joining argv with "$*" cannot tell `a b` from `"a b"`, which is
# exactly the pass-through bug this suite needs to be able to see. ~keep
make_cli() {
	local target="$1" marker="${2:-CLI}"
	mkdir -p "$(dirname "$target")"
	printf '%s\n' \
		'#!/usr/bin/env bash' \
		"if [ \"\$1\" = \"--version\" ]; then printf '%s\\n' \"${VERSION_REPORTED:-$PINNED_VERSION}\"; exit 0; fi" \
		"printf '${marker}:ran\\n'" \
		"for a in \"\$@\"; do printf '${marker}[%s]\\n' \"\$a\"; done" \
		'exit 0' >"$target"
	chmod +x "$target"
}

# An executable that refuses to report a version, which is how the script decides a binary it
# found is not usable.
make_broken_cli() {
	local target="$1"
	mkdir -p "$(dirname "$target")"
	printf '%s\n' \
		'#!/usr/bin/env bash' \
		'if [ "$1" = "--version" ]; then exit 1; fi' \
		'printf "BROKEN:ran\n"' \
		'exit 0' >"$target"
	chmod +x "$target"
}

cached_binary() { printf '%s/bin/%s' "$PLUGIN_ROOT" "$BINARY_NAME"; }

# --- launcher selection -------------------------------------------------------------------------

@test "mcp-launch should_reject_an_unrecognised_launcher_value" {
	LAUNCH_MODE="wget-please"
	launch

	xberg_assert_status 1
	xberg_assert_output_contains "invalid ${LAUNCHER_VAR}='wget-please'"
	xberg_assert_output_contains "auto|npx|brew|download"
}

@test "mcp-launch should_accept_every_documented_launcher_value" {
	local mode
	for mode in auto npx brew download; do
		LAUNCH_MODE="$mode"
		launch
		# Each of these still fails -- nothing is installed -- but it must fail at the end, not
		# at the argument check. Asserting the absence of the validation error is what separates
		# "the mode is accepted" from "the mode happens to fail the same way". ~keep
		[[ "$output" != *"invalid ${LAUNCHER_VAR}"* ]] || {
			echo "mode '$mode' was rejected as invalid" >&2
			return 1
		}
	done
}

@test "mcp-launch should_report_installation_instructions_when_no_strategy_can_provide_a_binary" {
	launch

	xberg_assert_status 1
	xberg_assert_output_contains "could not locate or install"
	xberg_assert_output_contains "npm install -g @xberg-io/xberg-cli@${PINNED_VERSION}"
	xberg_assert_output_contains "cargo install"
}

# --- cached and PATH binaries -------------------------------------------------------------------

@test "mcp-launch should_exec_the_cached_binary_when_it_reports_a_version" {
	make_cli "$(cached_binary)" CACHED
	launch

	xberg_assert_status 0
	[[ "$output" == *"CACHED:ran"* ]]
}

@test "mcp-launch should_pass_arguments_through_to_the_cached_binary_verbatim" {
	make_cli "$(cached_binary)" CACHED
	launch --config "a b" --flag

	xberg_assert_status 0
	# One line per argument: "a b" must survive as a single argument, not become two.
	xberg_assert_output_contains "CACHED[--config]"
	xberg_assert_output_contains "CACHED[a b]"
	xberg_assert_output_contains "CACHED[--flag]"
}

@test "mcp-launch should_ignore_the_cached_binary_when_it_cannot_report_a_version" {
	make_broken_cli "$(cached_binary)"
	launch

	xberg_assert_status 1
	[[ "$output" != *"BROKEN:ran"* ]]
}

@test "mcp-launch should_exec_the_binary_on_path_when_there_is_no_cached_binary" {
	make_cli "$XBERG_STUB_BIN/$BINARY_NAME" ONPATH
	launch

	xberg_assert_status 0
	[[ "$output" == *"ONPATH:ran"* ]]
}

@test "mcp-launch should_ignore_a_path_binary_that_cannot_report_a_version" {
	make_broken_cli "$XBERG_STUB_BIN/$BINARY_NAME"
	launch

	xberg_assert_status 1
	[[ "$output" != *"BROKEN:ran"* ]]
}

@test "mcp-launch should_prefer_the_cached_binary_over_the_one_on_path" {
	make_cli "$(cached_binary)" CACHED
	make_cli "$XBERG_STUB_BIN/$BINARY_NAME" ONPATH
	launch

	xberg_assert_status 0
	[[ "$output" == *"CACHED:ran"* ]]
	[[ "$output" != *"ONPATH:ran"* ]]
}

# --- malloc magazines ---------------------------------------------------------------------------

# An executable that reports a version, then prints the MallocMaxMagazines value it received.
make_env_cli() {
	local target="$1"
	mkdir -p "$(dirname "$target")"
	printf '%s\n' \
		'#!/usr/bin/env bash' \
		"if [ \"\$1\" = \"--version\" ]; then printf '%s\\n' \"${PINNED_VERSION}\"; exit 0; fi" \
		'printf "MAGAZINES[%s]\n" "${MallocMaxMagazines-<unset>}"' >"$target"
	chmod +x "$target"
}

stub_kernel() {
	xberg_stub uname "printf '%s\\n' '$1'"
}

# `launch` with MallocMaxMagazines preset in the clean environment.
launch_with_magazines() {
	run env -i \
		PATH="$(xberg_isolated_path)" \
		HOME="$XBERG_WORK/home" \
		CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" \
		"${LAUNCHER_VAR}=${LAUNCH_MODE}" \
		MallocMaxMagazines="$1" \
		"$SCRIPT"
}

@test "mcp-launch should_give_the_binary_one_malloc_magazine_on_macos" {
	stub_kernel Darwin
	make_env_cli "$(cached_binary)"
	launch

	xberg_assert_status 0
	xberg_assert_output_contains "MAGAZINES[1]"
}

@test "mcp-launch should_give_a_path_binary_one_malloc_magazine_on_macos" {
	stub_kernel Darwin
	make_env_cli "$XBERG_STUB_BIN/$BINARY_NAME"
	launch

	xberg_assert_status 0
	xberg_assert_output_contains "MAGAZINES[1]"
}

@test "mcp-launch should_keep_a_malloc_magazine_value_the_user_set" {
	stub_kernel Darwin
	make_env_cli "$(cached_binary)"
	local preset
	for preset in 4 ""; do
		launch_with_magazines "$preset"
		xberg_assert_status 0
		xberg_assert_output_contains "MAGAZINES[$preset]"
	done
}

@test "mcp-launch should_not_set_malloc_magazines_off_macos" {
	stub_kernel Linux
	make_env_cli "$(cached_binary)"
	launch

	xberg_assert_status 0
	xberg_assert_output_contains "MAGAZINES[<unset>]"
}

@test "mcp-launch should_leave_malloc_magazines_to_the_npm_launcher_when_it_runs_npx" {
	stub_kernel Darwin
	xberg_stub node 'exit 0'
	xberg_stub npx 'printf "NPX[%s]\n" "${MallocMaxMagazines-<unset>}"'
	launch

	xberg_assert_status 0
	xberg_assert_output_contains "NPX[<unset>]"
}

# --- version gate -------------------------------------------------------------------------------

@test "mcp-launch should_ignore_a_cached_binary_older_than_the_pinned_version" {
	VERSION_REPORTED="0.0.1"
	make_cli "$(cached_binary)" CACHED
	launch

	xberg_assert_status 1
	[[ "$output" != *"CACHED:ran"* ]]
	xberg_assert_output_contains "older than the pinned ${PINNED_VERSION}"
}

@test "mcp-launch should_ignore_a_path_binary_older_than_the_pinned_version" {
	VERSION_REPORTED="0.0.1"
	make_cli "$XBERG_STUB_BIN/$BINARY_NAME" ONPATH
	launch

	xberg_assert_status 1
	[[ "$output" != *"ONPATH:ran"* ]]
}

@test "mcp-launch should_ignore_a_prerelease_of_the_pinned_version" {
	VERSION_REPORTED="${PINNED_VERSION}-rc.1"
	make_cli "$(cached_binary)" CACHED
	launch

	xberg_assert_status 1
	[[ "$output" != *"CACHED:ran"* ]]
}

@test "mcp-launch should_exec_a_binary_newer_than_the_pinned_version" {
	VERSION_REPORTED="999.0.0"
	make_cli "$(cached_binary)" CACHED
	launch

	xberg_assert_status 0
	[[ "$output" == *"CACHED:ran"* ]]
}

# --- npx ----------------------------------------------------------------------------------------

# node and npx stubs. The npx stub records its argv one line per argument in $XBERG_STUB_BIN/npx.args
# and announces itself, so the exact package spec and the pass-through are both observable. ~keep
make_npx() {
	xberg_stub node 'exit 0'
	xberg_stub npx \
		"printf '%s\\n' \"\$@\" >\"\$0.args\"" \
		"printf 'NPX:ran\\n'" \
		'exit 0'
}

@test "mcp-launch should_run_the_exact_pinned_npm_package_through_npx_and_pass_arguments_through" {
	make_npx
	launch mcp --transport stdio "a b"

	xberg_assert_status 0
	[[ "$output" == *"NPX:ran"* ]]
	xberg_assert_file "$XBERG_STUB_BIN/npx.args" "--yes
--quiet
--no-fund
--no-audit
@xberg-io/xberg-cli@${PINNED_VERSION}
mcp
--transport
stdio
a b"
}

@test "mcp-launch should_prefer_a_path_binary_over_npx" {
	make_npx
	make_cli "$XBERG_STUB_BIN/$BINARY_NAME" ONPATH
	launch

	xberg_assert_status 0
	[[ "$output" == *"ONPATH:ran"* ]]
	[[ "$output" != *"NPX:ran"* ]]
}

@test "mcp-launch should_prefer_the_cached_binary_over_npx" {
	make_npx
	make_cli "$(cached_binary)" CACHED
	launch

	xberg_assert_status 0
	[[ "$output" == *"CACHED:ran"* ]]
	[[ "$output" != *"NPX:ran"* ]]
}

@test "mcp-launch should_use_npx_for_a_path_binary_older_than_the_pin" {
	VERSION_REPORTED="0.0.1"
	make_npx
	make_cli "$XBERG_STUB_BIN/$BINARY_NAME" ONPATH
	launch

	xberg_assert_status 0
	[[ "$output" == *"NPX:ran"* ]]
	[[ "$output" != *"ONPATH:ran"* ]]
}

@test "mcp-launch should_reach_npx_before_any_download_in_auto_mode" {
	make_npx
	xberg_stub_trace curl
	launch

	xberg_assert_status 0
	[[ "$output" == *"NPX:ran"* ]]
	xberg_assert_trace_empty
}

@test "mcp-launch should_run_npx_when_the_launcher_mode_is_npx" {
	LAUNCH_MODE="npx"
	make_npx
	launch serve

	xberg_assert_status 0
	[[ "$output" == *"NPX:ran"* ]]
}

@test "mcp-launch should_fail_in_npx_mode_without_node_and_not_fall_back_to_download" {
	LAUNCH_MODE="npx"
	xberg_stub_trace curl
	launch

	xberg_assert_status 1
	xberg_assert_output_contains "could not locate or install"
	xberg_assert_trace_empty
}

@test "mcp-launch should_fall_through_to_download_when_npx_is_missing" {
	xberg_stub node 'exit 0'
	xberg_stub_trace curl
	launch

	# curl is stubbed to succeed without writing the archive, so the download branch is entered
	# and then fails on extraction; reaching it at all is the claim. ~keep
	xberg_assert_status 1
	xberg_assert_output_contains "downloading"
}

# --- brew ---------------------------------------------------------------------------------------

@test "mcp-launch should_not_run_brew_in_auto_mode" {
	xberg_stub_trace brew
	launch

	xberg_assert_status 1
	xberg_assert_trace_empty
}

@test "mcp-launch should_install_through_brew_and_exec_what_it_put_on_path" {
	LAUNCH_MODE="brew"
	# A brew that actually delivers the binary, so the post-install PATH lookup has something to
	# find. Asserting only that `brew install` ran would pass against a script that never used
	# the result. ~keep
	xberg_stub brew \
		"cat >\"\$0.called\" <<<\"\$*\"" \
		"printf '%s\\n' '#!/usr/bin/env bash' 'if [ \"\$1\" = \"--version\" ]; then echo $PINNED_VERSION; exit 0; fi' 'for a in \"\$@\"; do printf \"BREWED[%s]\\\\n\" \"\$a\"; done' 'exit 0' >\"$XBERG_STUB_BIN/$BINARY_NAME\"" \
		"chmod +x \"$XBERG_STUB_BIN/$BINARY_NAME\"" \
		'exit 0'
	launch serve

	xberg_assert_status 0
	[[ "$output" == *"BREWED[serve]"* ]]
	xberg_assert_file "$XBERG_STUB_BIN/brew.called" "install xberg-io/tap/${BINARY_NAME}"
}

@test "mcp-launch should_fall_through_when_brew_install_fails" {
	LAUNCH_MODE="brew"
	xberg_stub_exit brew 1
	launch

	xberg_assert_status 1
	xberg_assert_output_contains "falling through"
}

# --- download -----------------------------------------------------------------------------------

@test "mcp-launch should_fall_through_when_the_platform_has_no_prebuilt_archive" {
	LAUNCH_MODE="download"
	xberg_stub uname \
		'case "$1" in' \
		'  -m) printf "sparc64\n" ;;' \
		'  *) printf "Plan9\n" ;;' \
		'esac'
	launch

	xberg_assert_status 1
	xberg_assert_output_contains "falling through"
}

@test "mcp-launch should_fall_through_when_no_downloader_is_available" {
	LAUNCH_MODE="download"
	# curl and wget are both missing from the mirrored system path, which is the state this
	# branch exists for.
	launch

	xberg_assert_status 1
	xberg_assert_output_contains "falling through"
}

# The download path stops here, deliberately.
#
# Fall-through is shared -- every launcher declines rather than proceeds when the platform has no
# published archive or no downloader exists. What happens AFTER a successful download is not
# shared, and the five have diverged along four independent axes: how the release is located (a
# `releases/latest/download` URL versus a GitHub API query for tag_name), how the asset is named,
# which architectures are published, and -- the one that matters -- what integrity means.
#
# Three postures exist today across five products shipping the same launcher:
#   * no checksum at all; warns that HTTPS is the only protection, then installs
#   * fetches a SHA256SUMS asset and REFUSES to execute a binary with no entry
#   * fetches it, and on a missing entry warns and installs anyway
#
# A single byte-identical test covering all of that would have to model each implementation, and
# the version that "passed everywhere" passed in two repos only because an earlier step failed
# first -- it would have gone on passing had those launchers installed unverified binaries
# silently. A test that cannot fail for the reason it names is worse than no test, so the
# divergence is written down here for people to settle rather than frozen into an assertion. ~keep

# --- plugin root resolution ---------------------------------------------------------------------

@test "mcp-launch should_resolve_the_plugin_root_from_its_own_location_when_the_variable_is_unset" {
	# Without CLAUDE_PLUGIN_ROOT the script derives the plugin directory from BASH_SOURCE, so the
	# cached binary it looks for is the one in the repo's own plugin/bin. Asserting the derived
	# path directly would require writing into the working tree; asserting that it does NOT use
	# the temp root is the same claim without the side effect. ~keep
	make_cli "$(cached_binary)" CACHED
	run env -i \
		PATH="$(xberg_isolated_path)" \
		HOME="$XBERG_WORK/home" \
		"${LAUNCHER_VAR}=auto" \
		"$SCRIPT"

	[[ "$output" != *"CACHED:ran"* ]]
}
