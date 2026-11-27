#!/usr/bin/env bash
#
# Reserve 0.0.1 placeholder releases for the xberg npm integration packages.
#
# Purpose: reserve each scoped name and configure its npm trusted publisher
# BEFORE the functional package (version-locked to xberg) ships via CI.
#
# Run this yourself — `npm publish` needs an interactive `npm login` and 2FA/OTP
# which a headless agent can't provide. Each placeholder is built in a temp dir,
# so the real package sources are never published.
#
# Usage:
#   npm login                                                   # if not authenticated
#   integrations/scripts/publish-placeholders.sh                # all packages
#   integrations/scripts/publish-placeholders.sh @xberg-io/langchain-xberg   # one
# ~keep
set -euo pipefail

VERSION="0.0.1"

# name|description — one entry per npm integration package. ~keep
PACKAGES=(
  "@xberg-io/n8n-nodes-xberg|n8n community node for Xberg document extraction"
  "@xberg-io/langchain-xberg|LangChain.js document loader for Xberg document extraction"
)

publish_one() {
  local pkg="$1" desc="$2" tmp
  if npm view "${pkg}@${VERSION}" version >/dev/null 2>&1; then
    echo "skip ${pkg}@${VERSION} (already published)"
    return 0
  fi
  tmp="$(mktemp -d)"
  trap 'rm -rf "$tmp"' RETURN

  cat >"$tmp/package.json" <<JSON
{
  "name": "${pkg}",
  "version": "${VERSION}",
  "description": "${desc} (placeholder release).",
  "license": "MIT",
  "homepage": "https://github.com/xberg-io/xberg",
  "publishConfig": { "access": "public" }
}
JSON

  cat >"$tmp/README.md" <<MD
# ${pkg}

**Placeholder release.** This 0.0.1 reserves the name; the functional package
publishes in lockstep with [Xberg](https://github.com/xberg-io/xberg).
MD

  echo "Publishing ${pkg}@${VERSION} (placeholder) to npm..."
  (cd "$tmp" && npm publish --access public)
}

targets=("$@")
if [ "${#targets[@]}" -eq 0 ]; then
  for entry in "${PACKAGES[@]}"; do
    publish_one "${entry%%|*}" "${entry#*|}"
  done
else
  for want in "${targets[@]}"; do
    found=""
    for entry in "${PACKAGES[@]}"; do
      if [ "${entry%%|*}" = "$want" ]; then
        publish_one "${entry%%|*}" "${entry#*|}"
        found=1
      fi
    done
    [ -n "$found" ] || {
      echo "unknown package: $want" >&2
      exit 1
    }
  done
fi

echo
echo "Done. Configure the npm trusted publisher for each reserved name, then"
echo "future releases publish over the top via CI (.github/workflows/publish.yaml)."
