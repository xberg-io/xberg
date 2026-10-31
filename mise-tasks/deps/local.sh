#!/usr/bin/env bash
#MISE description="Update or upgrade the dependencies of the ecosystems with xberg-specific steps"
#MISE hide=true
#USAGE arg "<mode>" help="update stays within major versions; upgrade takes the latest, breaking ones included"
#USAGE arg "[lang]" default="all" help="rust, python, node, ruby, java or zig; all refreshes every one"
# shellcheck disable=SC2154 # usage_* are set by mise from the #USAGE lines
set -euo pipefail

# The shared toolchains deps engine (the `deps` task) covers the other ecosystems. These keep
# steps it cannot express, so they stay here. ~keep
mode="$usage_mode"
case "$mode" in
update | upgrade) ;;
*)
  echo "mode must be update or upgrade, got: $mode" >&2
  exit 2
  ;;
esac

refresh_rust() {
  local manifest
  if [ "$mode" = update ]; then
    cargo update
    # Standalone crates keep their OWN Cargo.lock (they are NOT workspace members), so the root
    # `cargo update` never reaches them. Refresh each so a sub-lock cannot drift behind the
    # workspace (e.g. a stale `cc` pin) and break only the e2e / native-binding builds while the
    # --locked workspace build stays green. ~keep
    for manifest in packages/ruby/ext/xberg_rb/native/Cargo.toml \
      packages/elixir/native/xberg_nif/Cargo.toml crates/xberg-tesseract/Cargo.toml e2e/rust/Cargo.toml; do
      cargo update --manifest-path "$manifest" || true
    done
    return
  fi
  # Public registry getters expose parking_lot 0.12 lock types; 0.13 changes their identity. ~keep
  cargo upgrade --incompatible --exclude parking_lot
  cargo update
  # Workspace members are covered above; upgrading one by --manifest-path rewrites the root
  # manifest again. List only excluded/standalone workspaces. ~keep
  for manifest in packages/ruby/ext/xberg_rb/native/Cargo.toml packages/elixir/native/xberg_nif/Cargo.toml; do
    { cargo upgrade --incompatible --exclude parking_lot --manifest-path "$manifest" &&
      cargo update --manifest-path "$manifest"; } || true
  done
  # e2e/rust is generated -- do NOT rewrite its Cargo.toml; only refresh its lock so its pins
  # track the workspace and the e2e build stays resolvable. ~keep
  cargo update --manifest-path e2e/rust/Cargo.toml || true
}

refresh_python() {
  if [ "$mode" = update ]; then
    (cd packages/python && uv sync --no-install-project --no-install-workspace --all-extras --upgrade-package "*")
    return
  fi
  (cd packages/python && uv sync --no-install-project --no-install-workspace --all-extras --upgrade)
  # The generated e2e harness keeps its own uv.lock; refresh it so `upgrade` covers e2e and its
  # lock does not drift behind the workspace. ~keep
  (cd e2e/python && uv lock --upgrade) || true
}

refresh_node() {
  if [ "$mode" = update ]; then
    pnpm up
    return
  fi
  pnpm up --latest
  # The generated e2e/wasm harness is a standalone pnpm project (installed with
  # --ignore-workspace); refresh its lock so `upgrade` covers e2e. ~keep
  (cd e2e/wasm && pnpm up --latest --ignore-workspace) || true
}

refresh_ruby() {
  if [ "$mode" = update ]; then
    (cd packages/ruby && bundle update --conservative)
    return
  fi
  command -v bundle >/dev/null 2>&1 || {
    echo "Bundle not found, skipping Ruby upgrade"
    return
  }
  (
    cd packages/ruby
    prev_frozen="$(bundle config get frozen 2>/dev/null | awk '/Set for your local app/ {print $NF}')"
    bundle config set --local frozen false
    trap 'if [ -n "$prev_frozen" ] && [ "$prev_frozen" != "false" ]; then bundle config set --local frozen "$prev_frozen"; fi' EXIT
    bundle update --all
  )
}

# packages/java ships no Maven wrapper, so this runs the mvn mise installs. ~keep
refresh_java() {
  local goal=versions:use-next-versions
  [ "$mode" = update ] || goal=versions:use-latest-releases
  (cd packages/java && command -v mvn >/dev/null 2>&1 && mvn "$goal" -DgenerateBackupPoms=false ||
    echo "Maven not found, skipping Java upgrade")
}

# packages/zig pins no URL today, and the shared engine fails on a build.zig.zon without one, so
# an empty set is reported rather than treated as an error. ~keep
refresh_zig() {
  local urls url
  urls="$(grep '\.url' packages/zig/build.zig.zon 2>/dev/null | cut -d '=' -f 2 | cut -d '#' -f 1 | tr -d '", ' || true)"
  if [ -z "$urls" ]; then
    echo "No external Zig dependencies in build.zig.zon — nothing to $mode"
    return
  fi
  for url in $urls; do
    echo "Updating module $url"
    (cd packages/zig && zig fetch --save "$url")
    echo
  done
}

case "$usage_lang" in
all)
  for lang in rust python node ruby java zig; do
    "refresh_$lang"
  done
  ;;
rust | python | node | ruby | java | zig) "refresh_$usage_lang" ;;
*)
  echo "unknown ecosystem: $usage_lang" >&2
  exit 2
  ;;
esac
