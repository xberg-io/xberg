#!/usr/bin/env bats

setup() {
	[ -z "${XBERG_EXPECTED_BASH_MAJOR_MINOR:-}" ] || [[ "$BASH_VERSION" == "$XBERG_EXPECTED_BASH_MAJOR_MINOR".* ]]
	bats_load_library xberg-bats
	xberg_setup
	REPOSITORY_ROOT="$(cd "$BATS_TEST_DIRNAME/../../.." && pwd -P)"
	mkdir -p "$XBERG_WORK/docker" "$XBERG_WORK/crates/xberg" "$XBERG_WORK/crates/xberg-py" "$XBERG_WORK/scripts/test"
	printf '[workspace]\nmembers = ["crates/xberg", "crates/xberg-py"]\n' >"$XBERG_WORK/Cargo.toml"
	printf '*\n!crates/\n!crates/xberg/\n!crates/xberg-py/\n' >"$XBERG_WORK/.dockerignore"
	printf 'COPY crates/xberg/ crates/xberg/\nCOPY crates/xberg-py/ crates/xberg-py/\n' >"$XBERG_WORK/docker/Dockerfile.first"
	cp "$XBERG_WORK/docker/Dockerfile.first" "$XBERG_WORK/docker/Dockerfile.second"
	unset PYTHON_STATUS CONFIG_STATUS
	xberg_stub git 'printf "%s\n" "$XBERG_WORK"'
	xberg_stub python3 'printf "python3 %s\n" "$*" >>"$XBERG_TRACE"' 'exit "${PYTHON_STATUS:-0}"'
	printf '#!/usr/bin/env bash\nprintf "config %%s\\n" "$*" >>"$XBERG_TRACE"\nexit "${CONFIG_STATUS:-0}"\n' \
		>"$XBERG_WORK/scripts/test/test-docker-config-local.sh"
	chmod +x "$XBERG_WORK/scripts/test/test-docker-config-local.sh"
	cd "$XBERG_WORK" || return
}

run_coverage() {
	run "$BASH" "$REPOSITORY_ROOT/scripts/ci/docker/check-dockerfile-crate-coverage.sh"
}

@test "Docker crate guard should verify both real members across every Dockerfile and ignore allowlist" {
	run_coverage
	xberg_assert_status 0
	xberg_assert_output_contains 'All Cargo workspace members are covered'
	xberg_assert_output_contains 'Checked 2 workspace crates across 2 Dockerfiles.'
}

@test "Docker crate guard should reject a missing core COPY even when a wrapper crate is sed-excluded" {
	printf 'COPY crates/xberg-py/ crates/xberg-py/\nRUN sed -i '\''/xberg-py/d'\'' Cargo.toml\n' >docker/Dockerfile.second
	run_coverage
	xberg_assert_status 1
	xberg_assert_output_contains "file=docker/Dockerfile.second::workspace member 'crates/xberg' is neither COPYd nor sed-excluded"
}

@test "Docker crate guard should accept an exact member exclusion without confusing it with a substring" {
	printf 'COPY crates/xberg/ crates/xberg/\nRUN sed -i '\''/xberg-py/d'\'' Cargo.toml\n' >docker/Dockerfile.second
	run_coverage
	xberg_assert_status 0
	xberg_assert_output_contains 'every COPYd path survives .dockerignore'
}

@test "Docker crate guard should reject a dangling COPY absent from the workspace" {
	printf 'COPY crates/deleted/ crates/deleted/\n' >>docker/Dockerfile.second
	run_coverage
	xberg_assert_status 1
	xberg_assert_output_contains "COPY names 'crates/deleted', which is not a Cargo workspace member"
}

@test "Docker crate guard should reject COPY paths excluded from the actual build context" {
	printf '*\n!crates/\n!crates/xberg-py/\n' >.dockerignore
	run_coverage
	xberg_assert_status 1
	xberg_assert_output_contains "COPY names 'crates/xberg', which .dockerignore excludes"
}

