#!/usr/bin/env bash
# Enforce deny.toml on every dependency graph a released artifact is built from.
#
# The release jobs only audit the Windows no-heic CLI, so a license that reaches any other
# shipped graph (the `all` CLI used by the Docker images, a language binding, or the default
# CLI) went unaudited until release time. This checks each graph on every push instead.
#
# Each row below roots the graph at that crate's OWN manifest. With `--manifest-path` pointing
# row of scripts/ci/license-graphs.txt roots the graph at a crate's OWN manifest. With
# `--manifest-path` pointing at a workspace member, cargo-deny makes that member the sole root
# of the graph, so the resolved features match what the artifact ships. A workspace-root run
# instead makes every member a root: it would audit crates that never ship
# (benchmark-harness's `inferno`) and force acceptances that, being repo-wide, would then
# excuse those crates in the shipped graphs too. `generate-notices.py` reads the same table,
# so the notice bundle covers exactly the graphs this gate certifies.
#
# A license exception in deny.toml is repo-wide, so no single graph has to encounter every
# entry: symphonia only arrives with `transcription`, cbindgen only with the FFI build. The
# coverage check at the end therefore requires each exception to be encountered by at least
# one checked graph, and fails when an exception matches nothing anywhere (a stale acceptance).
# `XBERG_LICENSE_GRAPHS`, `XBERG_DENY_CONFIG` and `XBERG_NO_LICENSE_FIELD_OK` exist for the
# negative controls in scripts/tests/bats/ci_shipped_licenses.bats, which audit scratch
# manifests and configs without editing this file or deny.toml.
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$repo_root"

# label|manifest|features|target. Empty features mean the crate's default features; an empty
# target means every platform's target-specific dependencies are resolved, not the host's.
graphs_file="${XBERG_LICENSE_GRAPHS_FILE:-scripts/ci/license-graphs.txt}"

if [ -n "${XBERG_LICENSE_GRAPHS:-}" ]; then
  graphs=()
  while IFS= read -r row; do
    if [ -n "$row" ]; then
      graphs+=("$row")
    fi
  done <<<"$XBERG_LICENSE_GRAPHS"
  if [ "${#graphs[@]}" -eq 0 ]; then
    echo "error: XBERG_LICENSE_GRAPHS is set but names no graph" >&2
    exit 2
  fi
else
  [ -f "$graphs_file" ] || {
    echo "error: graph table $graphs_file does not exist" >&2
    exit 1
  }
  graphs=()
  while IFS= read -r row; do
    case "$row" in
    "" | "#"*) continue ;;
    esac
    graphs+=("$row")
  done <"$graphs_file"
  if [ "${#graphs[@]}" -eq 0 ]; then
    echo "error: graph table $graphs_file names no graph" >&2
    exit 1
  fi
fi

config="${XBERG_DENY_CONFIG:-deny.toml}"
[ -f "$config" ] || {
  echo "error: license config $config does not exist" >&2
  exit 1
}
# Always pass the config explicitly. Expanding an empty array is an unbound-variable error under
# the bash 3.2 that macOS still ships with `set -u`, and naming the file makes the policy every
# graph is checked against unambiguous.
config_args=(--config "$config")

# cargo-deny 0.21.0 merges a project-local exceptions file it discovers by walking up from the
# audited manifest, unconditionally and even when `--config` is passed (see
# KrateContext::get_local_exceptions_path and ValidConfig::load). Those entries extend the allow
# list, so a committed `deny.exceptions.toml` would silently excuse a license this policy rejects
# and would never be reported as an unmatched exception. Refuse any file the graphs would load.
reject_local_exceptions() {
  local label=$1 manifest=$2 dir candidate
  dir="$(cd "$(dirname "$manifest")" && pwd -P)"
  while :; do
    for candidate in deny.exceptions.toml .deny.exceptions.toml .cargo/deny.exceptions.toml .config/deny.exceptions.toml; do
      if [ -e "$dir/$candidate" ]; then
        echo "error: $label: cargo-deny would load local license exceptions from $dir/$candidate;" \
          "remove it and declare the exception in $config" >&2
        exit 1
      fi
    done
    if [ "$dir" = "/" ]; then
      break
    fi
    dir="$(dirname "$dir")"
  done
}

# Crates that ship a license file but no `license` field in their manifest. cargo-deny detects
# the license from the file and only warns, so these would pass unexamined; `-D no-license-field`
# cannot be enabled while they are dependencies, and a clarify entry cannot replace a license
# that was already detected from the file. The list is asserted in both directions below, so a
# new crate without a manifest license fails, and an entry that stops being needed fails too.
# The negative controls set it empty because their scratch graphs are not the shipped set.
no_license_field_ok="${XBERG_NO_LICENSE_FIELD_OK-nom-exif model2vec-rs allo-isolate}"

command -v cargo-deny >/dev/null || {
  echo "error: cargo-deny is not installed; run 'mise install'" >&2
  exit 2
}

work="$(mktemp -d)"
trap 'rm -rf -- "$work"' EXIT
: >"$work/labels"

