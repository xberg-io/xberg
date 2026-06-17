#!/usr/bin/env bash
#
# Generate checksum file for Elixir NIF binaries from GitHub release
#
# Usage: ./generate_elixir_checksums.sh <version>
# Example: ./generate_elixir_checksums.sh 4.0.6
#
# This script downloads all NIF binaries from the GitHub release and generates
# the checksum file required by RustlerPrecompiled. It must be run BEFORE
# `mix compile` because RustlerPrecompiled validates checksums during compilation.

set -euo pipefail

VERSION="${1:?Usage: $0 <version>}"
REPO="kreuzberg-dev/kreuzberg"
CHECKSUM_FILE="packages/elixir/checksum-Elixir.Kreuzberg.Native.exs"

# Targets that are actually built in CI (from publish.yaml elixir-natives matrix)
TARGETS=(
  "aarch64-apple-darwin"
  "aarch64-unknown-linux-gnu"
  "x86_64-unknown-linux-gnu"
  "x86_64-pc-windows-gnu"
)

# NIF versions from native.ex
NIF_VERSIONS=("2.16" "2.17")

# Create temporary directory for downloads
TMPDIR=$(mktemp -d)
trap 'rm -rf "$TMPDIR"' EXIT

echo "Generating checksums for v${VERSION}..."
echo "Download directory: $TMPDIR"

# Start building the checksum file content
CHECKSUMS=()

for TARGET in "${TARGETS[@]}"; do
  for NIF_VERSION in "${NIF_VERSIONS[@]}"; do
    # Determine extension based on target
    if [[ "$TARGET" == *"windows"* ]]; then
      EXT="dll"
    else
      EXT="so"
    fi

    FILENAME="libkreuzberg_nif-v${VERSION}-nif-${NIF_VERSION}-${TARGET}.${EXT}.tar.gz"
    URL="https://github.com/${REPO}/releases/download/v${VERSION}/${FILENAME}"

    echo "Downloading: $FILENAME"

    # Download the file
    if curl -fsSL -o "${TMPDIR}/${FILENAME}" "$URL"; then
      # Calculate SHA256 checksum
      if command -v sha256sum &>/dev/null; then
        CHECKSUM=$(sha256sum "${TMPDIR}/${FILENAME}" | cut -d' ' -f1)
      elif command -v shasum &>/dev/null; then
        CHECKSUM=$(shasum -a 256 "${TMPDIR}/${FILENAME}" | cut -d' ' -f1)
      else
        echo "ERROR: No sha256sum or shasum command found"
        exit 1
      fi

      echo "  Checksum: sha256:${CHECKSUM}"
      CHECKSUMS+=("  \"${FILENAME}\" => \"sha256:${CHECKSUM}\",")
    else
      echo "  ERROR: Failed to download $FILENAME"
      exit 1
    fi
  done
done

# Sort checksums for consistent output
mapfile -t SORTED_CHECKSUMS < <(printf '%s\n' "${CHECKSUMS[@]}" | sort)

# Write the checksum file
echo "Writing checksum file: $CHECKSUM_FILE"
{
  echo "%{"
  for CHECKSUM in "${SORTED_CHECKSUMS[@]}"; do
    echo "$CHECKSUM"
  done
  echo "}"
} >"$CHECKSUM_FILE"

echo ""
echo "Done! Generated checksums for ${#SORTED_CHECKSUMS[@]} files."
echo ""
echo "Contents of $CHECKSUM_FILE:"
cat "$CHECKSUM_FILE"
