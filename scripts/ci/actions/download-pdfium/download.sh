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
  echo "PDFIUM_VERSION env var required" >&2
  exit 2
fi

tmpdir="$(mktemp -d)"
curl -fL --retry 5 --retry-delay 2 --retry-max-time 180 --retry-all-errors \
  -o "$tmpdir/pdfium.tgz" \
  "https://github.com/bblanchon/pdfium-binaries/releases/download/chromium/${version}/pdfium-${platform_id}-${arch_id}.tgz"

mkdir -p "$tmpdir/extracted"
tar -xzf "$tmpdir/pdfium.tgz" -C "$tmpdir/extracted"

dest="$RUNNER_TEMP/pdfium-prebuilt"
rm -rf "$dest"
mv "$tmpdir/extracted" "$dest"
rm -rf "$tmpdir"

echo "pdfium_path=$dest" >>"$GITHUB_OUTPUT"