@test "Docker crate guard should reject stale allowlist entries even when every COPY is covered" {
	printf '!crates/deleted/\n' >>.dockerignore
	run_coverage
	xberg_assert_status 1
	xberg_assert_output_contains "allowlists 'crates/deleted', which no longer exists"
}

@test "Docker crate guard should reject a sweep with zero Dockerfiles" {
	rm docker/Dockerfile.first docker/Dockerfile.second
	run_coverage
	xberg_assert_status 1
	xberg_assert_output_contains 'no Dockerfiles found'
	[[ "$output" != *'All Cargo workspace members are covered'* ]]
}

@test "Docker crate guard should reject zero collected workspace crates" {
	printf '[workspace]\nmembers = []\n' >Cargo.toml
	printf 'FROM scratch\n' >docker/Dockerfile.first
	printf 'FROM scratch\n' >docker/Dockerfile.second
	printf '*\n' >.dockerignore
	run_coverage
	xberg_assert_status 1
	xberg_assert_output_contains 'no workspace crates found'
	[[ "$output" != *'All Cargo workspace members are covered'* ]]
}

@test "Docker CLI runner should execute unit contracts before the real image test command" {
	run "$BASH" "$REPOSITORY_ROOT/scripts/ci/docker/run-cli-tests.sh"
	xberg_assert_status 0
	xberg_assert_trace 'python3 -m unittest scripts/ci/docker/test_docker_unit.py
python3 scripts/ci/docker/test_docker.py --image xberg:cli --variant cli --verbose'
}

@test "Docker CLI runner should stop before image testing when its unit contract fails" {
	export PYTHON_STATUS=18
	run "$BASH" "$REPOSITORY_ROOT/scripts/ci/docker/run-cli-tests.sh"
	xberg_assert_status 18
	xberg_assert_trace 'python3 -m unittest scripts/ci/docker/test_docker_unit.py'
}

@test "Docker feature runner should forward the exact selected variant and preserve failure" {
	run "$BASH" "$REPOSITORY_ROOT/scripts/ci/docker/run-feature-tests.sh" full
	xberg_assert_status 0
	xberg_assert_trace 'python3 scripts/ci/docker/test_docker.py --image xberg:full --variant full --verbose'
	: >"$XBERG_TRACE"
	export PYTHON_STATUS=19
	run "$BASH" "$REPOSITORY_ROOT/scripts/ci/docker/run-feature-tests.sh" core
	xberg_assert_status 19
	xberg_assert_trace 'python3 scripts/ci/docker/test_docker.py --image xberg:core --variant core --verbose'
}

@test "Docker config runner should exec its image integration harness with the requested variant" {
	run "$BASH" "$REPOSITORY_ROOT/scripts/ci/docker/run-config-tests.sh" full
	xberg_assert_status 0
	xberg_assert_trace 'config --image xberg:full --variant full'
	: >"$XBERG_TRACE"
	export CONFIG_STATUS=20
	run "$BASH" "$REPOSITORY_ROOT/scripts/ci/docker/run-config-tests.sh" cli
	xberg_assert_status 20
	xberg_assert_trace 'config --image xberg:cli --variant cli'
}

@test "Docker variant runners should reject missing variants before executing any tools" {
	for script in run-feature-tests.sh run-config-tests.sh; do
		run "$BASH" "$REPOSITORY_ROOT/scripts/ci/docker/$script"
		xberg_assert_status 1
		xberg_assert_output_contains 'missing variant'
		xberg_assert_trace_empty
	done
}

@test "Docker crate guard should reject an empty ignore allowlist with a real COPY" {
	printf '*\n' >.dockerignore
	run_coverage
	xberg_assert_status 1
	xberg_assert_output_contains "COPY names 'crates/xberg', which .dockerignore excludes"
}

@test "Docker crate guard should count distinct member paths repeated by default members only once" {
	cat >Cargo.toml <<'MANIFEST'
[workspace]
members = ["crates/xberg", "crates/xberg-py"]
default-members = ["crates/xberg", "crates/xberg-py"]
MANIFEST
	run_coverage
	xberg_assert_status 0
	xberg_assert_output_contains 'Checked 2 workspace crates across 2 Dockerfiles.'
}
