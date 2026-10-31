#!/usr/bin/env bats

setup() {
	[ -z "${XBERG_EXPECTED_BASH_MAJOR_MINOR:-}" ] || [[ "$BASH_VERSION" == "$XBERG_EXPECTED_BASH_MAJOR_MINOR".* ]]
	bats_load_library xberg-bats
	xberg_setup
	REPOSITORY_ROOT="$(cd "$BATS_TEST_DIRNAME/../../.." && pwd -P)"
	export REPO_ROOT="$XBERG_WORK/repository"
	mkdir -p "$REPO_ROOT/scripts/lib" "$REPO_ROOT/scripts/ci/wasm" "$REPO_ROOT/packages/ruby/lib" \
		"$REPO_ROOT/target/wheels" "$REPO_ROOT/target/release" "$REPO_ROOT/target/tessdata" \
		"$REPO_ROOT/e2e/smoke/node" "$REPO_ROOT/crates/xberg-node" "$XBERG_WORK/tmp"
	cp "$REPOSITORY_ROOT/scripts/lib/common.sh" "$REPOSITORY_ROOT/scripts/lib/library-paths.sh" \
		"$REPOSITORY_ROOT/scripts/lib/tessdata.sh" "$REPO_ROOT/scripts/lib/"
	cp "$REPOSITORY_ROOT/scripts/ci/wasm/run-crate-tests.sh" "$REPOSITORY_ROOT/scripts/ci/wasm/install-wasm-pack.sh" "$REPO_ROOT/scripts/ci/wasm/"
	printf '[workspace]\n' >"$REPO_ROOT/Cargo.toml"
	printf '{}\n' >"$REPO_ROOT/e2e/smoke/node/package.json"
	printf '{}\n' >"$REPO_ROOT/crates/xberg-node/package.json"
	printf 'tarball bytes\n' >"$REPO_ROOT/node.tgz"
	dd if=/dev/zero of="$REPO_ROOT/target/tessdata/eng.traineddata" bs=1000000 count=1 2>/dev/null
	dd if=/dev/zero of="$REPO_ROOT/target/tessdata/osd.traineddata" bs=100000 count=1 2>/dev/null
	export RUNNER_OS=fixture GITHUB_WORKSPACE="$REPO_ROOT" TMPDIR="$XBERG_WORK/tmp"
	unset BUNDLE_PATH BUNDLE_GEMFILE PIP_STATUS PYTHON_STATUS BUNDLE_STATUS VENDOR_STATUS WASM_STATUS CC_STATUS POSIX_STATUS \
		RUST_MIN_STACK CARGO_FAILURE_MATCH NODE_PATH RUSTFLAGS CARGO_ENCODED_RUSTFLAGS XBERG_PDFIUM_PREBUILT
	xberg_stub_curl_offline
	xberg_stub pip 'printf "pip %s\n" "$*" >>"$XBERG_TRACE"' 'exit "${PIP_STATUS:-0}"'
	xberg_stub python 'printf "python %s\n" "$*" >>"$XBERG_TRACE"' 'exit "${PYTHON_STATUS:-0}"'
	xberg_stub python3 'printf "vendor %s\n" "$*" >>"$XBERG_TRACE"' 'exit "${VENDOR_STATUS:-0}"'
	xberg_stub ruby 'printf "ruby fixture\n"'
	xberg_stub rustc 'printf "rustc fixture\n"'
	xberg_stub cargo \
		'printf "cargo %s\n" "$*" >>"$XBERG_TRACE"' \
		'if [ "$1" = metadata ]; then cat "$METADATA_FIXTURE"; exit 0; fi' \
		'case "$*" in *"${CARGO_FAILURE_MATCH:-__never__}"*) exit 17 ;; esac' \
		'if [ "$1" = test ]; then printf "live=%s stack=%s backtrace=%s\n" "${XBERG_SKIP_LIVE_HF:-}" "${RUST_MIN_STACK:-}" "${RUST_BACKTRACE:-}" >>"$XBERG_WORK/cargo-env"; fi' \
		'printf "test result: ok. 1 passed; 0 failed; 0 ignored;\n"'
	xberg_stub bundle \
		'printf "bundle %s\n" "$*" >>"$XBERG_TRACE"' \
		'if [ "$1" = exec ]; then printf "jobs=%s backtrace=%s verbose=%s ld=%s\n" "$CARGO_BUILD_JOBS" "$RUST_BACKTRACE" "$RB_SYS_VERBOSE" "${LD_LIBRARY_PATH:-}" >>"$XBERG_TRACE"; fi' \
		'exit "${BUNDLE_STATUS:-0}"'
	xberg_stub gem 'printf "gem fixture\n"'
	xberg_stub mise 'printf "mise %s\n" "$*" >>"$XBERG_TRACE"'
	xberg_stub uname 'case "$1" in -s) printf "%s\n" "${TEST_OS:-Darwin}" ;; -m) printf "%s\n" "${TEST_ARCH:-arm64}" ;; esac'
	cd "$REPO_ROOT" || return
}

