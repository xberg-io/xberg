#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="${REPO_ROOT:-$(cd "$SCRIPT_DIR/../../.." && pwd)}"

source "$REPO_ROOT/scripts/lib/retry.sh"

echo "::group::Installing macOS dependencies"

if [[ -d "/opt/homebrew/bin" ]]; then
  export PATH="/opt/homebrew/bin:/opt/homebrew/sbin:${PATH}"
fi
if [[ -d "/usr/local/bin" ]]; then
  export PATH="/usr/local/bin:/usr/local/sbin:${PATH}"
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

if [ -d "/Applications/LibreOffice.app" ]; then
  echo "✓ LibreOffice already present"
else
  echo "Installing LibreOffice (this may take 10+ minutes, timeout: 20min)..."
  if retry_with_backoff_timeout 1200 brew install --cask libreoffice; then
    echo "✓ LibreOffice installed successfully"
  else
    exit_code=$?
    if [ $exit_code -eq 124 ]; then
      echo "::error::LibreOffice installation timed out after 20 minutes"
    else
      echo "::error::LibreOffice installation failed with exit code $exit_code"
    fi
    exit 1
  fi
fi

echo "::endgroup::"

echo "::group::Verifying macOS installations"

echo "CMake:"
if command -v cmake >/dev/null 2>&1; then
  cmake --version | head -1
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
echo "LibreOffice:"
soffice --version 2>/dev/null || echo "⚠ Warning: soffice not fully available"

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
