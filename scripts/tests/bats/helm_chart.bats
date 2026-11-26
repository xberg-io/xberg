#!/usr/bin/env bats

setup() {
	bats_load_library xberg-bats
	xberg_setup
	REPOSITORY_ROOT="$(cd "$BATS_TEST_DIRNAME/../../.." && pwd -P)"
	CHART="$REPOSITORY_ROOT/charts/xberg"
	APP_VERSION="$(awk -F'"' '/^appVersion:/ {print $2}' "$CHART/Chart.yaml")"
	[ -n "$APP_VERSION" ]
	command -v helm >/dev/null || {
		printf '%s\n' 'helm is required: run through `mise exec -- bats` or put the mise helm on PATH' >&2
		return 1
	}
}

render_deployment() {
	run helm template release "$CHART" --show-only templates/deployment.yaml "$@"
}

@test "chart should seed the cache volume from the image by default" {
	render_deployment
	xberg_assert_status 0
	xberg_assert_output_contains '- name: seed-cache'
	xberg_assert_output_contains "image: ghcr.io/xberg-io/xberg:$APP_VERSION"
	xberg_assert_output_contains 'cp -R --preserve=mode,timestamps --update=none /app/.xberg/. /seed/'
	xberg_assert_output_contains 'mountPath: /seed'
}

@test "chart seed should run after the ownership fix" {
	render_deployment
	xberg_assert_status 0
	chown_line="$(printf '%s\n' "$output" | grep -n -- '- name: init-cache' | cut -d: -f1)"
	seed_line="$(printf '%s\n' "$output" | grep -n -- '- name: seed-cache' | cut -d: -f1)"
	[ -n "$chown_line" ] && [ -n "$seed_line" ] && [ "$chown_line" -lt "$seed_line" ]
}

@test "chart should seed the emptyDir cache when the PVC is off" {
	render_deployment --set cache.enabled=false
	xberg_assert_status 0
	xberg_assert_output_contains '- name: seed-cache'
	xberg_assert_output_contains 'emptyDir: {}'
	case "$output" in *'- name: init-cache'*) return 1 ;; esac
}

@test "chart should render no init containers with seeding and chown off" {
	render_deployment --set cache.seedFromImage=false --set cache.initChown=false
	xberg_assert_status 0
	case "$output" in *initContainers*) return 1 ;; esac
}