refute_trace_pattern() {
	[ -f "$XBERG_TRACE" ]
	[ "$(grep -Ec -- "$1" "$XBERG_TRACE" || true)" -eq 0 ]
}

@test "Python wheel CI smoke should install exclusively from local wheels and execute version verification" {
	run "$BASH" "$REPOSITORY_ROOT/scripts/ci/python/smoke-test-wheel.sh"
	xberg_assert_status 0
	xberg_assert_trace "pip install --no-index --find-links $REPO_ROOT/target/wheels xberg
python $REPO_ROOT/scripts/python/print_xberg_version.py"
	xberg_assert_output_contains 'Smoke test passed!'
}

@test "Python wheel CI smoke should stop when local wheel installation fails" {
	export PIP_STATUS=13
	run "$BASH" "$REPOSITORY_ROOT/scripts/ci/python/smoke-test-wheel.sh"
	xberg_assert_status 13
	xberg_assert_trace "pip install --no-index --find-links $REPO_ROOT/target/wheels xberg"
	[[ "$output" != *'Smoke test passed!'* ]]
}

@test "Python wheel CI smoke should propagate a failed runtime verification" {
	export PYTHON_STATUS=14
	run "$BASH" "$REPOSITORY_ROOT/scripts/ci/python/smoke-test-wheel.sh"
	xberg_assert_status 14
	[ "$(wc -l <"$XBERG_TRACE" | tr -d '[:space:]')" -eq 2 ]
	[[ "$output" != *'Smoke test passed!'* ]]
}

@test "language CI scripts should reject a repository without its manifest before invoking tools" {
	rm "$REPO_ROOT/Cargo.toml"
	for script in python/smoke-test-wheel.sh ruby/install-ruby-deps.sh ruby/compile-extension.sh rust/run-unit-tests.sh; do
		run "$BASH" "$REPOSITORY_ROOT/scripts/ci/$script"
		xberg_assert_status 1
		xberg_assert_output_contains 'REPO_ROOT validation failed'
		xberg_assert_trace_empty
	done
}

@test "Ruby dependency CI should publish default bundle paths and execute the complete install sequence" {
	run "$BASH" "$REPOSITORY_ROOT/scripts/ci/ruby/install-ruby-deps.sh"
	xberg_assert_status 0
	xberg_assert_trace "bundle config set deployment false
bundle config set path $REPO_ROOT/packages/ruby/.bundle/bundle
bundle install --jobs 4"
	xberg_assert_github_env "BUNDLE_GEMFILE=$REPO_ROOT/packages/ruby/Gemfile
BUNDLE_PATH=$REPO_ROOT/packages/ruby/.bundle/bundle"
}

@test "Ruby dependency CI should respect explicit bundle paths without overwriting workflow environment" {
	export BUNDLE_PATH="$XBERG_WORK/custom bundle" BUNDLE_GEMFILE="$XBERG_WORK/custom Gemfile"
	run "$BASH" "$REPOSITORY_ROOT/scripts/ci/ruby/install-ruby-deps.sh"
	xberg_assert_status 0
	xberg_assert_trace "bundle config set deployment false
bundle config set path $BUNDLE_PATH
bundle install --jobs 4"
	xberg_assert_github_env ''
}

@test "Ruby dependency CI should stop immediately on bundle configuration failure" {
	export BUNDLE_STATUS=12
	run "$BASH" "$REPOSITORY_ROOT/scripts/ci/ruby/install-ruby-deps.sh"
	xberg_assert_status 12
	xberg_assert_trace 'bundle config set deployment false'
	[[ "$output" != *'Ruby dependencies installed'* ]]
}

@test "Ruby native compilation should vendor first and preserve the single-job diagnostic environment" {
	export RUNNER_OS=Linux
	run "$BASH" "$REPOSITORY_ROOT/scripts/ci/ruby/compile-extension.sh"
	xberg_assert_status 0
	grep -Fxq "vendor $REPO_ROOT/scripts/ci/ruby/vendor-xberg-core.py" "$XBERG_TRACE"
	grep -Fxq 'bundle exec rake compile --verbose --trace' "$XBERG_TRACE"
	grep -Fxq "jobs=1 backtrace=1 verbose=1 ld=$REPO_ROOT/target/release:" "$XBERG_TRACE"
	[ "$(grep -c '^vendor ' "$XBERG_TRACE")" -eq 1 ]
	[ "$(grep -c '^bundle ' "$XBERG_TRACE")" -eq 1 ]
}

