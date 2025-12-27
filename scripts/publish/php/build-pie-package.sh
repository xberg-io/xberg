#!/usr/bin/env bash

set -euo pipefail

if [[ $# -lt 2 ]]; then
	echo "Usage: $0 <platform> <output-dir>" >&2
	exit 1
fi

PLATFORM="$1"
OUTPUT_DIR="$2"
VERSION="${VERSION:-unknown}"

echo "::group::Building PIE package for ${PLATFORM}"

case "$PLATFORM" in
linux-x86_64)
	OS="linux"
	ARCH="x86_64"
	EXT_SUFFIX="so"
	;;
macos-arm64)
	OS="macos"
	ARCH="arm64"
	EXT_SUFFIX="dylib"
	;;
windows-x86_64)
	OS="windows"
	ARCH="x86_64"
	EXT_SUFFIX="dll"
	;;
*)
	echo "::error::Unknown platform: ${PLATFORM}" >&2
	exit 1
	;;
esac

echo "Platform: ${PLATFORM}"
echo "OS: ${OS}"
echo "Architecture: ${ARCH}"
echo "Version: ${VERSION}"

mkdir -p "$OUTPUT_DIR"

WORKSPACE="${GITHUB_WORKSPACE:-$(pwd)}"
PHP_DIR="${WORKSPACE}/packages/php"
TARGET_DIR="${WORKSPACE}/target/release"
EXT_FILE="libkreuzberg.${EXT_SUFFIX}"

if [[ ! -f "${TARGET_DIR}/${EXT_FILE}" ]]; then
	echo "::error::Extension file not found: ${TARGET_DIR}/${EXT_FILE}" >&2
	exit 1
fi

PKG_NAME="kreuzberg-${VERSION}-${PLATFORM}"
PKG_DIR="${OUTPUT_DIR}/${PKG_NAME}"
mkdir -p "${PKG_DIR}/ext"

echo "Creating PIE package: ${PKG_NAME}"

cp "${TARGET_DIR}/${EXT_FILE}" "${PKG_DIR}/ext/"

cp "${PHP_DIR}/composer.json" "${PKG_DIR}/"
cp "${PHP_DIR}/package.xml" "${PKG_DIR}/" || echo "::warning::package.xml not found"

# Copy README and LICENSE
cp "${PHP_DIR}/README.md" "${PKG_DIR}/" || echo "::warning::README.md not found"
cp "${PHP_DIR}/LICENSE" "${PKG_DIR}/" || echo "::warning::LICENSE not found"
cp "${PHP_DIR}/CHANGELOG.md" "${PKG_DIR}/" || echo "::warning::CHANGELOG.md not found"

cat >"${PKG_DIR}/pie.json" <<EOF
{
  "name": "kreuzberg",
  "version": "${VERSION}",
  "platform": "${PLATFORM}",
  "os": "${OS}",
  "arch": "${ARCH}",
  "php_version": ">=8.2",
  "extension_file": "ext/${EXT_FILE}",
  "built_at": "$(date -u +"%Y-%m-%dT%H:%M:%SZ")"
}
EOF

cat >"${PKG_DIR}/INSTALL.md" <<EOF
# Installation Instructions

This is a pre-built PIE package for the Kreuzberg PHP extension.

## Platform
- OS: ${OS}
- Architecture: ${ARCH}
- PHP Version: 8.2+

## Installation with PIE (Recommended)

The easiest way to install this extension is using PIE:

\`\`\`bash
pie install kreuzberg/kreuzberg
\`\`\`

PIE will automatically:
- Download and compile the extension
- Install it to the correct location
- Configure your php.ini

## Manual Installation

If you prefer manual installation:

1. Extract this package
2. Copy \`ext/${EXT_FILE}\` to your PHP extension directory
3. Add to your \`php.ini\`:
   \`\`\`ini
   extension=${EXT_FILE}
   \`\`\`
4. Install the Composer package:
   \`\`\`bash
   composer require kreuzberg/kreuzberg
   \`\`\`

## Verification

Verify the extension is loaded:
\`\`\`bash
php -m | grep kreuzberg
\`\`\`

## Support

For issues, visit: https://github.com/kreuzberg-dev/kreuzberg/issues
EOF

TARBALL_NAME="${PKG_NAME}.tar.gz"
echo "Creating tarball: ${TARBALL_NAME}"
tar -czf "${OUTPUT_DIR}/${TARBALL_NAME}" -C "${OUTPUT_DIR}" "${PKG_NAME}"

cd "${OUTPUT_DIR}"
shasum -a 256 "${TARBALL_NAME}" >"${TARBALL_NAME}.sha256"

echo "::notice::PIE package created: ${TARBALL_NAME}"
echo "Package size: $(du -h "${TARBALL_NAME}" | cut -f1)"
echo "SHA256: $(cat "${TARBALL_NAME}.sha256")"

rm -rf "${PKG_DIR}"

echo "::endgroup::"
