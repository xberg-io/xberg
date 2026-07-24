#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="${REPO_ROOT:-$(cd "$SCRIPT_DIR/../../.." && pwd)}"

source "$REPO_ROOT/scripts/lib/retry.sh"

echo "::group::Installing macOS dependencies"

if [[ -d "/opt/homebrew/bin" ]]; then
  export PATH="/opt/homebrew/bin:/opt/homebrew/sbin:${PATH}"
  echo "/opt/homebrew/bin" >>"$GITHUB_PATH"
  echo "/opt/homebrew/sbin" >>"$GITHUB_PATH"
fi
if [[ -d "/usr/local/bin" ]]; then
  export PATH="/usr/local/bin:/usr/local/sbin:${PATH}"
  echo "/usr/local/bin" >>"$GITHUB_PATH"
  echo "/usr/local/sbin" >>"$GITHUB_PATH"
fi

if ! brew list cmake &>/dev/null; then
  echo "Installing CMake..."
  retry_with_backoff brew install cmake || {
    echo "::error::Failed to install CMake after retries"
    exit 1
  }
else
  echo "✓ CMake already installed"
fi

if ! command -v cmake >/dev/null 2>&1; then
  echo "CMake not on PATH after install; attempting brew link..."
  brew link --overwrite cmake >/dev/null 2>&1 || true
fi

if ! brew list tesseract &>/dev/null; then
  echo "Installing Tesseract..."
  retry_with_backoff brew install tesseract || {
    echo "::error::Failed to install Tesseract after retries"
    exit 1
  }
else
  echo "✓ Tesseract already installed"
fi

if ! command -v tesseract >/dev/null 2>&1; then
  echo "Tesseract not on PATH after install; attempting brew link..."
  brew link --overwrite tesseract >/dev/null 2>&1 || true
fi

if ! brew list tesseract-lang &>/dev/null; then
  echo "Installing Tesseract language packs..."
  retry_with_backoff brew install tesseract-lang || {
    echo "::warning::Failed to install tesseract-lang, some languages may be unavailable"
  }
else
  echo "✓ Tesseract language packs already installed"
fi

if ! brew list libmagic &>/dev/null; then
  echo "Installing libmagic..."
  retry_with_backoff brew install libmagic || {
    echo "::warning::Failed to install libmagic after retries"
  }
else
  echo "✓ libmagic already installed"
fi

if ! brew list libheif &>/dev/null; then
  echo "Installing libheif..."
  retry_with_backoff brew install libheif || {
    echo "::warning::Failed to install libheif after retries"
  }
else
  echo "✓ libheif already installed"
fi

if ! brew list boost &>/dev/null; then
  echo "Installing boost (build-time header dep of librevenge + libwpd)..."
  retry_with_backoff brew install boost || {
    echo "::warning::Failed to install boost after retries"
  }
else
  echo "✓ boost already installed"
fi

if ! brew list pkg-config &>/dev/null; then
  echo "Installing pkg-config..."
  retry_with_backoff brew install pkg-config || {
    echo "::error::Failed to install pkg-config after retries"
    exit 1
  }
else
  echo "✓ pkg-config already installed"
fi

if ! brew list php &>/dev/null; then
  echo "Installing PHP..."
  retry_with_backoff brew install php || {
    echo "::error::Failed to install PHP after retries"
    exit 1
  }
else
  echo "✓ PHP already installed"
fi

if ! command -v php >/dev/null 2>&1; then
  echo "PHP not on PATH after install; attempting brew link..."
  brew link --overwrite php >/dev/null 2>&1 || true
fi

echo "::endgroup::"

echo "::group::Verifying macOS installations"

echo "CMake:"
if command -v cmake >/dev/null 2>&1; then
  cmake --version | head -1
  CMAKE_FULL_PATH="$(command -v cmake)"
  if [[ -n "$GITHUB_ENV" ]]; then
    echo "CMAKE=$CMAKE_FULL_PATH" >>"$GITHUB_ENV"
    echo "✓ Set CMAKE=$CMAKE_FULL_PATH in GITHUB_ENV"
  fi
  CMAKE_BIN="$(dirname "$CMAKE_FULL_PATH")"
  if [[ -n "$GITHUB_PATH" && -d "$CMAKE_BIN" ]]; then
    echo "$CMAKE_BIN" >>"$GITHUB_PATH"
    echo "✓ Added cmake directory to GITHUB_PATH: $CMAKE_BIN"
  fi
else
  echo "::error::CMake not found on PATH after installation"
  echo "PATH=$PATH"
  brew --prefix cmake 2>/dev/null || true
  exit 1
fi

echo ""
echo "Tesseract:"
if command -v tesseract >/dev/null 2>&1; then
  tesseract --version | head -1
else
  echo "::error::Tesseract not found on PATH after installation"
  echo "PATH=$PATH"
  brew --prefix tesseract 2>/dev/null || true
  exit 1
fi

echo ""
echo "Available languages:"
tesseract --list-langs | head -5

echo ""
echo "pkg-config:"
if command -v pkg-config >/dev/null 2>&1; then
  pkg-config --version
  echo "✓ pkg-config available"
else
  echo "::error::pkg-config not found on PATH after installation"
  echo "PATH=$PATH"
  exit 1
fi

echo ""
echo "PHP:"
if command -v php >/dev/null 2>&1; then
  php --version | head -1
else
  echo "::error::PHP not found on PATH after installation"
  echo "PATH=$PATH"
  exit 1
fi

echo "::endgroup::"
