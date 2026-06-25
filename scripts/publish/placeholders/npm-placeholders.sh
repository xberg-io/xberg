#!/usr/bin/env bash
#
# Reserve the @xberg/* npm package names with v0.0.1 placeholders.
#
# Why: npm trusted publishing (OIDC) and the scoped @xberg org require each
# package name to exist before it can be configured. Publishing a minimal 0.0.1
# stub reserves the name. CI later publishes the real 1.0.0-rc.1 with the `next`
# dist-tag (prereleases do not move `latest`), so the 0.0.1 placeholder staying
# on `latest` until the first stable release is expected.
#
# Covers the real published surface:
#   - @xberg/node, @xberg/wasm           (top-level entry points)
#   - @xberg/xberg-cli                   (CLI proxy)
#   - 8 napi platform packages           (optionalDependencies of @xberg/node)
#
# Usage:
#   scripts/publish/placeholders/npm-placeholders.sh [--dry-run]
#
# Requirements:
#   - npm on PATH, logged in to the @xberg org (`npm whoami` must succeed)
#
set -euo pipefail

DRY_RUN=0
for arg in "$@"; do
  case "$arg" in
  --dry-run) DRY_RUN=1 ;;
  -h | --help)
    grep '^#' "$0" | sed 's/^# \{0,1\}//'
    exit 0
    ;;
  *)
    echo "unknown argument: $arg" >&2
    exit 2
    ;;
  esac
done

REPO_URL="git+https://github.com/xberg-io/xberg.git"
PLACEHOLDER_VERSION="0.0.1"

log() { printf '\033[1;34m==>\033[0m %s\n' "$*"; }
warn() { printf '\033[1;33mwarn:\033[0m %s\n' "$*" >&2; }

command -v npm >/dev/null 2>&1 || {
  echo "missing required tool: npm" >&2
  exit 1
}

if ! npm whoami >/dev/null 2>&1; then
  warn "not logged in to npm (run 'npm login')."
  [[ "$DRY_RUN" -eq 1 ]] || exit 1
fi

TMPROOT=$(mktemp -d)
trap 'rm -rf "$TMPROOT"' EXIT

# Top-level packages: name only (no platform constraints).
TOP_LEVEL=(
  "@xberg/node"
  "@xberg/wasm"
  "@xberg/xberg-cli"
)

# Platform packages: "name|os|cpu|libc" (libc empty for darwin/win32).
# Mirrors crates/xberg-node/package.json optionalDependencies + npm/<triple>/.
PLATFORMS=(
  "@xberg/node-linux-x64-gnu|linux|x64|glibc"
  "@xberg/node-linux-arm64-gnu|linux|arm64|glibc"
  "@xberg/node-linux-x64-musl|linux|x64|musl"
  "@xberg/node-linux-arm64-musl|linux|arm64|musl"
  "@xberg/node-darwin-x64|darwin|x64|"
  "@xberg/node-darwin-arm64|darwin|arm64|"
  "@xberg/node-win32-x64-msvc|win32|x64|"
  "@xberg/node-win32-arm64-msvc|win32|arm64|"
)

# --- Publish one placeholder package -----------------------------------------
# args: name [os cpu libc]
publish_placeholder() {
  local name="$1" os="${2:-}" cpu="${3:-}" libc="${4:-}"
  local tmp
  tmp=$(mktemp -d "$TMPROOT/XXXXXX")

  {
    printf '{\n'
    printf '  "name": "%s",\n' "$name"
    printf '  "version": "%s",\n' "$PLACEHOLDER_VERSION"
    printf '  "description": "Placeholder — see https://github.com/xberg-io/xberg",\n'
    printf '  "license": "MIT",\n'
    printf '  "repository": { "type": "git", "url": "%s" },\n' "$REPO_URL"
    printf '  "main": "index.js",\n'
    if [[ -n "$os" ]]; then
      printf '  "os": ["%s"],\n' "$os"
      printf '  "cpu": ["%s"],\n' "$cpu"
      [[ -n "$libc" ]] && printf '  "libc": ["%s"],\n' "$libc"
    fi
    printf '  "publishConfig": { "access": "public" }\n'
    printf '}\n'
  } >"$tmp/package.json"

  printf 'module.exports = {};\n' >"$tmp/index.js"
  printf '# %s\n\nPlaceholder. See https://github.com/xberg-io/xberg\n' "$name" >"$tmp/README.md"

  local publish_args=(publish --access public)
  [[ "$DRY_RUN" -eq 1 ]] && publish_args+=(--dry-run)

  log "publishing $name@$PLACEHOLDER_VERSION${DRY_RUN:+ (dry-run)}"
  (cd "$tmp" && npm "${publish_args[@]}")
}

main() {
  # Platform packages first so @xberg/node's optionalDependencies resolve.
  local entry name os cpu libc
  for entry in "${PLATFORMS[@]}"; do
    IFS='|' read -r name os cpu libc <<<"$entry"
    publish_placeholder "$name" "$os" "$cpu" "$libc"
  done
  for name in "${TOP_LEVEL[@]}"; do
    publish_placeholder "$name"
  done
  log "done. $((${#PLATFORMS[@]} + ${#TOP_LEVEL[@]})) placeholder packages processed."
}

main
