#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="${REPO_ROOT:-$(cd "$SCRIPT_DIR/../../.." && pwd)}"

source "$REPO_ROOT/scripts/lib/retry.sh"

echo "::group::Installing Linux dependencies"

echo "Updating package index..."
if ! retry_with_backoff sudo apt-get update; then
  echo "::warning::apt-get update failed after retries, continuing anyway..."
fi

packages=(
  libreoffice
  libreoffice-writer
  libreoffice-calc
  libreoffice-impress
  tesseract-ocr
  tesseract-ocr-eng
  tesseract-ocr-tur
  tesseract-ocr-deu
  fonts-liberation
  fonts-dejavu-core
  fonts-noto-core
  libssl-dev
  pkg-config
  build-essential
  cmake
  php-cli
  php-dev
)

echo "Installing dependencies..."
if retry_with_backoff_timeout 900 sudo apt-get install -y "${packages[@]}"; then
  echo "✓ All packages installed successfully"
else
  exit_code=$?
  if [ $exit_code -eq 124 ]; then
    echo "::error::Package installation timed out after 15 minutes"
  else
    echo "::warning::Some packages failed to install, attempting individual installs..."
    for pkg in tesseract-ocr libreoffice libssl-dev pkg-config cmake; do
      echo "Installing $pkg..."
      if retry_with_backoff_timeout 300 sudo apt-get install -y "$pkg" 2>&1; then
        echo "  ✓ $pkg installed"
      else
        echo "  ⚠ Failed to install $pkg"
      fi
    done
  fi
fi

echo "::endgroup::"

echo "::group::Verifying Linux installations"

echo "CMake:"
if command -v cmake >/dev/null 2>&1; then
  cmake --version | head -1
  echo "✓ CMake available"
else
  echo "::error::CMake not found after installation"
  exit 1
fi

echo ""
echo "LibreOffice:"
if soffice --version 2>/dev/null; then
  echo "✓ LibreOffice available"
else
  echo "⚠ Warning: LibreOffice not fully available"
fi

echo ""
echo "Tesseract:"
if command -v tesseract >/dev/null 2>&1; then
  if tesseract --version 2>/dev/null | head -1; then
    echo "✓ Tesseract CLI available"
  else
    echo "::warning::Tesseract CLI present but failed to run"
  fi
else
  echo "::warning::Tesseract CLI not found; continuing (OCR will rely on bundled Tesseract)"
fi

echo ""
echo "Available Tesseract languages:"
if command -v tesseract >/dev/null 2>&1; then
  tesseract --list-langs | head -10 || true
else
  echo "(tesseract CLI not available)"
fi

echo ""
echo "PHP:"
if command -v php >/dev/null 2>&1; then
  php --version | head -1
  echo "✓ PHP available"
else
  echo "::error::PHP not found after installation"
  exit 1
fi

echo ""
echo "Checking Tesseract data path..."

tessdata_found=0
for tessdata_path in "/usr/share/tesseract-ocr/5/tessdata" "/usr/share/tesseract-ocr/tessdata"; do
  if [ -d "$tessdata_path" ]; then
    echo "Found tessdata at: $tessdata_path"

    echo "Required language files:"
    for lang in eng tur deu; do
      if [ -f "$tessdata_path/${lang}.traineddata" ]; then
        size=$(stat -c%s "$tessdata_path/${lang}.traineddata" 2>/dev/null || echo "unknown")
        echo "  ✓ ${lang}.traineddata ($size bytes)"
      else
        echo "  ⚠ ${lang}.traineddata (missing)"
      fi
    done
    tessdata_found=1
    break
  fi
done

if [ $tessdata_found -eq 0 ]; then
  echo "::error::Tessdata directory not found in standard locations"
  exit 1
fi

echo ""
echo "Testing LibreOffice headless mode..."
if run_with_timeout 30 soffice --headless --version >/dev/null 2>&1; then
  echo "✓ LibreOffice headless mode works"
else
  echo "⚠ Warning: LibreOffice headless test failed (may still work)"
fi

echo "::endgroup::"
