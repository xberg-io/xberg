#!/usr/bin/env bash

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="${REPO_ROOT:-$(cd "$SCRIPT_DIR/../../.." && pwd)}"

source "$REPO_ROOT/scripts/lib/common.sh"

validate_repo_root "$REPO_ROOT" || exit 1

echo "=== Vendoring kreuzberg core crate ==="

# Extract version from root workspace
core_version=$(awk -F '"' '/^\[workspace.package\]/,/^version =/ {if ($0 ~ /^version =/) {print $2; exit}}' "$REPO_ROOT/Cargo.toml")

# Extract simple version string (handles both "version" and { version = "..." })
extract_version() {
  local dep_name="$1"
  awk -F '"' "
    /^${dep_name} = \\{ version =/ {print \$2; exit}
    /^${dep_name} = \"/ {print \$2; exit}
  " "$REPO_ROOT/Cargo.toml"
}

TOKIO_VERSION=$(extract_version "tokio")
SERDE_VERSION=$(extract_version "serde")
SERDE_JSON_VERSION=$(extract_version "serde_json")
THISERROR_VERSION=$(extract_version "thiserror")
ANYHOW_VERSION=$(extract_version "anyhow")
ASYNC_TRAIT_VERSION=$(extract_version "async-trait")
LIBC_VERSION=$(extract_version "libc")
PARKING_LOT_VERSION=$(extract_version "parking_lot")
TRACING_VERSION=$(extract_version "tracing")
AHASH_VERSION=$(extract_version "ahash")
BASE64_VERSION=$(extract_version "base64")
HEX_VERSION=$(extract_version "hex")
TOML_VERSION=$(extract_version "toml")
NUM_CPUS_VERSION=$(extract_version "num_cpus")
ONCE_CELL_VERSION=$(extract_version "once_cell")
HTML_TO_MARKDOWN_VERSION=$(extract_version "html-to-markdown-rs")
REQWEST_VERSION=$(extract_version "reqwest")
IMAGE_VERSION=$(extract_version "image")
TEMPFILE_VERSION=$(extract_version "tempfile")
CRITERION_VERSION=$(extract_version "criterion")
LZMA_RUST_VERSION=$(extract_version "lzma-rust2")
GETRANDOM_VERSION=$(extract_version "getrandom")

echo "Extracted versions from root workspace:"
echo "  core: $core_version"
echo "  reqwest: $REQWEST_VERSION"
echo "  tokio: $TOKIO_VERSION"

# Clean and create vendor directory
rm -rf "$REPO_ROOT/packages/ruby/vendor"
mkdir -p "$REPO_ROOT/packages/ruby/vendor"

# Copy crates
cp -R "$REPO_ROOT/crates/kreuzberg" "$REPO_ROOT/packages/ruby/vendor/kreuzberg"
cp -R "$REPO_ROOT/crates/kreuzberg-tesseract" "$REPO_ROOT/packages/ruby/vendor/kreuzberg-tesseract"
cp -R "$REPO_ROOT/crates/kreuzberg-ffi" "$REPO_ROOT/packages/ruby/vendor/kreuzberg-ffi"

if [ -d "$REPO_ROOT/vendor/rb-sys" ]; then
  cp -R "$REPO_ROOT/vendor/rb-sys" "$REPO_ROOT/packages/ruby/vendor/rb-sys"
fi

# Clean up build artifacts
for dir in kreuzberg kreuzberg-tesseract kreuzberg-ffi rb-sys; do
  if [ -d "$REPO_ROOT/packages/ruby/vendor/$dir" ]; then
    rm -rf "$REPO_ROOT/packages/ruby/vendor/$dir/.fastembed_cache"
    rm -rf "$REPO_ROOT/packages/ruby/vendor/$dir/target"
    find "$REPO_ROOT/packages/ruby/vendor/$dir" -name '*.swp' -delete 2>/dev/null || true
    find "$REPO_ROOT/packages/ruby/vendor/$dir" -name '*.bak' -delete 2>/dev/null || true
    find "$REPO_ROOT/packages/ruby/vendor/$dir" -name '*.tmp' -delete 2>/dev/null || true
    find "$REPO_ROOT/packages/ruby/vendor/$dir" -name '*~' -delete 2>/dev/null || true
  fi
done

# Update kreuzberg and kreuzberg-tesseract to use local workspace dependencies
for crate_dir in kreuzberg kreuzberg-tesseract; do
  # Replace workspace = true with actual versions for metadata fields
  sed -i.bak "s/^version.workspace = true/version = \"${core_version}\"/" "$REPO_ROOT/packages/ruby/vendor/$crate_dir/Cargo.toml"
  sed -i.bak 's/^edition.workspace = true/edition = "2024"/' "$REPO_ROOT/packages/ruby/vendor/$crate_dir/Cargo.toml"
  sed -i.bak 's/^rust-version.workspace = true/rust-version = "1.91"/' "$REPO_ROOT/packages/ruby/vendor/$crate_dir/Cargo.toml"
  sed -i.bak 's/^authors.workspace = true/authors = ["Na'\''aman Hirschfeld <nhirschfeld@gmail.com>"]/' "$REPO_ROOT/packages/ruby/vendor/$crate_dir/Cargo.toml"
  sed -i.bak 's/^license.workspace = true/license = "MIT"/' "$REPO_ROOT/packages/ruby/vendor/$crate_dir/Cargo.toml"

  # Fix reqwest features - ensure "rustls" is used (reqwest 0.13.0+ renamed rustls-tls to rustls)
  sed -i.bak 's/"rustls-tls",/"rustls",/' "$REPO_ROOT/packages/ruby/vendor/$crate_dir/Cargo.toml"

  rm -f "$REPO_ROOT/packages/ruby/vendor/$crate_dir/Cargo.toml.bak"
done

# Update kreuzberg-tesseract path in kreuzberg
sed -i.bak \
  's/^kreuzberg-tesseract = { version = "[^"]*", optional = true }/kreuzberg-tesseract = { path = "..\/kreuzberg-tesseract", optional = true }/' \
  "$REPO_ROOT/packages/ruby/vendor/kreuzberg/Cargo.toml"
rm -f "$REPO_ROOT/packages/ruby/vendor/kreuzberg/Cargo.toml.bak"

# Update native extension Cargo.toml to use vendored paths
echo "Updating native extension paths to vendored crates..."
NATIVE_CARGO_TOML="$REPO_ROOT/packages/ruby/ext/kreuzberg_rb/native/Cargo.toml"
if [ -f "$NATIVE_CARGO_TOML" ]; then
  echo "  Original Cargo.toml paths:"
  grep -E 'path = "\.\./|path = "\.\./' "$NATIVE_CARGO_TOML" || true

  # Use perl for more reliable path replacement (handles escaping better than sed)
  # Update kreuzberg path: ../../../../../crates/kreuzberg -> ../../../vendor/kreuzberg
  perl -i.bak -pe 's{path = "\.\./\.\./\.\./\.\./\.\./crates/kreuzberg"}{path = "../../../vendor/kreuzberg"}g' "$NATIVE_CARGO_TOML"

  # Update kreuzberg-ffi path: ../../../../../crates/kreuzberg-ffi -> ../../../vendor/kreuzberg-ffi
  perl -i.bak -pe 's{path = "\.\./\.\./\.\./\.\./\.\./crates/kreuzberg-ffi"}{path = "../../../vendor/kreuzberg-ffi"}g' "$NATIVE_CARGO_TOML"

  rm -f "$NATIVE_CARGO_TOML.bak"

  # Verify the replacements worked
  echo "  Updated Cargo.toml paths:"
  grep -E 'path = "\.\./|path = "\.\./' "$NATIVE_CARGO_TOML" || true

  # Validate that no original paths remain (use -F for fixed string matching)
  if grep -qF 'path = "../../../../../' "$NATIVE_CARGO_TOML"; then
    echo "::error::Failed to replace all crate paths - original 5-level paths still present"
    echo "  Remaining original paths:"
    grep -F 'path = "../../../../../' "$NATIVE_CARGO_TOML" || true
    exit 1
  fi

  # Validate that both vendored paths are present
  if ! grep -qF 'path = "../../../vendor/kreuzberg"' "$NATIVE_CARGO_TOML"; then
    echo "::error::Vendor path replacement failed - kreuzberg vendor path not found"
    exit 1
  fi
  if ! grep -qF 'path = "../../../vendor/kreuzberg-ffi"' "$NATIVE_CARGO_TOML"; then
    echo "::error::Vendor path replacement failed - kreuzberg-ffi vendor path not found"
    exit 1
  fi

  echo "✓ Updated native extension paths to use vendor directory"
else
  echo "::error::Native Cargo.toml not found at $NATIVE_CARGO_TOML"
  exit 1
fi

# Generate vendor workspace Cargo.toml with extracted versions
cat >"$REPO_ROOT/packages/ruby/vendor/Cargo.toml" <<EOF
[workspace]
members = ["kreuzberg", "kreuzberg-tesseract", "kreuzberg-ffi"]
resolver = "2"

[workspace.package]
version = "${core_version}"
edition = "2024"
rust-version = "1.91"
authors = ["Na'aman Hirschfeld <nhirschfeld@gmail.com>"]
license = "MIT"
repository = "https://github.com/kreuzberg-dev/kreuzberg"
homepage = "https://kreuzberg.dev"

[workspace.dependencies]
# Core async runtime
tokio = { version = "${TOKIO_VERSION}", features = [
    "rt",
    "rt-multi-thread",
    "macros",
    "sync",
    "process",
    "fs",
    "time",
    "io-util",
] }

# Serialization
serde = { version = "${SERDE_VERSION}", features = ["derive"] }
serde_json = "${SERDE_JSON_VERSION}"

# Error handling
thiserror = "${THISERROR_VERSION}"
anyhow = "${ANYHOW_VERSION}"

# Async utilities
async-trait = "${ASYNC_TRAIT_VERSION}"
libc = "${LIBC_VERSION}"
parking_lot = "${PARKING_LOT_VERSION}"

# Tracing/observability
tracing = "${TRACING_VERSION}"

# Utilities
ahash = "${AHASH_VERSION}"
base64 = "${BASE64_VERSION}"
hex = "${HEX_VERSION}"
toml = "${TOML_VERSION}"
num_cpus = "${NUM_CPUS_VERSION}"
once_cell = "${ONCE_CELL_VERSION}"
html-to-markdown-rs = { version = "${HTML_TO_MARKDOWN_VERSION}", default-features = false }
reqwest = { version = "${REQWEST_VERSION}", default-features = false, features = ["json", "rustls"] }
image = { version = "${IMAGE_VERSION}", default-features = false }
lzma-rust2 = { version = "${LZMA_RUST_VERSION}" }

# Fix for WASM builds: ensure getrandom has wasm_js feature enabled
# This is needed because ring/rustls depend on getrandom without the wasm_js feature
getrandom = { version = "${GETRANDOM_VERSION}", features = ["wasm_js"] }

# Testing (dev)
tempfile = "${TEMPFILE_VERSION}"
criterion = { version = "${CRITERION_VERSION}", features = ["html_reports"] }
EOF

echo "Vendoring complete (core version: $core_version)"
echo "Generated vendor workspace with dynamically extracted versions"
echo "Native extension Cargo.toml uses:"
echo "  - path '../../../vendor/kreuzberg' for kreuzberg crate"
echo "  - rb-sys from crates.io"