@test "Ruby native compilation should reject a failed vendor operation before rake" {
	export VENDOR_STATUS=15
	run "$BASH" "$REPOSITORY_ROOT/scripts/ci/ruby/compile-extension.sh"
	xberg_assert_status 15
	[ "$(grep -c '^vendor ' "$XBERG_TRACE")" -eq 1 ]
	refute_trace_pattern '^bundle '
}

@test "Ruby native compilation should reject a failed rake build despite an old extension" {
	export BUNDLE_STATUS=16
	printf 'stale extension\n' >"$REPO_ROOT/packages/ruby/lib/xberg.so"
	run "$BASH" "$REPOSITORY_ROOT/scripts/ci/ruby/compile-extension.sh"
	xberg_assert_status 1
	xberg_assert_output_contains 'ERROR: rake compile failed'
	[[ "$output" != *'=== Compilation complete ==='* ]]
}

prepare_node_tools() {
	xberg_capture_real mktemp
	xberg_stub mktemp '[ "$*" = -d ] || exit 94' 'exec "$XBERG_REAL_MKTEMP" -d "$XBERG_WORK/tmp/node.XXXXXX"'
	xberg_stub node 'printf "node %s\n" "$1" >>"$XBERG_TRACE"' '[ "$1" = -e ] || exit 93'
	xberg_stub pnpm 'printf "pnpm %s ld=%s\n" "$*" "${LD_LIBRARY_PATH:-}" >>"$XBERG_TRACE"' \
		'case "$*" in "${PNPM_FAILURE_MATCH:-__never__}") exit 19 ;; esac'
}

@test "Node package smoke should build TypeScript before installing and checking the smoke app" {
	prepare_node_tools
	export RUNNER_OS=Linux XBERG_PDFIUM_PREBUILT="$XBERG_WORK/pdfium"
	run "$BASH" "$REPOSITORY_ROOT/scripts/ci/actions/smoke-node/run.sh" node.tgz
	xberg_assert_status 0
	xberg_assert_trace "node -e
pnpm install --no-frozen-lockfile ld=$XBERG_WORK/pdfium/lib:
pnpm build:ts ld=$XBERG_WORK/pdfium/lib:
pnpm install --no-frozen-lockfile ld=$XBERG_WORK/pdfium/lib:
pnpm run check ld=$XBERG_WORK/pdfium/lib:"
	[ "$(find "$TMPDIR" -name xberg-node.tgz -type f | wc -l | tr -d '[:space:]')" -eq 1 ]
	xberg_assert_file "$(find "$TMPDIR" -name xberg-node.tgz -type f)" 'tarball bytes'
}

@test "Node package smoke should find an archive directory and reject an empty directory before tools" {
	prepare_node_tools
	mkdir archives
	run "$BASH" "$REPOSITORY_ROOT/scripts/ci/actions/smoke-node/run.sh" archives
	xberg_assert_status 1
	xberg_assert_output_contains 'No .tgz file found in directory'
	xberg_assert_trace_empty
	cp node.tgz archives/package.tgz
	run "$BASH" "$REPOSITORY_ROOT/scripts/ci/actions/smoke-node/run.sh" "$REPO_ROOT/archives"
	xberg_assert_status 0
	[ "$(grep -c '^pnpm ' "$XBERG_TRACE")" -eq 4 ]
}

@test "Node package smoke should stop after a failed TypeScript build before smoke installation" {
	prepare_node_tools
	export PNPM_FAILURE_MATCH=build:ts
	run "$BASH" "$REPOSITORY_ROOT/scripts/ci/actions/smoke-node/run.sh" node.tgz
	xberg_assert_status 19
	[ "$(grep -c '^pnpm ' "$XBERG_TRACE")" -eq 2 ]
	[[ "$output" != *'Node package smoke test passed'* ]]
}

@test "Node package smoke should propagate a failed smoke check instead of reporting success" {
	prepare_node_tools
	export PNPM_FAILURE_MATCH='run check'
	run "$BASH" "$REPOSITORY_ROOT/scripts/ci/actions/smoke-node/run.sh" node.tgz
	xberg_assert_status 19
	[ "$(grep -c '^pnpm ' "$XBERG_TRACE")" -eq 4 ]
	[[ "$output" != *'Node package smoke test passed'* ]]
}

