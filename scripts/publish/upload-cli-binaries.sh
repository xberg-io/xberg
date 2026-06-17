#!/usr/bin/env bash

set -euo pipefail

tag="${1:?Release tag argument required}"
artifacts_dir="${2:-dist/cli}"
max_retries="${MAX_RETRIES:-3}"
retry_delay="${RETRY_DELAY:-5}"

if [ ! -d "$artifacts_dir" ]; then
  echo "Error: Artifacts directory not found: $artifacts_dir" >&2
  exit 1
fi

existing_assets="$(mktemp)"
trap 'rm -f "$existing_assets"' EXIT

# Function to validate SHA256 format (64 hex characters)
validate_sha256() {
  local sha256="$1"
  if [[ ! $sha256 =~ ^[a-f0-9]{64}$ ]]; then
    echo "Invalid SHA256 format: $sha256" >&2
    return 1
  fi
  return 0
}

# Function to compute and validate SHA256
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

# Fetch existing assets with error handling
echo "Fetching existing release assets..."
if ! gh release view "$tag" --json assets | jq -r '.assets[].name' >"$existing_assets" 2>/dev/null; then
  echo "Warning: Could not fetch existing assets (release may be new)" >&2
fi

# Function to upload with retry logic
upload_with_retry() {
  local file="$1"
  local tag="$2"
  local attempt=1

  while [ $attempt -le "$max_retries" ]; do
    echo "Uploading $file (attempt $attempt/$max_retries)..."

    if gh release upload "$tag" "$file" --clobber 2>/dev/null; then
      echo "Upload successful"
      return 0
    else
      exit_code=$?
      echo "Upload failed with exit code $exit_code" >&2

      if [ $attempt -lt "$max_retries" ]; then
        echo "Waiting ${retry_delay}s before retry..."
        sleep "$retry_delay"
        ((attempt++))
      else
        echo "Error: Failed to upload after $max_retries attempts" >&2
        return 1
      fi
    fi
  done

  return 1
}

binary_count=0
uploaded_count=0
skipped_count=0

for file in "$artifacts_dir"/kreuzberg-cli-*; do
  if [ -f "$file" ]; then
    base="$(basename "$file")"
    ((binary_count++)) || true

    echo "Processing: $base"

    # Verify SHA256 is valid
    if ! sha256=$(compute_sha256 "$file"); then
      echo "Error: Failed to compute valid SHA256 for $file" >&2
      exit 1
    fi
    echo "  SHA256: $sha256"

    if grep -Fxq "$base" "$existing_assets" 2>/dev/null; then
      echo "Skipping $base (already uploaded)"
      ((skipped_count++)) || true
    else
      if upload_with_retry "$file" "$tag"; then
        echo "Uploaded $base"
        ((uploaded_count++)) || true
      else
        echo "Error: Failed to upload $base after $max_retries attempts" >&2
        exit 1
      fi
    fi
  fi
done

echo ""
echo "=== Upload Summary ==="
echo "Total binaries: $binary_count"
echo "Uploaded: $uploaded_count"
echo "Skipped: $skipped_count"
echo ""

if [ "$uploaded_count" -eq 0 ] && [ "$binary_count" -gt 0 ]; then
  echo "Note: All $binary_count binaries already exist in release"
fi

echo "CLI binaries uploaded to $tag"
