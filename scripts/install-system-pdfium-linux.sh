#!/usr/bin/env bash

#   ./scripts/install-system-pdfium-linux.sh [PREFIX] [PDFIUM_VERSION]

set -euo pipefail

PREFIX="${1:-${PREFIX:-/usr/local}}"
PDFIUM_VERSION="${2:-${PDFIUM_VERSION:-7529}}"
PDFIUM_PLATFORM="linux-x64"
DOWNLOAD_URL="https://github.com/bblanchon/pdfium-binaries/releases/download/chromium/${PDFIUM_VERSION}/pdfium-${PDFIUM_PLATFORM}.tgz"

TEMP_DIR=$(mktemp -d)
trap 'rm -rf "${TEMP_DIR}"' EXIT

echo "Installing PDFium ${PDFIUM_VERSION} to ${PREFIX}..."
echo "Platform: ${PDFIUM_PLATFORM}"
echo "Download URL: ${DOWNLOAD_URL}"
echo ""

echo "[1/5] Downloading PDFium ${PDFIUM_VERSION}..."
if ! curl -f -L --progress-bar "${DOWNLOAD_URL}" -o "${TEMP_DIR}/pdfium.tgz"; then
	echo "Error: Failed to download PDFium ${PDFIUM_VERSION}" >&2
	exit 1
fi

echo "[2/5] Extracting PDFium archive..."
if ! tar -xzf "${TEMP_DIR}/pdfium.tgz" -C "${TEMP_DIR}"; then
	echo "Error: Failed to extract PDFium archive" >&2
	exit 1
fi

echo "[3/5] Installing shared library to ${PREFIX}/lib/..."
sudo mkdir -p "${PREFIX}/lib"
if [ ! -f "${TEMP_DIR}/lib/libpdfium.so" ]; then
	echo "Error: libpdfium.so not found in archive" >&2
	exit 1
fi
sudo cp "${TEMP_DIR}/lib/libpdfium.so" "${PREFIX}/lib/libpdfium.so"
sudo chmod 0755 "${PREFIX}/lib/libpdfium.so"

echo "   Running ldconfig to update library cache..."
sudo ldconfig

echo "[4/5] Installing headers to ${PREFIX}/include/pdfium/..."
sudo mkdir -p "${PREFIX}/include/pdfium"
if [ ! -d "${TEMP_DIR}/include" ]; then
	echo "Error: include directory not found in archive" >&2
	exit 1
fi
sudo cp -r "${TEMP_DIR}/include/"* "${PREFIX}/include/pdfium/"

echo "[5/5] Creating pkg-config file..."
PKG_CONFIG_DIR="${PREFIX}/lib/pkgconfig"
sudo mkdir -p "${PKG_CONFIG_DIR}"

cat <<EOF | sudo tee "${PKG_CONFIG_DIR}/pdfium.pc" >/dev/null
prefix=${PREFIX}
exec_prefix=\${prefix}
libdir=\${exec_prefix}/lib
includedir=\${prefix}/include/pdfium

Name: PDFium
Description: PDF rendering library
Version: ${PDFIUM_VERSION}
Libs: -L\${libdir} -lpdfium
Cflags: -I\${includedir}
EOF

echo ""
echo "Verifying installation..."
if ! pkg-config --modversion pdfium >/dev/null 2>&1; then
	echo "Warning: pkg-config verification failed. You may need to add ${PKG_CONFIG_DIR} to PKG_CONFIG_PATH" >&2
	echo "   export PKG_CONFIG_PATH=\"${PKG_CONFIG_PATH:-}:${PKG_CONFIG_DIR}\"" >&2
else
	INSTALLED_VERSION=$(pkg-config --modversion pdfium)
	echo "Successfully installed PDFium ${INSTALLED_VERSION}"
fi

echo ""
echo "Installation complete!"
echo ""
echo "Configuration details:"
echo "  Library: ${PREFIX}/lib/libpdfium.so"
echo "  Headers: ${PREFIX}/include/pdfium/"
echo "  pkg-config: ${PKG_CONFIG_DIR}/pdfium.pc"
echo ""
echo "To use PDFium in your build system:"
echo ""
echo "  Rust (in build.rs):"
echo "    pkg_config::probe_library(\"pdfium\").unwrap();"
echo ""
echo "  C/C++ (with pkg-config):"
echo "    gcc -o app app.c \$(pkg-config --cflags --libs pdfium)"
echo ""
echo "  CMake:"
echo "    find_package(PkgConfig REQUIRED)"
echo "    pkg_check_modules(PDFIUM REQUIRED pdfium)"
echo "    target_link_libraries(app \${PDFIUM_LIBRARIES})"
echo "    target_include_directories(app PUBLIC \${PDFIUM_INCLUDE_DIRS})"