@test "Rust unit runner should execute every curated macOS leg and preserve an explicit stack budget" {
	export TEST_OS=Darwin TEST_ARCH=arm64 RUST_MIN_STACK=33554432
	run "$BASH" "$REPOSITORY_ROOT/scripts/ci/rust/run-unit-tests.sh"
	xberg_assert_status 0
	[ "$(grep -c '^cargo test ' "$XBERG_TRACE")" -eq 5 ]
	grep -Fxq 'cargo clean -p xberg' "$XBERG_TRACE"
	grep -Fxq 'mise run rust:clean' "$XBERG_TRACE"
	[ "$(grep -c '^live=1 stack=33554432 backtrace=full$' "$XBERG_WORK/cargo-env")" -eq 5 ]
	grep -Fq -- '--features full,formula-recognition,bundle-tessdata-eng --all-targets --verbose' "$XBERG_TRACE"
	grep -Fq -- '-p xberg-candle-ocr --features trocr,paddleocr-vl,glm-ocr,deepseek-ocr' "$XBERG_TRACE"
	grep -Fq -- '-p xberg-gliner --features candle,ort-dynamic' "$XBERG_TRACE"
	grep -Fq -- '-p xberg-libheif --all-targets' "$XBERG_TRACE"
}

@test "Rust unit runner should avoid unsupported candle features on Linux aarch64" {
	export TEST_OS=Linux TEST_ARCH=aarch64
	run "$BASH" "$REPOSITORY_ROOT/scripts/ci/rust/run-unit-tests.sh"
	xberg_assert_status 0
	[ "$(grep -c '^cargo test ' "$XBERG_TRACE")" -eq 4 ]
	grep -Fq -- '--features full-no-heic,heic,formula-recognition,bundle-tessdata-eng' "$XBERG_TRACE"
	grep -Fq -- '--exclude xberg-ffi --exclude xberg-php --exclude xberg-dart --exclude xberg-swift' "$XBERG_TRACE"
	grep -Fq -- '-p xberg-gliner --features ort-dynamic' "$XBERG_TRACE"
	refute_trace_pattern '^mise '
	refute_trace_pattern '-p xberg-candle-ocr'
}

@test "Rust unit runner should reject a failed first leg before cleaning or starting the workspace leg" {
	export CARGO_FAILURE_MATCH='-p xberg --features'
	run "$BASH" "$REPOSITORY_ROOT/scripts/ci/rust/run-unit-tests.sh"
	xberg_assert_status 1
	xberg_assert_output_contains '=== Test execution failed ==='
	[ "$(grep -c '^cargo test ' "$XBERG_TRACE")" -eq 1 ]
	refute_trace_pattern '^cargo clean '
	[[ "$output" != *'=== Tests complete ==='* ]]
}

prepare_chunk_metadata() {
	export METADATA_FIXTURE="$XBERG_WORK/metadata.json"
	printf '{"target_directory":"%s/target","packages":[{"name":"xberg","targets":%s}]}\n' \
		"$REPO_ROOT" "$1" >"$METADATA_FIXTURE"
	export TEST_OS=Linux TEST_ARCH=x86_64 XBERG_TEST_CHUNK_SIZE=1
}

@test "Rust chunked runner should reject zero collected integration targets before executing tests" {
	prepare_chunk_metadata '[]'
	run "$BASH" "$REPOSITORY_ROOT/scripts/ci/rust/run-unit-tests.sh"
	xberg_assert_status 1
	xberg_assert_output_contains 'no integration-test targets found for xberg'
	grep -Fxq 'cargo metadata --no-deps --format-version 1' "$XBERG_TRACE"
	refute_trace_pattern '^cargo test '
}

@test "Rust chunked runner should execute both collected targets and prune only their executables" {
	prepare_chunk_metadata '[{"name":"first-case","kind":["test"]},{"name":"second_case","kind":["test"]}]'
	mkdir -p "$REPO_ROOT/target/debug/deps"
	for file in first_case-abc123 second_case-def456 first_case-abc123.rlib unrelated-abc123; do
		printf 'artifact bytes\n' >"$REPO_ROOT/target/debug/deps/$file"
	done
	run "$BASH" "$REPOSITORY_ROOT/scripts/ci/rust/run-unit-tests.sh"
	xberg_assert_status 0
	xberg_assert_output_contains 'integration-test targets: 2 (chunks of 1)'
	[ "$(grep -c '^cargo test ' "$XBERG_TRACE")" -eq 7 ]
	[ "$(grep -c -- '--test first-case --verbose' "$XBERG_TRACE")" -eq 1 ]
	[ "$(grep -c -- '--test second_case --verbose' "$XBERG_TRACE")" -eq 1 ]
	[ ! -e "$REPO_ROOT/target/debug/deps/first_case-abc123" ]
	[ ! -e "$REPO_ROOT/target/debug/deps/second_case-def456" ]
	xberg_assert_file "$REPO_ROOT/target/debug/deps/first_case-abc123.rlib" 'artifact bytes'
	xberg_assert_file "$REPO_ROOT/target/debug/deps/unrelated-abc123" 'artifact bytes'
}
