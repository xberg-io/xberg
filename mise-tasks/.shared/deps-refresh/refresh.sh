#!/usr/bin/env bash
#MISE description="Update or upgrade one ecosystem's dependencies, or every ecosystem in DEPS_LANGS"
#USAGE arg "<mode>" help="update stays within major versions; upgrade takes the latest, breaking ones included"
#USAGE arg "[lang]" default="all" help="Ecosystem to refresh, or all for every one in DEPS_LANGS"
# shellcheck disable=SC2154 # usage_* are set by mise from the #USAGE lines
set -euo pipefail

# Repository specifics come from the task's env: DEPS_LANGS (order for all), DEPS_<LANG>_DIR (default
# packages/<lang>), DEPS_NODE_DIRS and DEPS_RUST_MANIFESTS (extra manifests beside the workspace). ~keep
mode="$usage_mode"
case "$mode" in
update | upgrade) ;;
*)
  echo "mode must be update or upgrade, got: $mode" >&2
  exit 2
  ;;
esac

package_dir() {
  local var
  var="DEPS_$(printf '%s' "$1" | tr '[:lower:]-' '[:upper:]_')_DIR"
  printf '%s' "${!var:-packages/$1}"
}

refresh_zig() {
  local urls url
  # grep exits 1 for a manifest that pins no URL; under pipefail that is not an error here. ~keep
  urls="$({ grep '\.url' build.zig.zon || true; } | cut -d '=' -f 2 | cut -d '#' -f 1 | tr -d '", ')"
  if [ -z "$urls" ]; then
    echo "zig: build.zig.zon pins no dependency URL; nothing to refresh"
    return
  fi
  for url in $urls; do
    echo "Updating module $url"
    zig fetch --save "$url"
    echo
  done
}

refresh() {
  local lang="$1" manifest dir
  case "$lang" in
  rust)
    if [ "$mode" = upgrade ]; then cargo upgrade --incompatible; fi
    cargo update
    for manifest in ${DEPS_RUST_MANIFESTS:-}; do
      if [ "$mode" = upgrade ]; then cargo upgrade --incompatible --manifest-path "$manifest"; fi
      cargo update --manifest-path "$manifest"
    done
    ;;
  node)
    for dir in ${DEPS_NODE_DIRS:-packages/typescript}; do
      if [ "$mode" = upgrade ]; then (cd "$dir" && pnpm up --latest); else (cd "$dir" && pnpm up); fi
    done
    ;;
  kotlin-android)
    echo "[kotlin_android] skip: ben-manes/gradle-versions-plugin incompatible with Gradle 9.x"
    ;;
  *)
    dir="$(package_dir "$lang")"
    (
      cd "$dir"
      case "$lang:$mode" in
      python:update) uv sync --upgrade-package "*" --no-prerelease --no-install-project --no-install-workspace ;;
      python:upgrade) uv sync --upgrade --no-install-project --no-install-workspace ;;
      ruby:update) bundle update --conservative ;;
      ruby:upgrade) bundle update ;;
      php:update) composer update --with-dependencies ;;
      php:upgrade) composer update --with-all-dependencies ;;
      go:update) go get -u=patch ./... && go mod tidy ;;
      go:upgrade) go get -u ./... && go mod tidy ;;
      java:update) ./mvnw versions:use-next-versions -DgenerateBackupPoms=false ;;
      java:upgrade) ./mvnw versions:use-latest-releases -DgenerateBackupPoms=false ;;
      csharp:update) dotnet outdated -u --version-lock Minor ;;
      csharp:upgrade) dotnet outdated -u ;;
      elixir:update) mix deps.update --all ;;
      elixir:upgrade) { mix hex.outdated --all || true; } && mix deps.update --all ;;
      dart:update) dart pub upgrade ;;
      dart:upgrade) dart pub upgrade --major-versions ;;
      # SwiftPM has neither a conservative nor a major-version flag. ~keep
      swift:*) swift package update ;;
      zig:*) refresh_zig ;;
      *)
        echo "unknown ecosystem: $lang" >&2
        exit 2
        ;;
      esac
    )
    ;;
  esac
}

if [ "$usage_lang" = all ]; then
  for lang in ${DEPS_LANGS:?set DEPS_LANGS to the ecosystems all refreshes}; do
    refresh "$lang"
  done
else
  refresh "$usage_lang"
fi
