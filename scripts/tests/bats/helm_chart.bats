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

# Print the value of one container env entry; status 1 when the entry is absent.
env_value() {
	printf '%s\n' "$output" | awk -v name="$1" '
		$0 ~ "- name: " name "$" { found = 1; next }
		found && /value:/ { sub(/.*value: /, ""); gsub(/"/, ""); print; hit = 1; exit }
		END { exit hit ? 0 : 1 }
	'
}

# Assert both the server and the seed container run exactly this image.
assert_images() {
	images="$(printf '%s\n' "$output" | awk '/^ *image: ghcr/ {print $2}' | sort -u)"
	[ "$images" = "ghcr.io/xberg-io/xberg:$1" ] || xberg__fail_compare "images" "ghcr.io/xberg-io/xberg:$1" "$images"
	[ "$(printf '%s\n' "$output" | grep -c '^ *image: ghcr')" -eq 2 ]
}

@test "chart should default to fast models without TESSDATA_PREFIX" {
	render_deployment
	xberg_assert_status 0
	assert_images "$APP_VERSION"
	[ "$(env_value XBERG_OCR_TESSDATA_MODEL)" = fast ]
	run env_value TESSDATA_PREFIX
	xberg_assert_status 1
}

@test "chart should pass an explicit TESSDATA_PREFIX through for fast models" {
	render_deployment --set xberg.tessdataPrefix=/opt/tessdata
	xberg_assert_status 0
	[ "$(env_value TESSDATA_PREFIX)" = /opt/tessdata ]
}

@test "chart should select the best twin of each image tag" {
	for pair in ":$APP_VERSION-best" "latest:latest-best" "core:core-best" "1.2.3-core:1.2.3-core-best" "core-best:core-best" "latest-best:latest-best"; do
		render_deployment --set ocr.tesseractModelSet=best --set "image.tag=${pair%%:*}"
		xberg_assert_status 0
		assert_images "${pair#*:}"
		[ "$(env_value XBERG_OCR_TESSDATA_MODEL)" = best ]
	done
}

@test "chart should keep the tag and download best models with the download source" {
	render_deployment --set ocr.tesseractModelSet=best --set ocr.tesseractModelSource=download --set image.tag=core
	xberg_assert_status 0
	assert_images core
	[ "$(env_value XBERG_OCR_TESSDATA_MODEL)" = best ]
}

@test "chart should ignore the model source for fast models" {
	render_deployment --set ocr.tesseractModelSource=image --set image.tag=latest
	xberg_assert_status 0
	assert_images latest
}

@test "chart should reject TESSDATA_PREFIX with best models" {
	render_deployment --set ocr.tesseractModelSet=best --set xberg.tessdataPrefix=/opt/tessdata
	xberg_assert_status 1
	xberg_assert_output_contains 'xberg.tessdataPrefix must be empty with ocr.tesseractModelSet=best'
}

@test "chart should reject an unknown model set or source" {
	render_deployment --set ocr.tesseractModelSet=medium
	xberg_assert_status 1
	xberg_assert_output_contains "/ocr/tesseractModelSet"
	render_deployment --set ocr.tesseractModelSource=registry
	xberg_assert_status 1
	xberg_assert_output_contains "/ocr/tesseractModelSource"
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
