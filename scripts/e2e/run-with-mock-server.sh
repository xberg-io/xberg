#!/usr/bin/env bash
set -euo pipefail

if [ "$#" -eq 0 ]; then
  printf 'usage: %s <command> [args...]\n' "$0" >&2
  exit 2
fi

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

"$repo_root/scripts/e2e/build-mock-server.sh" e2e

mock_server_bin="$repo_root/e2e/rust/target/release/mock-server"
fixtures_dir="$repo_root/fixtures"

tmp_dir="$(mktemp -d)"
stdout_fifo="$tmp_dir/stdout"
stdin_fifo="$tmp_dir/stdin"
mkfifo "$stdout_fifo" "$stdin_fifo"

exec 3<>"$stdin_fifo"
"$mock_server_bin" "$fixtures_dir" <"$stdin_fifo" >"$stdout_fifo" &
mock_server_pid="$!"
exec 4<"$stdout_fifo"

cleanup() {
  kill "$mock_server_pid" >/dev/null 2>&1 || true
  wait "$mock_server_pid" >/dev/null 2>&1 || true
  exec 3>&- || true
  exec 4<&- || true
  rm -rf "$tmp_dir"
}
trap cleanup EXIT

mock_server_url=""
mock_servers_json=""

for _ in $(seq 1 20); do
  if IFS= read -r -t 1 line <&4; then
    case "$line" in
      MOCK_SERVER_URL=*)
        mock_server_url="${line#MOCK_SERVER_URL=}"
        ;;
      MOCK_SERVERS=*)
        mock_servers_json="${line#MOCK_SERVERS=}"
        break
        ;;
      *)
        printf '%s\n' "$line" >&2
        ;;
    esac
  fi
done

if [ -z "$mock_server_url" ]; then
  printf 'mock-server did not emit MOCK_SERVER_URL\n' >&2
  exit 1
fi

export MOCK_SERVER_URL="$mock_server_url"

if [ -n "$mock_servers_json" ]; then
  export MOCK_SERVERS="$mock_servers_json"
  while IFS='=' read -r key value; do
    [ -n "$key" ] || continue
    export "$key=$value"
  done < <(
    MOCK_SERVERS_JSON="$mock_servers_json" python3 - <<'PY'
import json
import os

for fixture_id, url in json.loads(os.environ["MOCK_SERVERS_JSON"]).items():
    print(f"MOCK_SERVER_{fixture_id.upper()}={url}")
PY
  )
fi

"$@"
