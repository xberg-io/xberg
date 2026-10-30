#!/usr/bin/env bats

setup() {
	bats_load_library xberg-bats
	xberg_setup
	REPOSITORY_ROOT="$(cd "$BATS_TEST_DIRNAME/../../.." && pwd -P)"
	cd "$XBERG_WORK"
	mkdir -p crates/xberg-node/npm crates/xberg-node/artifacts
}

write_platform() {
	local platform="$1" binary="$2"
	mkdir -p "crates/xberg-node/npm/$platform"
	printf '{"name":"@xberg-io/xberg-%s","version":"1.4.0","files":["%s"]}\n' \
		"$platform" "$binary" >"crates/xberg-node/npm/$platform/package.json"
	printf '%s\n' "$platform binary" >"crates/xberg-node/artifacts/$binary"
}

@test "Node packaging should archive each supported target with exact native and PDFium assets" {
	target_count=0
	while read -r target platform binary pdfium; do
		target_count=$((target_count + 1))
		write_platform "$platform" "$binary"
		printf '%s\n' "$platform pdfium" >"crates/xberg-node/$pdfium"
		run env TARGET="$target" bash "$REPOSITORY_ROOT/scripts/publish/node/package-artifacts.sh"
		xberg_assert_status 0
		mkdir unpacked
		tar -xzf "node-bindings-$target.tar.gz" -C unpacked
		xberg_assert_file "unpacked/$platform/$binary" "$platform binary"
		xberg_assert_file "unpacked/$platform/$pdfium" "$platform pdfium"
		cmp "$REPOSITORY_ROOT/LICENSE" "unpacked/$platform/LICENSE"
		cmp "$REPOSITORY_ROOT/THIRD_PARTY_LICENSES.md" "unpacked/$platform/THIRD_PARTY_LICENSES.md"
		[ "$(find unpacked -type f | wc -l | tr -d '[:space:]')" -eq 5 ]
		[ "$(jq -r --arg f "$pdfium" '[.files[] | select(. == $f)] | length' "unpacked/$platform/package.json")" -eq 1 ]
		rm -r unpacked
	done <<'TARGETS'
aarch64-apple-darwin darwin-arm64 xberg-node.darwin-arm64.node libpdfium.dylib
x86_64-apple-darwin darwin-x64 xberg-node.darwin-x64.node libpdfium.dylib
x86_64-pc-windows-msvc win32-x64-msvc xberg-node.win32-x64-msvc.node pdfium.dll
aarch64-pc-windows-msvc win32-arm64-msvc xberg-node.win32-arm64-msvc.node pdfium.dll
x86_64-unknown-linux-gnu linux-x64-gnu xberg-node.linux-x64-gnu.node libpdfium.so
aarch64-unknown-linux-gnu linux-arm64-gnu xberg-node.linux-arm64-gnu.node libpdfium.so
armv7-unknown-linux-gnueabihf linux-arm-gnueabihf xberg-node.linux-arm-gnueabihf.node libpdfium.so
TARGETS
	[ "$target_count" -eq 7 ]
}

@test "Node packaging should use the crate-local binary fallback when artifacts are absent" {
	write_platform linux-x64-gnu xberg-node.linux-x64-gnu.node
	mv crates/xberg-node/artifacts/xberg-node.linux-x64-gnu.node crates/xberg-node/
	run env TARGET=x86_64-unknown-linux-gnu bash "$REPOSITORY_ROOT/scripts/publish/node/package-artifacts.sh"
	xberg_assert_status 0
	xberg_assert_file crates/xberg-node/npm/linux-x64-gnu/xberg-node.linux-x64-gnu.node 'linux-x64-gnu binary'
	[ -s node-bindings-x86_64-unknown-linux-gnu.tar.gz ]
	xberg_assert_output_contains 'Warning: libpdfium.so not found'
}

@test "Node packaging should reject missing binaries and unsupported targets without producing an archive" {
	run env TARGET=x86_64-unknown-linux-gnu bash "$REPOSITORY_ROOT/scripts/publish/node/package-artifacts.sh"
	xberg_assert_status 1
	xberg_assert_output_contains 'Missing built NAPI binary: expected xberg-node.linux-x64-gnu.node'
	[ ! -e node-bindings-x86_64-unknown-linux-gnu.tar.gz ]
	run env TARGET=wasm32-unknown-unknown bash "$REPOSITORY_ROOT/scripts/publish/node/package-artifacts.sh"
	xberg_assert_status 1
	xberg_assert_output_contains 'Unsupported NAPI target: wasm32-unknown-unknown'
	[ ! -e node-bindings-wasm32-unknown-unknown.tar.gz ]
}

@test "Node prepublish should derive exact optional dependencies only from staged platform manifests" {
	printf '{"name":"@xberg-io/xberg","optionalDependencies":{"stale":"0.0.0"}}\n' >crates/xberg-node/package.json
	write_platform linux-x64-gnu xberg-node.linux-x64-gnu.node
	write_platform darwin-arm64 xberg-node.darwin-arm64.node
	run bash "$REPOSITORY_ROOT/scripts/publish/node/prepublish-main-package.sh"
	xberg_assert_status 0
	[ "$(jq -cS .optionalDependencies crates/xberg-node/package.json)" = \
		'{"@xberg-io/xberg-darwin-arm64":"1.4.0","@xberg-io/xberg-linux-x64-gnu":"1.4.0"}' ]
	[ "$(jq -r .name crates/xberg-node/package.json)" = '@xberg-io/xberg' ]
}

@test "Node prepublish should reject an empty platform set without modifying the main manifest" {
	printf '{"name":"@xberg-io/xberg"}\n' >crates/xberg-node/package.json
	run bash "$REPOSITORY_ROOT/scripts/publish/node/prepublish-main-package.sh"
	xberg_assert_status 1
	xberg_assert_output_contains 'No platform packages found'
	xberg_assert_file crates/xberg-node/package.json '{"name":"@xberg-io/xberg"}'
}

@test "Node prepublish should reject missing version metadata without modifying the main manifest" {
	printf '{"name":"@xberg-io/xberg"}\n' >crates/xberg-node/package.json
	mkdir -p crates/xberg-node/npm/broken
	printf '{"name":"@xberg-io/xberg-broken"}\n' >crates/xberg-node/npm/broken/package.json
	run bash "$REPOSITORY_ROOT/scripts/publish/node/prepublish-main-package.sh"
	xberg_assert_status 1
	xberg_assert_output_contains 'Invalid platform package.json: crates/xberg-node/npm/broken/package.json'
	xberg_assert_file crates/xberg-node/package.json '{"name":"@xberg-io/xberg"}'
}
