#!/usr/bin/env bash

set -euo pipefail

artifacts_dir="${1:-$(pwd)}"

cd "$artifacts_dir" || {
	echo "Error: Cannot change to directory: $artifacts_dir" >&2
	exit 1
}

if ! gem update --system; then
	echo "::warning::Skipping RubyGems update; system RubyGems installation does not support self-update (apt-managed runner)." >&2
fi

shopt -s nullglob
mapfile -t gems < <(find . -maxdepth 1 -name 'kreuzberg-*.gem' -print | sort)

if [ ${#gems[@]} -eq 0 ]; then
	echo "No gem artifacts found in $artifacts_dir" >&2
	exit 1
fi

echo "Rebuilding gems to fix potential corruption from artifact transfer..."
for gem in "${gems[@]}"; do
	echo "Rebuilding ${gem} to ensure consistent structure"

	echo "Checking gzip integrity of ${gem}..."
	if ! gunzip -t "${gem}" 2>/dev/null; then
		echo "::warning::Gem ${gem} has corrupted gzip structure, attempting repair..." >&2

		if ! xxd -p -l 2 "${gem}" | grep -q "^1f8b"; then
			echo "::error::Gem ${gem} is not a valid gzip file (missing magic number)" >&2
			exit 1
		fi

		echo "Attempting gzip repair for ${gem}..."
		temp_dir=$(mktemp -d)
		if gunzip -c "${gem}" >"${temp_dir}/uncompressed" 2>/dev/null; then
			gzip -9 -c "${temp_dir}/uncompressed" >"${gem}.repaired"
			mv "${gem}.repaired" "${gem}"
			echo "Successfully repaired ${gem}"
		else
			echo "::error::Cannot repair ${gem} - gzip structure is too damaged" >&2
			rm -rf "${temp_dir}"
			exit 1
		fi
		rm -rf "${temp_dir}"
	else
		echo "✓ Gzip integrity check passed for ${gem}"
	fi

	gem unpack "${gem}"
	gem_name=$(basename "${gem}" .gem)

	gem specification "${gem}" --ruby >"${gem_name}/${gem_name}.gemspec"

	(cd "${gem_name}" && gem build "${gem_name}.gemspec")

	mv "${gem_name}/${gem}" "./${gem}"

	rm -rf "${gem_name}"

	echo "Rebuilt ${gem} successfully"
done

echo "All gems rebuilt successfully"
echo ""

echo "Validating rebuilt gem files..."
for gem in "${gems[@]}"; do
	echo "Checking $gem..."

	if [ ! -f "$gem" ] || [ ! -r "$gem" ] || [ ! -s "$gem" ]; then
		echo "::error::Gem file is invalid (missing, unreadable, or empty): $gem" >&2
		exit 1
	fi

	file_output=$(file "$gem" 2>/dev/null || echo "")
	echo "File type: $file_output"

	echo "Validating gem with gem spec..."
	if ! gem spec "$gem" >/dev/null 2>&1; then
		echo "::error::Gem file validation failed: $gem" >&2
		echo "File type: $(file "$gem")" >&2
		exit 1
	fi
	echo "✓ Gem file validation passed"
done

echo "All gem files validated successfully"
echo ""

echo "Publishing gems to RubyGems..."
failed_gems=()
for gem in "${gems[@]}"; do
	echo "Pushing ${gem} to RubyGems"
	publish_log=$(mktemp)
	set +e
	gem push "$gem" 2>&1 | tee "$publish_log"
	status=${PIPESTATUS[0]}
	set -e

	if [ "$status" -ne 0 ]; then
		if grep -qE "Repushing of gem versions is not allowed|already been pushed" "$publish_log"; then
			echo "::notice::Gem $gem version already published on RubyGems; skipping."
			if [ -n "${GITHUB_STEP_SUMMARY:-}" ]; then
				echo "Gem $(basename "$gem") already published; skipping." >>"$GITHUB_STEP_SUMMARY"
			fi
		else
			failed_gems+=("$gem")
		fi
	fi

	rm -f "$publish_log"
done

if [ ${#failed_gems[@]} -gt 0 ]; then
	echo "::error::Failed to publish the following gems:" >&2
	for gem in "${failed_gems[@]}"; do
		echo "  - $gem" >&2
	done
	exit 1
fi

if [ -n "${GITHUB_STEP_SUMMARY:-}" ] && [ -n "${RUBYGEMS_VERSION:-}" ]; then
	echo "Successfully published kreuzberg version ${RUBYGEMS_VERSION} to RubyGems" >>"$GITHUB_STEP_SUMMARY"
fi

echo "All gems processed"
