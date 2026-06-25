#!/usr/bin/env bash
#
# Reserve the xberg crate names on crates.io with v0.0.1 placeholders.
#
# Why: trusted publishing (GitHub OIDC) can only be configured for a crate that
# already exists on the registry. Publishing a minimal 0.0.1 stub reserves the
# name and gives a concrete version to attach the trusted publisher to. CI later
# publishes the real 1.0.0-rc.1 (> 0.0.1) via .github/workflows/publish.yaml.
#
# It also yanks the legacy 5.0.0-rc.* alias versions of `xberg` (the old
# Kreuzberg-alias crate) so the v1 line launches clean. Yank only hides a
# version from new resolution; it does not delete it, and is reversible with
# `cargo unyank`.
#
# Usage:
#   scripts/publish/placeholders/crates-placeholders.sh [--dry-run] [--no-yank]
#
# Requirements:
#   - cargo on PATH
#   - authenticated: either `cargo login` already run, or CARGO_REGISTRY_TOKEN set
#   - jq + curl (only needed for the yank step)
#
set -euo pipefail

DRY_RUN=0
DO_YANK=1
for arg in "$@"; do
  case "$arg" in
  --dry-run) DRY_RUN=1 ;;
  --no-yank) DO_YANK=0 ;;
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

REPO_URL="https://github.com/xberg-io/xberg"
PLACEHOLDER_VERSION="0.0.1"

# Dependency order: leaf component crates first, then the umbrella `xberg`, then
# the CLI. crates.io requires every dependency of a crate to already be on the
# registry, so leaves must reserve their names before `xberg` does.
CRATES=(
  xberg-tesseract
  xberg-libheif
  xberg-paddle-ocr
  xberg-candle-ocr
  xberg-gliner
  xberg
  xberg-cli
)

log() { printf '\033[1;34m==>\033[0m %s\n' "$*"; }
warn() { printf '\033[1;33mwarn:\033[0m %s\n' "$*" >&2; }

require() {
  command -v "$1" >/dev/null 2>&1 || {
    echo "missing required tool: $1" >&2
    exit 1
  }
}

require cargo

TMPROOT=$(mktemp -d)
trap 'rm -rf "$TMPROOT"' EXIT

if [[ -z "${CARGO_REGISTRY_TOKEN:-}" ]] && [[ ! -f "${CARGO_HOME:-$HOME/.cargo}/credentials.toml" ]]; then
  warn "no CARGO_REGISTRY_TOKEN and no cargo credentials found."
  warn "run 'cargo login' or export CARGO_REGISTRY_TOKEN before a real publish."
  [[ "$DRY_RUN" -eq 1 ]] || exit 1
fi

# --- Yank legacy xberg 5.0.0-rc.* alias versions -----------------------------
yank_legacy_xberg() {
  require jq
  require curl
  log "querying crates.io for existing xberg versions"
  local versions
  versions=$(curl -fsSL "https://crates.io/api/v1/crates/xberg" |
    jq -r '.versions[].num' 2>/dev/null || true)
  if [[ -z "$versions" ]]; then
    warn "could not list xberg versions (crate may not exist yet); skipping yank"
    return 0
  fi
  local v
  while IFS= read -r v; do
    [[ "$v" == 5.* ]] || continue
    if [[ "$DRY_RUN" -eq 1 ]]; then
      log "[dry-run] cargo yank --version $v xberg"
    else
      log "yanking xberg@$v"
      cargo yank --version "$v" xberg || warn "yank failed for $v (already yanked?)"
    fi
  done <<<"$versions"
}

# --- Publish one placeholder crate -------------------------------------------
publish_placeholder() {
  local name="$1"
  local tmp
  tmp=$(mktemp -d "$TMPROOT/XXXXXX")

  local is_bin=0
  [[ "$name" == "xberg-cli" ]] && is_bin=1

  cat >"$tmp/Cargo.toml" <<EOF
[package]
name = "$name"
version = "$PLACEHOLDER_VERSION"
edition = "2021"
license = "MIT"
description = "Placeholder for $name; see $REPO_URL"
repository = "$REPO_URL"

EOF

  mkdir -p "$tmp/src"
  if [[ "$is_bin" -eq 1 ]]; then
    printf 'fn main() {}\n' >"$tmp/src/main.rs"
    printf '[[bin]]\nname = "%s"\npath = "src/main.rs"\n' "$name" >>"$tmp/Cargo.toml"
  else
    printf '//! Placeholder crate. See %s\n' "$REPO_URL" >"$tmp/src/lib.rs"
  fi

  local publish_args=(publish --allow-dirty --manifest-path "$tmp/Cargo.toml")
  [[ "$DRY_RUN" -eq 1 ]] && publish_args+=(--dry-run)

  log "publishing $name@$PLACEHOLDER_VERSION${DRY_RUN:+ (dry-run)}"
  (cd "$tmp" && cargo "${publish_args[@]}")
}

main() {
  if [[ "$DO_YANK" -eq 1 ]]; then
    yank_legacy_xberg
  fi
  for crate in "${CRATES[@]}"; do
    publish_placeholder "$crate"
    # crates.io needs a moment to index a new crate before a dependent can
    # reference it. Harmless for independent leaves; required before `xberg`.
    if [[ "$DRY_RUN" -eq 0 ]]; then
      sleep 15
    fi
  done
  log "done. ${#CRATES[@]} placeholder crates processed."
}

main
