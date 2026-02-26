#!/usr/bin/env bash
set -euo pipefail

mode="${1:-check}"

root="$(git rev-parse --show-toplevel)"

export PKG_CONFIG_PATH="$root/crates/kreuzberg-ffi:${PKG_CONFIG_PATH:-}"
export DYLD_LIBRARY_PATH="$root/target/debug:${DYLD_LIBRARY_PATH:-}"
export LD_LIBRARY_PATH="$root/target/debug:${LD_LIBRARY_PATH:-}"

# Go module directories in go.work
workspace_dirs=(
  packages/go/v4
  e2e/go
  tests/test_apps/go
  tools/benchmark-harness/scripts
)

# Standalone modules NOT in go.work (duplicate module paths, need GOWORK=off)
standalone_dirs=(
  crates/kreuzberg-wasm/e2e/go
  tools/e2e-generator/e2e/go
)

failed=0

lint_dir() {
  local dir="$1"
  local full="$root/$dir"

  if [ ! -f "$full/go.mod" ]; then
    return
  fi

  echo "==> Linting $dir"
  cd "$full"

  case "$mode" in
  fix)
    go fmt ./...
    golangci-lint run --config "$root/.golangci.yml" --fix ./... || failed=1
    ;;
  check)
    if gofmt -l . | read -r; then
      echo "  gofmt issues in $dir:"
      gofmt -l .
      failed=1
    fi
    golangci-lint run --config "$root/.golangci.yml" ./... || failed=1
    ;;
  *)
    echo "Usage: $0 [fix|check]" >&2
    exit 2
    ;;
  esac
}

for dir in "${workspace_dirs[@]}"; do
  lint_dir "$dir"
done

for dir in "${standalone_dirs[@]}"; do
  GOWORK=off lint_dir "$dir"
done

exit $failed
