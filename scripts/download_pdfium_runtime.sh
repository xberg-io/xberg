#!/usr/bin/env bash
set -euo pipefail

# 7678, because that is the binding set compiled into the crate, not because it is newest
# (bblanchon publishes far higher). The constraint is the BINDINGS, not the binary:
# `crates/xberg-pdfium-render/src/lib.rs` hardcodes `include!("bindgen/pdfium_7678.rs")` and
# `src/bindgen/` holds exactly that one file, so 7678 IS the compiled ABI contract. A binary whose
# exported symbols do not match it fails at run time, which is exactly how the vendored fork rotted
# (symbols were hand-added and hand-removed to chase drift).
#
# This default read 7881 from 2026-08-21 until GH#1882. Nothing in the tree ever had 7881 bindings,
# so every published image and released musl CLI staged a runtime one release ahead of the bindings
# it was loaded against. It survived because "verified" meant the download succeeded -- the exact
# thing the next sentence forbids.
#
# Bump it only together with a regenerated binding set at the same version, and verify by loading
# the library and resolving symbols -- never by the download alone, which succeeds regardless.
# `DynamicPdfiumBindings::new` probes all 446 required symbols, so a missing one fails cleanly; a
# symbol that is still present but whose signature or struct layout changed is NOT detected
# (`build.rs` sets `layout_tests(false)`), and that is the case worth fearing. ~keep
version="${PDFIUM_VERSION:-${1:-7678}}"
root_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
dest_release="$root_dir/target/release"
dest_debug="$root_dir/target/debug"

platform="$(uname -s)"
case "$platform" in
Linux*) platform_id="linux" ;;
Darwin*) platform_id="mac" ;;
MINGW* | MSYS* | CYGWIN*) platform_id="win" ;;
*)
  echo "Unsupported platform: $platform" >&2
  exit 1
  ;;
esac

arch="$(uname -m)"
case "$arch" in
x86_64 | amd64) arch_id="x64" ;;
arm64 | aarch64) arch_id="arm64" ;;
*)
  echo "Unsupported architecture: $arch" >&2
  exit 1
  ;;
esac

tmpdir="$(mktemp -d)"
trap 'rm -rf "$tmpdir"' EXIT
echo "Downloading Pdfium ${version} for ${platform_id}/${arch_id}..."
curl -fsSL -o "$tmpdir/pdfium.tgz" "https://github.com/bblanchon/pdfium-binaries/releases/download/chromium/${version}/pdfium-${platform_id}-${arch_id}.tgz"
mkdir -p "$tmpdir/extracted"
tar -xzf "$tmpdir/pdfium.tgz" -C "$tmpdir/extracted"

src_lib="$tmpdir/extracted/lib"
if [[ ! -d "$src_lib" ]]; then
  echo "Pdfium archive did not contain lib directory" >&2
  exit 1
fi

mkdir -p "$dest_release" "$dest_debug"
cp -a "$src_lib/." "$dest_release/"
cp -a "$src_lib/." "$dest_debug/"

echo "Pdfium runtime staged to:"
echo "  $dest_release"
echo "  $dest_debug"

rm -rf "$tmpdir"
