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
  patchelf
  cmake
  libmagic-dev
  libuv1-dev
  libde265-dev
  libaom-dev
  libx265-dev
  libdav1d-dev
  libnuma-dev
  # liblzma-dev provides the liblzma.so linker symlink. The swift package
  # statically links libxberg_ffi.a, whose lzma-sys transitive dep surfaces
  # `-llzma` at the swift link step; the runner ships liblzma5 (runtime) but
  # not the dev symlink, so ld.gold fails with "cannot find -llzma". ~keep
  liblzma-dev
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
    for pkg in tesseract-ocr libssl-dev pkg-config cmake; do
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

echo "::group::Building libheif from source (Noble ships 1.17.6, libheif-sys needs >=1.21)"

LIBHEIF_VERSION="${LIBHEIF_VERSION:-1.23.0}"
LIBHEIF_PREFIX="${LIBHEIF_PREFIX:-/usr/local}"

echo "Removing apt's libheif to prevent shadowing..."
if dpkg -l | grep -q "^ii.*libheif"; then
  sudo apt-get remove -y libheif* || echo "::warning::Failed to remove apt libheif, continuing..."
else
  echo "✓ apt libheif not installed"
fi

LIBHEIF_MARKER="$LIBHEIF_PREFIX/lib/pkgconfig/libheif.pc"

if [ -f "$LIBHEIF_MARKER" ] && pkg-config --modversion libheif 2>/dev/null | grep -q "^${LIBHEIF_VERSION}$"; then
  echo "✓ libheif ${LIBHEIF_VERSION} already installed (cached)"
else
  echo "Building libheif ${LIBHEIF_VERSION} from source..."
  build_dir="$(mktemp -d)"
  pushd "$build_dir" >/dev/null

  if retry_with_backoff_timeout 300 curl -fsSL -o libheif.tar.gz \
    "https://github.com/strukturag/libheif/releases/download/v${LIBHEIF_VERSION}/libheif-${LIBHEIF_VERSION}.tar.gz"; then
    tar xzf libheif.tar.gz
    cd "libheif-${LIBHEIF_VERSION}"
    mkdir build
    cd build
    cmake .. \
      -DCMAKE_BUILD_TYPE=Release \
      -DCMAKE_INSTALL_PREFIX="$LIBHEIF_PREFIX" \
      -DCMAKE_INSTALL_LIBDIR=lib \
      -DWITH_EXAMPLES=OFF \
      -DWITH_GDK_PIXBUF=OFF \
      -DBUILD_TESTING=OFF
    make -j"$(nproc)"
    sudo make install
    echo "✓ libheif ${LIBHEIF_VERSION} installed to $LIBHEIF_PREFIX"
  else
    echo "::error::Failed to download libheif source"
    exit 1
  fi

  popd >/dev/null
  rm -rf "$build_dir"
fi

sudo ldconfig

if [ -n "${GITHUB_ACTION:-}" ]; then
  mkdir -p /tmp/libheif-cache/usr/local/lib/pkgconfig
  mkdir -p /tmp/libheif-cache/usr/local/include
  mkdir -p /tmp/libheif-cache/usr/local/share
  cp -a /usr/local/lib/libheif* /tmp/libheif-cache/usr/local/lib/ 2>/dev/null || true
  cp -a /usr/local/lib/pkgconfig/libheif.pc /tmp/libheif-cache/usr/local/lib/pkgconfig/ 2>/dev/null || true
  [ -d /usr/local/include/libheif ] && cp -a /usr/local/include/libheif /tmp/libheif-cache/usr/local/include/
  [ -d /usr/local/share/libheif ] && cp -a /usr/local/share/libheif /tmp/libheif-cache/usr/local/share/
fi

echo ""
echo "libheif symbol verification:"
if [ -f "$LIBHEIF_PREFIX/lib/libheif.so.1" ]; then
  if nm -D "$LIBHEIF_PREFIX/lib/libheif.so.1" 2>/dev/null | grep -q "heif_image_get_plane_readonly2"; then
    echo "✓ $LIBHEIF_PREFIX/lib/libheif.so.1 has heif_image_get_plane_readonly2"
  else
    echo "::warning::$LIBHEIF_PREFIX/lib/libheif.so.1 missing heif_image_get_plane_readonly2"
  fi
else
  echo "::warning::$LIBHEIF_PREFIX/lib/libheif.so.1 not found"
fi

echo "ldconfig cache contains:"
ldconfig -p | grep libheif || echo "(no libheif in ldconfig cache)"

if [[ -n "${GITHUB_ENV:-}" ]]; then
  echo "PKG_CONFIG_PATH=$LIBHEIF_PREFIX/lib/pkgconfig:${PKG_CONFIG_PATH:-}" >>"$GITHUB_ENV"
  echo "LD_LIBRARY_PATH=$LIBHEIF_PREFIX/lib:${LD_LIBRARY_PATH:-}" >>"$GITHUB_ENV"
fi

echo "::endgroup::"

echo "::group::Verifying Linux installations"

echo "CMake:"
if command -v cmake >/dev/null 2>&1; then
  cmake --version | head -1
  echo "✓ CMake available"
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
  echo "::error::CMake not found after installation"
  exit 1
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

echo "::endgroup::"
