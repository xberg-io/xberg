#!/usr/bin/env bash
set -euo pipefail

echo "=== Installing OpenSSL and pkg-config ==="
echo "Platform: $(uname -m)"

if brew list openssl@3 &>/dev/null; then
	echo "openssl@3 already installed"
else
	echo "Installing openssl@3..."
	brew install openssl@3
fi

if brew list pkg-config &>/dev/null; then
	echo "pkg-config already installed"
else
	echo "Installing pkg-config..."
	brew install pkg-config
fi

echo ""
echo "=== Verification ==="
brew --prefix openssl@3 && echo "✓ openssl@3 prefix found" || echo "✗ openssl@3 prefix not found"
ls -la "$(brew --prefix openssl@3)/lib" 2>/dev/null && echo "✓ openssl@3 lib directory accessible" || echo "✗ openssl@3 lib directory not accessible"
