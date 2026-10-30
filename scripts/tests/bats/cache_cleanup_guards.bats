#!/usr/bin/env bats

setup() {
	[ -z "${XBERG_EXPECTED_BASH_MAJOR_MINOR:-}" ] || [[ "$BASH_VERSION" == "$XBERG_EXPECTED_BASH_MAJOR_MINOR".* ]]
	bats_load_library xberg-bats
	xberg_setup
	REPOSITORY_ROOT="$(cd "$BATS_TEST_DIRNAME/../../.." && pwd -P)"
	cd "$XBERG_WORK" || return
	mkdir -p .tesseract-cache/selected .tesseract-cache/neighbor .xdg-cache/selected .xdg-cache/neighbor \
		target/selected/xberg-tesseract-cache target/selected/release target/neighbor/xberg-tesseract-cache
	for file in .tesseract-cache/selected/marker .tesseract-cache/neighbor/marker .xdg-cache/selected/marker \
		.xdg-cache/neighbor/marker target/selected/xberg-tesseract-cache/marker target/selected/release/marker \
		target/neighbor/xberg-tesseract-cache/marker; do
		printf 'keep bytes\n' >"$file"
	done
}

@test "Tesseract directory cleanup should remove only both selected cache directories" {
	run "$BASH" "$REPOSITORY_ROOT/scripts/ci/actions/setup-tesseract-cache/clean-dirs.sh" selected
	xberg_assert_status 0
	[ ! -e .tesseract-cache/selected ]
	[ ! -e .xdg-cache/selected ]
	xberg_assert_file .tesseract-cache/neighbor/marker 'keep bytes'
	xberg_assert_file .xdg-cache/neighbor/marker 'keep bytes'
	xberg_assert_file target/selected/release/marker 'keep bytes'
}

@test "Tesseract target cleanup should preserve neighboring targets and release artifacts" {
	run "$BASH" "$REPOSITORY_ROOT/scripts/ci/actions/setup-tesseract-cache/clean-target-cache.sh" selected
	xberg_assert_status 0
	[ ! -e target/selected/xberg-tesseract-cache ]
	xberg_assert_file target/selected/release/marker 'keep bytes'
	xberg_assert_file target/neighbor/xberg-tesseract-cache/marker 'keep bytes'
	xberg_assert_file .tesseract-cache/selected/marker 'keep bytes'
}

@test "Tesseract cache cleanup should not follow a selected cache symlink into another directory" {
	mkdir victim
	printf 'outside cache bytes\n' >victim/marker
	rm .tesseract-cache/selected/marker
	rmdir .tesseract-cache/selected
	ln -s "$XBERG_WORK/victim" .tesseract-cache/selected
	run "$BASH" "$REPOSITORY_ROOT/scripts/ci/actions/setup-tesseract-cache/clean-dirs.sh" selected
	xberg_assert_status 0
	[ ! -e .tesseract-cache/selected ]
	xberg_assert_file victim/marker 'outside cache bytes'
}

@test "Tesseract directory cleanup should reject traversal labels before deleting a disposable sibling" {
	mkdir victim
	printf 'outside cache bytes\n' >victim/marker
	run "$BASH" "$REPOSITORY_ROOT/scripts/ci/actions/setup-tesseract-cache/clean-dirs.sh" ../victim
	xberg_assert_status 2
	xberg_assert_output_contains 'invalid cache label'
	xberg_assert_file victim/marker 'outside cache bytes'
	xberg_assert_file .tesseract-cache/selected/marker 'keep bytes'
}

@test "Tesseract target cleanup should reject traversal before deleting another disposable target" {
	mkdir -p victim/xberg-tesseract-cache
	printf 'outside target bytes\n' >victim/xberg-tesseract-cache/marker
	run "$BASH" "$REPOSITORY_ROOT/scripts/ci/actions/setup-tesseract-cache/clean-target-cache.sh" ../victim
	xberg_assert_status 2
	xberg_assert_output_contains 'invalid rust target'
	xberg_assert_file victim/xberg-tesseract-cache/marker 'outside target bytes'
	xberg_assert_file target/selected/xberg-tesseract-cache/marker 'keep bytes'
}

@test "Tesseract cleanup scripts should reject missing and dot-directory selectors" {
	for script in clean-dirs.sh clean-target-cache.sh; do
		run "$BASH" "$REPOSITORY_ROOT/scripts/ci/actions/setup-tesseract-cache/$script"
		xberg_assert_status 1
		for selector in . .. bad/name; do
			run "$BASH" "$REPOSITORY_ROOT/scripts/ci/actions/setup-tesseract-cache/$script" "$selector"
			xberg_assert_status 2
		done
	done
	xberg_assert_file .tesseract-cache/neighbor/marker 'keep bytes'
	xberg_assert_file target/neighbor/xberg-tesseract-cache/marker 'keep bytes'
}
