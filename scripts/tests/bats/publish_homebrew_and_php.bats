#!/usr/bin/env bats

setup() {
	bats_load_library xberg-bats
	xberg_setup
	REPOSITORY_ROOT="$(cd "$BATS_TEST_DIRNAME/../../.." && pwd -P)"
	cd "$XBERG_WORK" || return
	export TAP_DIR="$XBERG_WORK/tap" TAG=v1.4.0 VERSION=1.4.0 GITHUB_WORKSPACE="$XBERG_WORK"
	export CHECKOUT_SHA=1111111111111111111111111111111111111111
	unset DRY_RUN MOVED_TAG CORRUPT_ASSET UPLOAD_STATUS
	mkdir -p tap/Formula source packages/php target/release output
	printf 'source bytes\n' >source/README
	tar -czf source.tar.gz -C source README
	printf 'class Xberg < Formula\n  url "old-url"\n  sha256 "old-sha"\n  depends_on "rust" => :build\n  bottle do\n    root_url "old-bottle"\n  end\nend\n' >tap/Formula/xberg.rb
	cp tap/Formula/xberg.rb original.rb
	printf '{"name":"xberg/xberg"}\n' >packages/php/composer.json
	for name in package.xml README.md LICENSE CHANGELOG.md; do printf '%s fixture\n' "$name" >"packages/php/$name"; done
	xberg_stub git 'printf "git %s\n" "$*" >>"$XBERG_TRACE"; printf "%s\n" "$CHECKOUT_SHA"'
	xberg_stub curl 'printf "curl %s\n" "$*" >>"$XBERG_TRACE"' 'while [ "$1" != -o ]; do shift; done; cp "$XBERG_WORK/source.tar.gz" "$2"'
	xberg_stub gh \
		'printf "gh %s\n" "$*" >>"$XBERG_TRACE"' \
		'case "$1 $2" in' \
		'api*) if [ -f "$XBERG_WORK/api-seen" ] && [ "${MOVED_TAG:-false}" = true ]; then printf "2222222222222222222222222222222222222222\n"; else printf "1111111111111111111111111111111111111111\n"; fi; touch "$XBERG_WORK/api-seen" ;;' \
		'"release upload") [ "${UPLOAD_STATUS:-0}" = 0 ] || exit "$UPLOAD_STATUS"; cp "$4" "$XBERG_WORK/published.tar.gz" ;;' \
		'"release download") while [ "$1" != --dir ]; do shift; done; cp "$XBERG_WORK/published.tar.gz" "$2/xberg-$VERSION.tar.gz"; if [ "${CORRUPT_ASSET:-false}" = true ]; then printf corrupt >"$2/xberg-$VERSION.tar.gz"; fi ;;' \
		'*) exit 97 ;; esac'
	xberg_stub date 'printf "2026-10-29T00:00:00Z\n"'
}

@test "Homebrew update should pin verified published bytes and replace stale bottle metadata" {
	run "$BASH" "$REPOSITORY_ROOT/scripts/publish/update-homebrew-formula.sh"
	xberg_assert_status 0
	python3 - <<'PY'
from pathlib import Path
import hashlib
text=Path('tap/Formula/xberg.rb').read_text()
assert 'url "https://github.com/xberg-io/xberg/releases/download/v1.4.0/xberg-1.4.0.tar.gz"' in text
assert f'sha256 "{hashlib.sha256(Path("source.tar.gz").read_bytes()).hexdigest()}"' in text
assert text.count('depends_on "libheif"')==1
assert 'bottle do' not in text and 'old-bottle' not in text
PY
	[ "$(grep -c '^gh api ' "$XBERG_TRACE")" -eq 2 ]
	[ "$(grep -c '^gh release upload ' "$XBERG_TRACE")" -eq 1 ]
	[ "$(grep -c '^gh release download ' "$XBERG_TRACE")" -eq 1 ]
}

@test "Homebrew dry run should leave formula bytes unchanged without external commands" {
	run env DRY_RUN=true "$BASH" "$REPOSITORY_ROOT/scripts/publish/update-homebrew-formula.sh"
	xberg_assert_status 0
	cmp original.rb tap/Formula/xberg.rb
	xberg_assert_trace_empty
}

