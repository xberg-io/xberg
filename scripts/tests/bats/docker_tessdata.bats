#!/usr/bin/env bats

setup() {
	bats_load_library xberg-bats
	xberg_setup
	REPOSITORY_ROOT="$(cd "$BATS_TEST_DIRNAME/../../.." && pwd -P)"
}

assert_tessdata_contract() {
	local dockerfile="$1"
	grep -Fq 'tessdata is not world-readable' "$dockerfile" || return 1
	grep -Fq '! -perm -0444' "$dockerfile" || return 1
	grep -Fq '/usr/share/tesseract-ocr/*/tessdata' "$dockerfile" || return 1
	if grep -Eq 'chmod[[:space:]]+-R' "$dockerfile"; then
		printf '%s\n' 'a recursive chmod copies the tessdata into a second layer' >&2
		return 1
	fi
	grep -q '^USER xberg$' "$dockerfile" || return 1
	grep -Fq 'chown -R xberg:xberg' "$dockerfile" || return 1
	if grep -Eq 'tesseract-ocr/[45]/tessdata' "$dockerfile"; then
		printf '%s\n' 'tessdata discovery must not depend on a Tesseract major version' >&2
		return 1
	fi
}

@test "core Dockerfile should discover readable tessdata for its nonroot user" {
	assert_tessdata_contract "$REPOSITORY_ROOT/docker/Dockerfile.core"
}

@test "full Dockerfile should discover readable tessdata for its nonroot user" {
	assert_tessdata_contract "$REPOSITORY_ROOT/docker/Dockerfile.full"
}

@test "tessdata contract should reject a hardcoded Tesseract version" {
	cp "$REPOSITORY_ROOT/docker/Dockerfile.core" "$XBERG_WORK/Dockerfile"
	printf '%s\n' 'ENV TESSDATA_PREFIX=/usr/share/tesseract-ocr/5/tessdata' >>"$XBERG_WORK/Dockerfile"
	run assert_tessdata_contract "$XBERG_WORK/Dockerfile"
	xberg_assert_status 1
	xberg_assert_output_contains 'must not depend on a Tesseract major version'
}

@test "tessdata contract should reject a missing readability check" {
	sed '/tessdata is not world-readable/d' "$REPOSITORY_ROOT/docker/Dockerfile.core" >"$XBERG_WORK/Dockerfile"
	run assert_tessdata_contract "$XBERG_WORK/Dockerfile"
	xberg_assert_status 1
}

@test "tessdata contract should reject a recursive chmod that duplicates the layer" {
	cp "$REPOSITORY_ROOT/docker/Dockerfile.core" "$XBERG_WORK/Dockerfile"
	printf '%s\n' 'RUN chmod -R a+rx /usr/share/tesseract-ocr' >>"$XBERG_WORK/Dockerfile"
	run assert_tessdata_contract "$XBERG_WORK/Dockerfile"
	xberg_assert_status 1
	xberg_assert_output_contains 'a recursive chmod copies the tessdata into a second layer'
}

@test "tessdata contract should reject missing discovery paths" {
	sed 's#/usr/share/tesseract-ocr/\*/tessdata#/usr/share/tessdata#' "$REPOSITORY_ROOT/docker/Dockerfile.core" >"$XBERG_WORK/Dockerfile"
	run assert_tessdata_contract "$XBERG_WORK/Dockerfile"
	xberg_assert_status 1
}
