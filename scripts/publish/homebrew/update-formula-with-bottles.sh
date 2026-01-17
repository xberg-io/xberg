#!/usr/bin/env bash

set -euo pipefail

artifacts_dir="${1:?Artifacts directory argument required}"
tap_dir="${2:-homebrew-tap}"
tag="${TAG:?TAG not set}"
version="${VERSION:?VERSION not set}"
dry_run="${DRY_RUN:-false}"
max_retries="${MAX_RETRIES:-3}"
retry_delay="${RETRY_DELAY:-5}"

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

# Function to validate SHA256 format (64 hex characters)
validate_sha256() {
  local sha256="$1"
  if [[ ! $sha256 =~ ^[a-f0-9]{64}$ ]]; then
    echo "Invalid SHA256 format: $sha256" >&2
    return 1
  fi
  return 0
}

# Function to compute and validate SHA256 with verification
compute_sha256() {
  local file="$1"
  local sha256
  sha256=$(shasum -a 256 "$file" | cut -d' ' -f1)

  if ! validate_sha256 "$sha256"; then
    echo "Error: Failed to compute valid SHA256 for $file" >&2
    return 1
  fi

  echo "$sha256"
}

# Download and process bottles from GitHub Release to ensure checksums match uploaded files
echo "Downloading bottles from GitHub Release to compute checksums..."
temp_bottles_dir=$(mktemp -d)

# Get list of expected bottles from local artifacts
for bottle in "$artifacts_dir"/kreuzberg-*.bottle.tar.gz; do
  if [ -f "$bottle" ]; then
    filename="$(basename "$bottle")"
    without_suffix="${filename%.bottle.tar.gz}"
    bottle_tag="${without_suffix##*.}"

    echo "Downloading bottle from release: $filename"
    bottle_url="https://github.com/kreuzberg-dev/kreuzberg/releases/download/$tag/$filename"
    downloaded_bottle="$temp_bottles_dir/$filename"

    # Download the actual uploaded bottle
    if ! download_with_retry "$bottle_url" "$downloaded_bottle"; then
      echo "Error: Failed to download bottle from $bottle_url" >&2
      exit 1
    fi

    # Verify downloaded file integrity
    if ! tar -tzf "$downloaded_bottle" >/dev/null 2>&1; then
      echo "Error: Downloaded bottle is corrupted or not a valid tar.gz: $downloaded_bottle" >&2
      exit 1
    fi

    # Compute checksum from the DOWNLOADED file (matches what users will get)
    if ! sha256=$(compute_sha256 "$downloaded_bottle"); then
      echo "Error: Failed to compute SHA256 for downloaded bottle" >&2
      exit 1
    fi

    bottle_hashes[$bottle_tag]=$sha256
    bottle_tags+=("$bottle_tag")
    echo "  $bottle_tag: $sha256"
  fi
done

if [ ${#bottle_hashes[@]} -eq 0 ]; then
  echo "Error: No bottle artifacts found in $artifacts_dir" >&2
  exit 1
fi

echo "Successfully validated ${#bottle_hashes[@]} bottles"

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

# Function to download file with retry logic and validation
download_with_retry() {
  local url="$1"
  local output_file="$2"
  local attempt=1

  while [ $attempt -le "$max_retries" ]; do
    echo "Downloading $url (attempt $attempt/$max_retries)..."

    # Download to temp file with curl checking HTTP status
    if curl -f -L --max-time 120 --retry 1 --retry-delay 2 -o "$output_file" "$url" 2>/dev/null; then
      echo "Download successful"
      return 0
    else
      exit_code=$?
      echo "Download failed with exit code $exit_code" >&2

      if [ $attempt -lt "$max_retries" ]; then
        echo "Waiting ${retry_delay}s before retry..."
        sleep "$retry_delay"
        ((attempt++))
      else
        echo "Error: Failed to download after $max_retries attempts" >&2
        return 1
      fi
    fi
  done

  return 1
}

# Fetch the SHA256 of the source tarball with proper temp file handling
echo "Fetching SHA256 of source tarball..."
tarball_url="https://github.com/kreuzberg-dev/kreuzberg/archive/$tag.tar.gz"
tarball_temp=$(mktemp)
trap 'rm -f "$tarball_temp"; rm -rf "$temp_bottles_dir"' EXIT

if ! download_with_retry "$tarball_url" "$tarball_temp"; then
  echo "Error: Failed to download source tarball from $tarball_url" >&2
  exit 1
fi

# Verify temp file is valid tar.gz before hashing
if ! tar -tzf "$tarball_temp" >/dev/null 2>&1; then
  echo "Error: Downloaded tarball is corrupted or not a valid tar.gz" >&2
  exit 1
fi

if ! tarball_sha256=$(compute_sha256 "$tarball_temp"); then
  echo "Error: Failed to compute valid SHA256 for source tarball" >&2
  exit 1
fi

echo "Source tarball SHA256: $tarball_sha256"

bottle_block="  bottle do"
bottle_block+=$'\n'"    root_url \"https://github.com/kreuzberg-dev/kreuzberg/releases/download/$tag\""

for bottle_tag in "${bottle_tags[@]}"; do
  sha256=${bottle_hashes[$bottle_tag]}
  bottle_block+=$'\n'"    sha256 cellar: :any_skip_relocation, $bottle_tag: \"$sha256\""
done

bottle_block+=$'\n'"  end"

# Update URL and sha256 (sha256 comes right after url line)
new_formula=$(echo "$formula_content" | sed \
  -e "s|url \"https://github.com/kreuzberg-dev/kreuzberg/archive/.*\.tar\.gz\"|url \"https://github.com/kreuzberg-dev/kreuzberg/archive/$tag.tar.gz\"|" \
  -e "s|sha256 \"[a-f0-9]*\"|sha256 \"$tarball_sha256\"|")

# Remove any existing bottle blocks (both commented and uncommented)
new_formula=$(echo "$new_formula" | sed '/^  bottle do$/,/^  end$/d')
new_formula=$(echo "$new_formula" | sed '/^  # bottle do$/,/^  # end$/d')

# Use Python for reliable multiline replacement since bash/sed/awk have issues with multiline variables
# Also removes extra blank lines and inserts the bottle block before first depends_on
new_formula=$(
  python3 <<PYTHON_SCRIPT
import re

formula = """$new_formula"""
bottle_block = """$bottle_block"""

# Remove multiple consecutive blank lines (keep max 1 blank line between sections)
lines = formula.split('\n')
result = []
prev_blank = False

for line in lines:
  is_blank = line.strip() == ''

  # Skip consecutive blank lines
  if is_blank and prev_blank:
    continue

  prev_blank = is_blank
  result.append(line)

# Now insert the bottle block before the first depends_on
final_result = []
inserted = False

for line in result:
  if line.startswith('  depends_on') and not inserted:
    # Insert bottle block before this line
    final_result.append(bottle_block)
    final_result.append('')
    inserted = True
  final_result.append(line)

print('\n'.join(final_result))
PYTHON_SCRIPT
)

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
