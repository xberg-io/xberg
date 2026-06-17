#!/usr/bin/env bash
set -euo pipefail

tarball="${1:?package tarball (file or directory) required}"

if [[ -n "${KREUZBERG_PDFIUM_PREBUILT:-}" ]]; then
  case "${RUNNER_OS:-unknown}" in
  Linux) export LD_LIBRARY_PATH="$KREUZBERG_PDFIUM_PREBUILT/lib:${LD_LIBRARY_PATH:-}" ;;
  macOS) export DYLD_LIBRARY_PATH="$KREUZBERG_PDFIUM_PREBUILT/lib:${DYLD_LIBRARY_PATH:-}" ;;
  Windows) export PATH="$KREUZBERG_PDFIUM_PREBUILT/bin;${PATH:-}" ;;
  esac
fi

if [[ -d "$tarball" ]]; then
  tarball="$(find "$tarball" -name "*.tgz" -type f | head -n 1)"
  [ -n "$tarball" ] || {
    echo "No .tgz file found in directory" >&2
    exit 1
  }
fi

if [[ "$tarball" != /* ]]; then
  tarball="${GITHUB_WORKSPACE}/$tarball"
fi

echo "Using tarball: $tarball"
tmp="$(mktemp -d)"
cp -R e2e/smoke/node/. "$tmp"/
pushd "$tmp" >/dev/null
cp "$tarball" ./kreuzberg-node.tgz
cp -R "${GITHUB_WORKSPACE}/crates/kreuzberg-node" ./kreuzberg-node-pkg

node -e "
  const fs = require('fs');
  const pkg = JSON.parse(fs.readFileSync('kreuzberg-node-pkg/package.json', 'utf8'));
  fs.writeFileSync('kreuzberg-node-pkg/package.json', JSON.stringify(pkg, null, 2) + '\n');
  const smokePkg = JSON.parse(fs.readFileSync('package.json', 'utf8'));
  smokePkg.dependencies ||= {};
  smokePkg.dependencies['@kreuzberg/node'] = 'file:./kreuzberg-node-pkg';
  fs.writeFileSync('package.json', JSON.stringify(smokePkg, null, 2) + '\n');
"

pushd kreuzberg-node-pkg >/dev/null
pnpm install --no-frozen-lockfile
pnpm build:ts
popd >/dev/null

rm -f pnpm-lock.yaml
pnpm install --no-frozen-lockfile
pnpm run check
popd >/dev/null
echo "✓ Node package smoke test passed"
