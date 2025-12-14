#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="${REPO_ROOT:-$(cd "$SCRIPT_DIR/../../.." && pwd)}"

source "$REPO_ROOT/scripts/lib/retry.sh"

echo "::group::Installing macOS dependencies"

if ! brew list tesseract &>/dev/null; then
	echo "Installing Tesseract..."
	retry_with_backoff brew install tesseract || {
		echo "::error::Failed to install Tesseract after retries"
		exit 1
	}
else
	echo "✓ Tesseract already installed"
fi

if ! brew list tesseract-lang &>/dev/null; then
	echo "Installing Tesseract language packs..."
	retry_with_backoff brew install tesseract-lang || {
		echo "::warning::Failed to install tesseract-lang, some languages may be unavailable"
	}
else
	echo "✓ Tesseract language packs already installed"
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

echo "Tesseract:"
tesseract --version | head -1

echo ""
echo "Available languages:"
tesseract --list-langs | head -5

echo ""
echo "LibreOffice:"
soffice --version 2>/dev/null || echo "⚠ Warning: soffice not fully available"

echo "::endgroup::"
