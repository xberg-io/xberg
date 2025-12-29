#!/usr/bin/env bash
set -euo pipefail

OPENSSL_ROOT="/usr"
MULTIARCH_TRIPLET="$(dpkg-architecture -qDEB_HOST_MULTIARCH)"
PKG_CONFIG_DIR="/usr/lib/${MULTIARCH_TRIPLET}/pkgconfig"

if [ ! -f "${PKG_CONFIG_DIR}/openssl.pc" ]; then
	echo "ERROR: openssl.pc not found at ${PKG_CONFIG_DIR}" >&2
	echo "Searching for openssl.pc..." >&2
	find /usr -name "openssl.pc" 2>/dev/null || true
	exit 1
fi

{
	echo "OPENSSL_DIR=${OPENSSL_ROOT}"
	echo "OPENSSL_LIB_DIR=/usr/lib/${MULTIARCH_TRIPLET}"
	echo "OPENSSL_INCLUDE_DIR=/usr/include"
	echo "PKG_CONFIG_PATH=${PKG_CONFIG_DIR}:${PKG_CONFIG_PATH:-}"
} >>"$GITHUB_ENV"

echo "OpenSSL configuration completed:"
echo "  OPENSSL_DIR=${OPENSSL_ROOT}"
echo "  PKG_CONFIG_PATH=${PKG_CONFIG_DIR}"
pkg-config --modversion openssl
