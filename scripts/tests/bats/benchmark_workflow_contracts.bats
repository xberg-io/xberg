#!/usr/bin/env bats

setup() {
	[ -z "${XBERG_EXPECTED_BASH_MAJOR_MINOR:-}" ] || [[ "$BASH_VERSION" == "$XBERG_EXPECTED_BASH_MAJOR_MINOR".* ]]
	bats_load_library xberg-bats
	xberg_setup
	REPOSITORY_ROOT="$(cd "$BATS_TEST_DIRNAME/../../.." && pwd -P)"
	cp "$REPOSITORY_ROOT/.github/workflows/benchmarks.yaml" "$XBERG_WORK/workflow.yaml"
	cp "$REPOSITORY_ROOT/.github/actions/cache-benchmark-harness/action.yml" "$XBERG_WORK/cache-action.yml"
	cd "$XBERG_WORK" || return
}

rewrite_fixture() {
	sed "$1" "$2" >"$2.tmp"
	mv "$2.tmp" "$2"
}

run_validation() {
	run "$BASH" "$REPOSITORY_ROOT/scripts/ci/benchmarks/validate-workflow-build-config.sh" workflow.yaml cache-action.yml
}

@test "benchmark workflow guard should validate the actual complete workflow and cache action" {
	run_validation
	xberg_assert_status 0
	xberg_assert_output 'benchmark workflow build configuration is valid'
}

@test "benchmark workflow guard should reject the legacy mutable branch override" {
	rewrite_fixture 's/ref: \${{ github\.sha }}/ref: ${{ github.event.inputs.branch || github.sha }}/' workflow.yaml
	[ "$(grep -Fc '          ref: ${{ github.event.inputs.branch || github.sha }}' workflow.yaml)" -eq 14 ]
	run_validation
	xberg_assert_status 1
	xberg_assert_output_contains 'standalone harness validation must check out the immutable workflow revision'
}

@test "benchmark workflow guard should reject an empty workflow rather than accepting an empty job sweep" {
	: >workflow.yaml
	run_validation
	xberg_assert_status 1
	xberg_assert_output_contains 'setup Rust must disable per-object sccache uploads'
}

@test "benchmark workflow guard should reject sccache upload changes in the measured setup" {
	rewrite_fixture 's/use-sccache: "false"/use-sccache: "true"/' workflow.yaml
	run_validation
	xberg_assert_status 1
	xberg_assert_output_contains 'setup Rust must disable per-object sccache uploads'
}

@test "benchmark workflow guard should require the exact setup CLI feature set" {
	rewrite_fixture 's/all,xberg\/sceptre-ocr-tract/all/' workflow.yaml
	run_validation
	xberg_assert_status 1
	xberg_assert_output_contains 'setup must build exactly one all-feature CLI'
}

@test "benchmark workflow guard should reject a missing independent harness validation job" {
	rewrite_fixture 's/^  validate-harness:/  unrelated-job:/' workflow.yaml
	run_validation
	xberg_assert_status 1
	xberg_assert_output_contains 'standalone job'
}

@test "benchmark workflow guard should reject release-profile contract validation" {
	rewrite_fixture '/cargo test --locked -p benchmark-harness --no-fail-fast/s/$/ --release/' workflow.yaml
	run_validation
	xberg_assert_status 1
	xberg_assert_output_contains 'non-release test profile'
}

@test "benchmark workflow guard should reject a missing artifact contract target" {
	rewrite_fixture 's/--test lossless_aggregation/--test unrelated_target/' workflow.yaml
	run_validation
	xberg_assert_status 1
	xberg_assert_output_contains 'missing target: lossless_aggregation'
}

@test "benchmark workflow guard should reject a cache key that omits compiler inputs" {
	rewrite_fixture 's/rustc -vV/rustc --version/' cache-action.yml
	run_validation
	xberg_assert_status 1
	xberg_assert_output_contains 'cache key omits build input rustc -vV'
}

@test "benchmark workflow guard should reject a logged build command with no actual harness invocation" {
	rewrite_fixture 's/if ! cargo build/echo cargo build/' cache-action.yml
	run_validation
	xberg_assert_status 1
	xberg_assert_output_contains 'harness cache must build only the uploaded benchmark-harness binary'
}

@test "benchmark workflow guard should reject an additional unuploaded binary in the actual build" {
	rewrite_fixture '/if ! cargo build/s/; then/ --bin unrelated; then/' cache-action.yml
	run_validation
	xberg_assert_status 1
	xberg_assert_output_contains 'harness cache must build only the uploaded benchmark-harness binary'
}

@test "benchmark workflow guard should reject a second actual Cargo build command" {
	printf '        cargo build --release -p unrelated\n' >>cache-action.yml
	run_validation
	xberg_assert_status 1
	xberg_assert_output_contains 'harness cache must build only the uploaded benchmark-harness binary'
}

