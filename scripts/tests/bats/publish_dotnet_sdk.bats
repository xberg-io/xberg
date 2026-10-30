#!/usr/bin/env bats

setup() {
	bats_load_library xberg-bats
	xberg_setup
	xberg_shadow_system_path_without dotnet
	REPOSITORY_ROOT="$(cd "$BATS_TEST_DIRNAME/../../.." && pwd -P)"
	cd "$XBERG_WORK" || return
	mkdir sdk-home
	unset CURL_STATUS SDK_INSTALL_STATUS SDK_INFO_STATUS
	cat >sdk-installer <<'INSTALLER'
#!/usr/bin/env bash
printf 'install %s\n' "$*" >>"$XBERG_TRACE"
[ "${SDK_INSTALL_STATUS:-0}" = 0 ] || exit "$SDK_INSTALL_STATUS"
while [ "$#" -gt 0 ]; do
  if [ "$1" = --install-dir ]; then install_dir="$2"; break; fi
  shift
done
mkdir -p "$install_dir"
cat >"$install_dir/dotnet" <<'SDK'
#!/usr/bin/env bash
printf 'dotnet %s\n' "$*" >>"$XBERG_TRACE"
exit "${SDK_INFO_STATUS:-0}"
SDK
chmod +x "$install_dir/dotnet"
INSTALLER
	xberg_stub curl \
		'printf "curl %s\n" "$*" >>"$XBERG_TRACE"' \
		'[ "${CURL_STATUS:-0}" = 0 ] || exit "$CURL_STATUS"' \
		'while [ "$#" -gt 0 ]; do if [ "$1" = -o ]; then cp "$XBERG_WORK/sdk-installer" "$2"; exit; fi; shift; done' \
		'exit 91'
}

run_dotnet_setup() {
	run env HOME="$XBERG_WORK/sdk-home" PATH="$(xberg_isolated_path)" \
		"$BASH" "$REPOSITORY_ROOT/scripts/publish/csharp/ensure-dotnet.sh"
}

@test "dotnet setup should use an existing SDK without downloading or exporting installation paths" {
	xberg_stub_trace dotnet
	run_dotnet_setup
	xberg_assert_status 0
	xberg_assert_trace 'dotnet --info'
	xberg_assert_github_path ''
	[ ! -e dotnet-install.sh ]
}

@test "dotnet setup should preserve an existing SDK inspection failure" {
	xberg_stub_exit dotnet 13
	run_dotnet_setup
	xberg_assert_status 13
	xberg_assert_trace_empty
	xberg_assert_github_path ''
}

@test "dotnet setup should install the pinned channel and immediately inspect the SDK from its isolated home" {
	run_dotnet_setup
	xberg_assert_status 0
	xberg_assert_trace "curl -sSL https://dot.net/v1/dotnet-install.sh -o dotnet-install.sh
install --channel 10.0 --quality ga --install-dir $XBERG_WORK/sdk-home/.dotnet
dotnet --info"
	xberg_assert_github_path "$XBERG_WORK/sdk-home/.dotnet
$XBERG_WORK/sdk-home/.dotnet/tools"
	[ -x sdk-home/.dotnet/dotnet ]
}

@test "dotnet setup should preserve download failures without running an installer or exporting paths" {
	export CURL_STATUS=7
	run_dotnet_setup
	xberg_assert_status 7
	xberg_assert_trace 'curl -sSL https://dot.net/v1/dotnet-install.sh -o dotnet-install.sh'
	xberg_assert_github_path ''
	[ ! -e dotnet-install.sh ]
}

@test "dotnet setup should preserve installation failures before exporting paths or inspecting an SDK" {
	export SDK_INSTALL_STATUS=11
	run_dotnet_setup
	xberg_assert_status 11
	xberg_assert_trace "curl -sSL https://dot.net/v1/dotnet-install.sh -o dotnet-install.sh
install --channel 10.0 --quality ga --install-dir $XBERG_WORK/sdk-home/.dotnet"
	xberg_assert_github_path ''
	[ ! -e sdk-home/.dotnet ]
}

@test "dotnet setup should propagate inspection errors from a newly installed SDK" {
	export SDK_INFO_STATUS=17
	run_dotnet_setup
	xberg_assert_status 17
	grep -Fxq 'dotnet --info' "$XBERG_TRACE"
	[ -x sdk-home/.dotnet/dotnet ]
}
