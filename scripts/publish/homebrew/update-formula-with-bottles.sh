#!/usr/bin/env bash

set -euo pipefail

artifacts_dir="${1:?Artifacts directory argument required}"
tap_dir="${2:-homebrew-tap}"
tag="${TAG:?TAG not set}"
version="${VERSION:?VERSION not set}"
dry_run="${DRY_RUN:-false}"

if [ ! -d "$artifacts_dir" ]; then
	echo "Error: Artifacts directory not found: $artifacts_dir" >&2
	exit 1
fi

echo "=== Updating Homebrew formula with bottles ==="
echo "Tag: $tag"
echo "Version: $version"
echo "Artifacts: $artifacts_dir"

declare -A bottle_hashes
declare -a bottle_tags

for bottle in "$artifacts_dir"/kreuzberg--*.bottle.tar.gz; do
	if [ -f "$bottle" ]; then
		filename="$(basename "$bottle")"
		without_suffix="${filename%.bottle.tar.gz}"
		bottle_tag="${without_suffix##*.}"
		sha256=$(shasum -a 256 "$bottle" | cut -d' ' -f1)

		bottle_hashes[$bottle_tag]=$sha256
		bottle_tags+=("$bottle_tag")
		echo "  $bottle_tag: $sha256"
	fi
done

if [ ${#bottle_hashes[@]} -eq 0 ]; then
	echo "Warning: No bottle artifacts found" >&2
	exit 1
fi

if [ ! -d "$tap_dir" ]; then
	echo "Cloning homebrew-tap..."
	git clone https://github.com/kreuzberg-dev/homebrew-tap.git "$tap_dir"
fi

formula_path="$tap_dir/Formula/kreuzberg.rb"

if [ ! -f "$formula_path" ]; then
	echo "Error: Formula not found at $formula_path" >&2
	exit 1
fi

formula_content=$(<"$formula_path")

bottle_block="  bottle do"
bottle_block+=$'\n'"    root_url \"https://github.com/kreuzberg-dev/kreuzberg/releases/download/$tag\""

for bottle_tag in "${bottle_tags[@]}"; do
	sha256=${bottle_hashes[$bottle_tag]}
	bottle_block+=$'\n'"    sha256 cellar: :any_skip_relocation, $bottle_tag: \"$sha256\""
done

bottle_block+=$'\n'"  end"

new_formula=$(echo "$formula_content" | sed \
	-e "s/url \"https:\/\/github.com\/kreuzberg-dev\/kreuzberg\/archive\/.*\.tar\.gz\"/url \"https:\/\/github.com\/kreuzberg-dev\/kreuzberg\/archive\/$tag.tar.gz\"/" \
	-e "s/version \"[^\"]*\"/version \"$version\"/")

new_formula=$(echo "$new_formula" | sed '/# bottle do/,/# end/d')

new_formula=$(echo "$new_formula" | sed "/^  depends_on/i\\
$bottle_block
")

echo "$new_formula" >"$formula_path"

echo ""
echo "=== Updated formula ==="
head -30 "$formula_path"
echo "..."

if [ "$dry_run" = "true" ]; then
	echo ""
	echo "Dry run mode: skipping git operations"
	echo "Formula would be updated at: $formula_path"
	exit 0
fi

cd "$tap_dir"
git config user.name "kreuzberg-bot"
git config user.email "bot@kreuzberg.dev"

if git diff --quiet Formula/kreuzberg.rb; then
	echo "No changes to formula"
	exit 0
fi

git add Formula/kreuzberg.rb
git commit -m "chore(homebrew): update kreuzberg to $version

Auto-update from release $tag

Includes pre-built bottles for macOS"

echo "Pushing to homebrew-tap..."
git push origin main

echo "Formula updated successfully"
