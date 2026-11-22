#!/usr/bin/env bash
set -euo pipefail

platform_id="${PLATFORM_ID:-}"
arch_id="${ARCH_ID:-}"
version="${PDFIUM_VERSION:-}"

if [ -z "$platform_id" ]; then
  if [ -n "${PDFIUM_PLATFORM:-}" ]; then
    platform_id="$PDFIUM_PLATFORM"
  else
    platform="$(uname -s)"
    case "$platform" in
    Linux*) platform_id="linux" ;;
    Darwin*) platform_id="mac" ;;
    *)
      echo "Unsupported platform: $platform" >&2
      exit 1
      ;;
    esac
  fi
fi

if [ -z "$arch_id" ]; then
  if [ -n "${PDFIUM_ARCH:-}" ]; then
    arch_id="$PDFIUM_ARCH"
  else
    arch="$(uname -m)"
    case "$arch" in
    x86_64 | amd64) arch_id="x64" ;;
    arm64 | aarch64) arch_id="arm64" ;;
    *)
      echo "Unsupported architecture: $arch" >&2
      exit 1
      ;;
    esac
  fi
fi

if [ -z "$version" ]; then
  version="$(tr -d '[:space:]' <"$(dirname -- "${BASH_SOURCE[0]}")/../../../pdfium-version")"
fi

if [[ ! "$version" =~ ^[0-9]+$ ]]; then
  echo "Invalid PDFium Chromium revision: $version" >&2
  exit 1
fi
if [[ ! "$platform_id" =~ ^[A-Za-z0-9][A-Za-z0-9_-]*$ || ! "$arch_id" =~ ^[A-Za-z0-9][A-Za-z0-9_-]*$ ]]; then
  echo "Invalid PDFium platform or architecture: $platform_id/$arch_id" >&2
  exit 1
fi

repo_root="$(CDPATH='' cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../../../.." && pwd -P)"
asset="pdfium-${platform_id}-${arch_id}.tgz"
archive_name="${version}-${asset}"
verify_archive="$repo_root/scripts/ci/actions/setup-onnx-runtime/verify-archive.sh"
# Supply a trusted manifest containing the revision and asset when overriding the version. ~keep
checksums="${PDFIUM_CHECKSUMS_FILE:-$repo_root/scripts/pdfium-runtime-checksums.sha256}"
bash "$verify_archive" "$archive_name" "$checksums" --known

tmpdir="$(mktemp -d)"
trap 'rm -rf "$tmpdir"' EXIT
archive="$tmpdir/$archive_name"
curl -fL --retry 5 --retry-delay 2 --retry-max-time 180 --retry-all-errors \
  -o "$archive" \
  "https://github.com/bblanchon/pdfium-binaries/releases/download/chromium/${version}/${asset}"
bash "$verify_archive" "$archive" "$checksums"

mkdir -p "$tmpdir/extracted"
tar -xzf "$archive" -C "$tmpdir/extracted"

dest="$RUNNER_TEMP/pdfium-prebuilt"
rm -rf "$dest"
mv "$tmpdir/extracted" "$dest"

echo "pdfium_path=$dest" >>"$GITHUB_OUTPUT"