index=0
for row in "${graphs[@]}"; do
  IFS='|' read -r label manifest features target <<<"$row"
  index=$((index + 1))
  [ -f "$manifest" ] || {
    echo "error: $label: manifest $manifest does not exist" >&2
    exit 1
  }
  reject_local_exceptions "$label" "$manifest"

  args=(--manifest-path "$manifest" --locked "${config_args[@]}")
  if [ -n "$features" ]; then
    args+=(--features "$features")
  fi
  if [ -n "$target" ]; then
    args+=(--target "$target")
  fi

  # JSON diagnostics go to stderr; stdout stays empty. The exit status is authoritative, so a
  # failure re-runs the set in the human format to keep the inclusion graph readable in CI.
  if ! cargo deny "${args[@]}" -f json check licenses bans sources >/dev/null 2>"$work/$index.json"; then
    echo "FAIL $label" >&2
    cargo deny "${args[@]}" check licenses bans sources || true
    exit 1
  fi
  printf '%s\n' "$label" >>"$work/labels"
done

# One pass over the per-graph reports: verify each graph against its own summary counters, then
# assert the policy's acceptances are neither stale nor unaccounted for.
python3 - "$work" "$config" "$no_license_field_ok" <<'PY'
import json
import sys
import tomllib
from pathlib import Path

work, config, allowed_missing = Path(sys.argv[1]), sys.argv[2], set(sys.argv[3].split())
labels = (work / "labels").read_text().splitlines()
reports = sorted(work.glob("*.json"), key=lambda path: int(path.stem))
if len(labels) != len(reports):
    sys.exit(f"error: {len(labels)} graph labels but {len(reports)} reports")

unmatched_sets = []
missing_field_sets = []
for label, report in zip(labels, reports, strict=True):
    summary = None
    errors = warnings = 0
    unmatched = set()
    missing_field = set()
    for line in report.read_text().splitlines():
        line = line.strip()
        if not line.startswith("{"):
            continue
        event = json.loads(line)
        if event.get("type") == "summary":
            summary = event["fields"]
            continue
        if event.get("type") != "diagnostic":
            continue
        fields = event["fields"]
        severity = fields.get("severity")
        if severity == "error":
            errors += 1
        elif severity == "warning":
            warnings += 1
        if fields.get("code") == "license-exception-not-encountered":
            labels_ = fields.get("labels") or []
            if not labels_ or not labels_[0].get("span"):
                sys.exit(f"error: {label}: unmatched-exception diagnostic without a crate name")
            unmatched.add(labels_[0]["span"])
        if fields.get("code") == "no-license-field":
            crates = [entry["Krate"]["name"] for entry in fields.get("graphs") or [] if "Krate" in entry]
            if not crates:
                sys.exit(f"error: {label}: no-license-field diagnostic without a crate name")
            missing_field.add(crates[0])

    if summary is None:
        sys.exit(f"error: {label}: cargo-deny reported no summary event")
    for check in ("licenses", "bans", "sources"):
        if check not in summary:
            sys.exit(f"error: {label}: the summary is missing the {check} counters")
    counted = {key: sum(check[key] for check in summary.values()) for key in ("errors", "warnings")}
    if (errors, warnings) != (counted["errors"], counted["warnings"]):
        sys.exit(
            f"error: {label}: parsed {errors} errors and {warnings} warnings, "
            f"but cargo-deny counted {counted['errors']} and {counted['warnings']}"
        )
    accepted = summary["licenses"]["helps"]
    if accepted <= 0:
        sys.exit(f"error: {label}: the license check accepted no crate; the graph is empty")

    print(f"ok   {label}: {errors} errors, {warnings} warnings, {accepted} licenses accepted")
    unmatched_sets.append(unmatched)
    missing_field_sets.append(missing_field)

licenses_config = tomllib.loads(Path(config).read_text(encoding="utf-8")).get("licenses", {})
exceptions = set()
for entry in licenses_config.get("exceptions") or []:
    name = entry.get("crate") or entry.get("name")
    if name:
        exceptions.add(name)

flagged = set().union(*unmatched_sets)
undeclared = flagged - exceptions
if undeclared:
    sys.exit(f"error: {config} has no exception for unmatched crates: {', '.join(sorted(undeclared))}")

stale = exceptions.intersection(*unmatched_sets)
if stale:
    sys.exit(
        f"error: {config}: license exceptions match nothing in any checked graph:\n  "
        + "\n  ".join(sorted(stale))
    )

seen_missing = set().union(*missing_field_sets)
unknown_missing = seen_missing - allowed_missing
if unknown_missing:
    sys.exit(
        "error: crates ship without a license field in their manifest (declare one, or justify "
        f"it in no_license_field_ok): {', '.join(sorted(unknown_missing))}"
    )
stale_missing = allowed_missing - seen_missing
if stale_missing:
    sys.exit(
        "error: no_license_field_ok entries matched no crate lacking a license field; drop them: "
        f"{', '.join(sorted(stale_missing))}"
    )

if exceptions:
    print(f"checked {len(reports)} graphs; {len(exceptions)} license exceptions, each matched by at least one graph")
else:
    print(f"checked {len(reports)} graphs; no license exceptions declared")
PY
