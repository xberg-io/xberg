#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
target="${1:-e2e}"

build_mock_server() {
  local manifest="$1"
  cargo build --release --manifest-path "$manifest" --bin mock-server
}

case "$target" in
  e2e)
    build_mock_server "$repo_root/e2e/rust/Cargo.toml"
    ;;
  test-apps)
    build_mock_server "$repo_root/test_apps/rust/Cargo.toml"
    ;;
  all)
    build_mock_server "$repo_root/e2e/rust/Cargo.toml"
    build_mock_server "$repo_root/test_apps/rust/Cargo.toml"
    ;;
  *)
    printf 'unknown mock-server target: %s\n' "$target" >&2
    exit 2
    ;;
esac
