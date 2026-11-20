#!/usr/bin/env bash
# Enforce deny.toml on every dependency graph a released artifact is built from.
#
# The release jobs only audit the Windows no-heic CLI, so a license that reaches any other
# shipped graph (the `all` CLI used by the Docker images, a language binding, or the default
# CLI) went unaudited until release time. This checks each graph on every push instead.
#
# Each row below roots the graph at that crate's OWN manifest. With `--manifest-path` pointing
# at a workspace member, cargo-deny makes that member the sole root of the graph, so the
# resolved features match what the artifact ships. A workspace-root run instead makes every
# member a root: it would audit crates that never ship (benchmark-harness's `inferno`) and
# force acceptances that, being repo-wide, would then excuse those crates in the shipped graphs
# too.
#
# A license exception in deny.toml is repo-wide, so no single graph has to encounter every
# entry: symphonia only arrives with `transcription`, cbindgen only with the FFI build. The
# coverage check at the end therefore requires each exception to be encountered by at least
# one checked graph, and fails when an exception matches nothing anywhere (a stale acceptance).
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$repo_root"

# label|manifest|features|target (empty features/target mean the crate's default set, host target)
sets=(
  "xberg-cli (default)|crates/xberg-cli/Cargo.toml||"
  "xberg-cli --features all|crates/xberg-cli/Cargo.toml|all|"
  "xberg-py (default)|crates/xberg-py/Cargo.toml||"
  "xberg-node (default)|crates/xberg-node/Cargo.toml||"
  "xberg-php (default)|crates/xberg-php/Cargo.toml||"
  "xberg-ffi (default)|crates/xberg-ffi/Cargo.toml||"
  "xberg-jni (default)|crates/xberg-jni/Cargo.toml||"
  "xberg-wasm (default)|crates/xberg-wasm/Cargo.toml||"
  # The Windows no-heic release archive: publish.yaml's matrix cargo_features for that leg.
  "xberg-cli --features mcp,mcp-http,api (windows no-heic)|crates/xberg-cli/Cargo.toml|mcp,mcp-http,api|x86_64-pc-windows-msvc"
)

command -v cargo-deny >/dev/null || {
  echo "error: cargo-deny is not installed; run 'mise install'" >&2
  exit 2
}

work="$(mktemp -d)"
trap 'rm -rf -- "$work"' EXIT

index=0
for row in "${sets[@]}"; do
  IFS='|' read -r label manifest features target <<<"$row"
  index=$((index + 1))
  [ -f "$manifest" ] || {
    echo "error: $label: manifest $manifest does not exist" >&2
    exit 1
  }
  json="$work/$index.json"
  unmatched="$work/$index.unmatched"

  args=(--manifest-path "$manifest" --locked)
  if [ -n "$features" ]; then
    args+=(--features "$features")
  fi
  if [ -n "$target" ]; then
    args+=(--target "$target")
  fi

  # JSON diagnostics go to stderr; stdout stays empty. The exit status is authoritative, so a
  # failure re-runs the set in the human format to keep the inclusion graph readable in CI.
  if ! cargo deny "${args[@]}" -f json check licenses bans sources >/dev/null 2>"$json"; then
    echo "FAIL $label" >&2
    cargo deny "${args[@]}" check licenses bans sources || true
    exit 1
  fi

  # Parses one run: prints its summary, records the exceptions this graph did not encounter,
  # and refuses output it cannot read (so an empty or reshaped report cannot pass as clean).
  python3 - "$json" "$label" "$unmatched" <<'PY'
import json
import sys

path, label, unmatched_path = sys.argv[1:4]
errors = 0
warnings = 0
unmatched = []
seen = False

with open(path, encoding="utf-8") as output:
    for line in output:
        line = line.strip()
        if not line.startswith("{"):
            continue
        try:
            event = json.loads(line)
        except json.JSONDecodeError:
            sys.exit(f"{label}: unreadable cargo-deny output: {line}")
        if event.get("type") != "diagnostic":
            continue
        seen = True
        fields = event["fields"]
        severity = fields.get("severity")
        if severity == "error":
            errors += 1
        elif severity == "warning":
            warnings += 1
        if fields.get("code") == "license-exception-not-encountered":
            labels = fields.get("labels") or []
            if not labels or not labels[0].get("span"):
                sys.exit(f"{label}: license-exception diagnostic without a crate name")
            unmatched.append(labels[0]["span"])

if not seen:
    sys.exit(f"{label}: cargo-deny reported no diagnostics; its output is not being read")

with open(unmatched_path, "w", encoding="utf-8") as output:
    output.write("".join(f"{crate}\n" for crate in unmatched))

print(f"ok   {label}: {errors} errors, {warnings} warnings")
PY
done

# An exception is stale when no checked graph encountered it. Intersecting the per-graph
# unmatched sets is what makes "matched somewhere" the requirement rather than "matched here".
flagged="$(cat "$work"/*.unmatched)"
if [ -n "$flagged" ]; then
  stale="$(printf '%s\n' "$flagged" | sort | uniq -c | awk -v n="$index" '$1 == n { print $2 }')"
else
  stale=""
fi

exceptions="$(grep -c 'crate = "' deny.toml)"
if [ -n "$stale" ]; then
  echo "error: deny.toml license exceptions match nothing in any checked graph:" >&2
  while IFS= read -r crate; do
    printf '  %s\n' "$crate" >&2
  done <<<"$stale"
  exit 1
fi
if [ -n "$flagged" ]; then
  echo "checked $index graphs; $exceptions license exceptions, each matched by at least one graph"
else
  # No graph reported an unmatched exception, so nothing proves the acceptances are exercised:
  # the parser or cargo-deny's diagnostic code changed under us. Fail instead of reporting a pass.
  echo "error: no graph reported an unmatched license exception; coverage of $exceptions entries cannot be verified" >&2
  exit 1
fi