@test "Homebrew update should reject a published asset whose bytes differ from the upload" {
	export CORRUPT_ASSET=true
	run "$BASH" "$REPOSITORY_ROOT/scripts/publish/update-homebrew-formula.sh"
	xberg_assert_status 1
	xberg_assert_output_contains 'differs from uploaded'
	cmp original.rb tap/Formula/xberg.rb
}

@test "Homebrew update should reject tag movement before writing the formula" {
	export MOVED_TAG=true
	run "$BASH" "$REPOSITORY_ROOT/scripts/publish/update-homebrew-formula.sh"
	xberg_assert_status 1
	xberg_assert_output_contains 'moved during the formula update'
	cmp original.rb tap/Formula/xberg.rb
}

@test "Homebrew update should reject a checkout that differs from the resolved release tag" {
	export CHECKOUT_SHA=2222222222222222222222222222222222222222
	run "$BASH" "$REPOSITORY_ROOT/scripts/publish/update-homebrew-formula.sh"
	xberg_assert_status 1
	xberg_assert_output_contains 'but this run checked out'
	cmp original.rb tap/Formula/xberg.rb
	[ "$(wc -l <"$XBERG_TRACE" | tr -d '[:space:]')" -eq 2 ]
}

@test "Homebrew update should propagate failed uploads without changing the formula" {
	export UPLOAD_STATUS=9
	run "$BASH" "$REPOSITORY_ROOT/scripts/publish/update-homebrew-formula.sh"
	xberg_assert_status 9
	cmp original.rb tap/Formula/xberg.rb
}

@test "PHP PIE packaging should archive exact Linux extension bytes and matching metadata and checksum" {
	printf 'Linux extension bytes\n' >target/release/libxberg_php.so
	run "$BASH" "$REPOSITORY_ROOT/scripts/publish/php/build-pie-package.sh" linux-x86_64 "$XBERG_WORK/output"
	xberg_assert_status 0
	mkdir extracted
	tar -xzf output/xberg-1.4.0-linux-x86_64.tar.gz -C extracted
	xberg_assert_file extracted/xberg-1.4.0-linux-x86_64/ext/libxberg_php.so 'Linux extension bytes'
	cmp "$REPOSITORY_ROOT/LICENSE" extracted/xberg-1.4.0-linux-x86_64/LICENSE
	cmp "$REPOSITORY_ROOT/THIRD_PARTY_LICENSES.md" extracted/xberg-1.4.0-linux-x86_64/THIRD_PARTY_LICENSES.md
	[ "$(find extracted -type f | wc -l | tr -d '[:space:]')" -eq 9 ]
	python3 - <<'PY'
from pathlib import Path
import hashlib,json
metadata=json.loads(Path('extracted/xberg-1.4.0-linux-x86_64/pie.json').read_text())
assert metadata==dict(name='xberg',version='1.4.0',platform='linux-x86_64',os='linux',arch='x86_64',php_version='>=8.2',extension_file='ext/libxberg_php.so',built_at='2026-10-29T00:00:00Z')
archive=Path('output/xberg-1.4.0-linux-x86_64.tar.gz')
assert archive.with_suffix('.gz.sha256').read_text().split()[0]==hashlib.sha256(archive.read_bytes()).hexdigest()
PY
	[ ! -e output/xberg-1.4.0-linux-x86_64 ]
}

@test "PHP PIE packaging should reject another operating system's extension instead of mislabeling it" {
	printf 'Windows DLL bytes\n' >target/release/xberg_php.dll
	run "$BASH" "$REPOSITORY_ROOT/scripts/publish/php/build-pie-package.sh" linux-x86_64 "$XBERG_WORK/output"
	xberg_assert_status 1
	xberg_assert_output_contains 'Extension file not found'
	[ -z "$(find output -type f -print -quit)" ]
}

@test "PHP PIE packaging should reject unsupported platform labels" {
	run "$BASH" "$REPOSITORY_ROOT/scripts/publish/php/build-pie-package.sh" unknown "$XBERG_WORK/output"
	xberg_assert_status 1
	xberg_assert_output_contains 'Unknown platform: unknown'
}

@test "PHP PIE packaging should reject an empty target directory with a useful missing-extension error" {
	run "$BASH" "$REPOSITORY_ROOT/scripts/publish/php/build-pie-package.sh" linux-x86_64 "$XBERG_WORK/output"
	xberg_assert_status 1
	xberg_assert_output_contains 'Extension file not found'
}