@test "benchmark workflow guard should reject an extra inline composite action build step" {
	cat >>cache-action.yml <<'STEP'
    - name: Extra build
      shell: bash
      run: cargo build --release -p unrelated
STEP
	run_validation
	xberg_assert_status 1
	xberg_assert_output_contains 'harness cache must build only the uploaded benchmark-harness binary'
}

@test "benchmark workflow guard should reject an environment-prefixed inline build step" {
	cat >>cache-action.yml <<'STEP'
    - name: Extra build
      shell: bash
      run: CARGO_BUILD_JOBS=2 cargo build --release -p unrelated
STEP
	run_validation
	xberg_assert_status 1
	xberg_assert_output_contains 'harness cache must build only the uploaded benchmark-harness binary'
}

@test "benchmark workflow guard should reject an environment-prefixed block build step" {
	cat >>cache-action.yml <<'STEP'
    - name: Extra build
      shell: bash
      run: |
        CARGO_BUILD_JOBS=2 cargo build --release -p unrelated
STEP
	run_validation
	xberg_assert_status 1
	xberg_assert_output_contains 'harness cache must build only the uploaded benchmark-harness binary'
}

@test "benchmark workflow guard should ignore Cargo command text printed by another action step" {
	cat >>cache-action.yml <<'STEP'
    - name: Log build example
      shell: bash
      run: echo "cargo build --release -p unrelated"
STEP
	run_validation
	xberg_assert_status 0
	xberg_assert_output 'benchmark workflow build configuration is valid'
}

@test "benchmark workflow guard should reject a build launched through env" {
	cat >>cache-action.yml <<'STEP'
    - name: Extra build
      shell: bash
      run: env CARGO_BUILD_JOBS=2 cargo build --release -p unrelated
STEP
	run_validation
	xberg_assert_status 1
	xberg_assert_output_contains 'harness cache must build only the uploaded benchmark-harness binary'
}

@test "benchmark workflow guard should reject an actual build after a shell command boundary" {
	cat >>cache-action.yml <<'STEP'
    - name: Extra build
      shell: bash
      run: echo ready && cargo build --release -p unrelated
STEP
	run_validation
	xberg_assert_status 1
	xberg_assert_output_contains 'harness cache must build only the uploaded benchmark-harness binary'
}

@test "benchmark workflow guard should reject a quoted inline YAML build command" {
	cat >>cache-action.yml <<'STEP'
    - name: Extra build
      shell: bash
      run: "cargo build --release -p unrelated" # real inline command
STEP
	run_validation
	xberg_assert_status 1
	xberg_assert_output_contains 'harness cache must build only the uploaded benchmark-harness binary'
}

@test "benchmark workflow guard should reject a continued block build command" {
	cat >>cache-action.yml <<'STEP'
    - name: Extra build
      shell: bash
      run: |
        cargo \
          build --release -p unrelated
STEP
	run_validation
	xberg_assert_status 1
	xberg_assert_output_contains 'harness cache must build only the uploaded benchmark-harness binary'
}

@test "benchmark workflow guard should reject a folded YAML build command" {
	cat >>cache-action.yml <<'STEP'
    - name: Extra build
      shell: bash
      run: >
        cargo
        build --release -p unrelated
STEP
	run_validation
	xberg_assert_status 1
	xberg_assert_output_contains 'harness cache must build only the uploaded benchmark-harness binary'
}

@test "benchmark workflow guard should ignore separately logged Cargo command words" {
	cat >>cache-action.yml <<'STEP'
    - name: Log build example
      shell: bash
      run: echo cargo build --release -p unrelated
STEP
	run_validation
	xberg_assert_status 0
	xberg_assert_output 'benchmark workflow build configuration is valid'
}

@test "benchmark workflow guard should reject a build after an equals-form env unset option" {
	cat >>cache-action.yml <<'STEP'
    - name: Extra build
      shell: bash
      run: env --unset=RUSTFLAGS cargo build --release -p unrelated
STEP
	run_validation
	xberg_assert_status 1
	xberg_assert_output_contains 'harness cache must build only the uploaded benchmark-harness binary'
}

@test "benchmark workflow guard should reject an unsupported env option instead of ignoring a possible build" {
	cat >>cache-action.yml <<'STEP'
    - name: Extra build
      shell: bash
      run: env --chdir=/tmp cargo build --release -p unrelated
STEP
	run_validation
	xberg_assert_status 1
	xberg_assert_output_contains 'harness cache must build only the uploaded benchmark-harness binary'
}

@test "benchmark workflow guard should reject an unsupported env split string that can execute a build" {
	cat >>cache-action.yml <<'STEP'
    - name: Extra build
      shell: bash
      run: env --split-string="cargo build --release -p unrelated"
STEP
	run_validation
	xberg_assert_status 1
	xberg_assert_output_contains 'harness cache must build only the uploaded benchmark-harness binary'
}
