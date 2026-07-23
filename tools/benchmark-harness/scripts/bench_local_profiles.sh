#!/usr/bin/env bash

configure_benchmark_profile() {
  local profile="${1:-}"

  case "$profile" in
  "")
    BENCH_PROFILE_LABEL="default"
    BENCH_PROFILE_CARGO_FEATURES="all"
    BENCH_PROFILE_DEFAULT_FEATURES=true
    BENCH_PROFILE_TARGET_DIR="$REPO_ROOT/target"
    BENCH_PROFILE_BINARY=""
    BENCH_PROFILE_CARGO_ARGS=(--features all)
    ;;
  full)
    BENCH_PROFILE_LABEL="full"
    BENCH_PROFILE_CARGO_FEATURES="all"
    BENCH_PROFILE_DEFAULT_FEATURES=true
    BENCH_PROFILE_TARGET_DIR="$REPO_ROOT/target/benchmark-profiles/full"
    BENCH_PROFILE_BINARY="$BENCH_PROFILE_TARGET_DIR/release/xberg"
    BENCH_PROFILE_CARGO_ARGS=(--features all)
    ;;
  pdf-heuristic | pdf-ocr)
    BENCH_PROFILE_LABEL="$profile"
    BENCH_PROFILE_CARGO_FEATURES="$profile"
    BENCH_PROFILE_DEFAULT_FEATURES=false
    BENCH_PROFILE_TARGET_DIR="$REPO_ROOT/target/benchmark-profiles/$profile"
    BENCH_PROFILE_BINARY="$BENCH_PROFILE_TARGET_DIR/release/xberg"
    BENCH_PROFILE_CARGO_ARGS=(--no-default-features --features "$profile")
    ;;
  *)
    echo "[bench:local] unsupported XBERG_BENCH_PROFILE: $profile" >&2
    echo "[bench:local] expected one of: full, pdf-heuristic, pdf-ocr" >&2
    return 1
    ;;
  esac
}

apply_benchmark_profile_defaults() {
  case "$BENCH_PROFILE_LABEL" in
  pdf-heuristic | pdf-ocr)
    if [ "$FRAMEWORKS_EXPLICIT" = 0 ]; then
      FRAMEWORKS="xberg-markdown-baseline,liteparse"
    fi
    ;;
  esac

  if [ -n "${XBERG_BENCH_PROFILE:-}" ]; then
    OUT="$OUT/$BENCH_PROFILE_LABEL"
  fi
}

canonical_executable() {
  local candidate="$1"
  [ -x "$candidate" ] || return 1
  python3 - "$candidate" <<'PY'
import pathlib
import sys

print(pathlib.Path(sys.argv[1]).resolve(strict=True))
PY
}

resolve_default_xberg_binary() {
  local candidate resolved

  if [ -n "${XBERG_CLI_BINARY:-}" ]; then
    candidate="$XBERG_CLI_BINARY"
    resolved="$(command -v "$candidate" 2>/dev/null || true)"
    if [ -z "$resolved" ] || ! resolved="$(canonical_executable "$resolved")"; then
      echo "[bench:local] XBERG_CLI_BINARY is not executable: $candidate" >&2
      return 1
    fi
    printf '%s\n' "$resolved"
    return
  fi

  for candidate in "$REPO_ROOT/target/release/xberg" "$REPO_ROOT/target/debug/xberg"; do
    if resolved="$(canonical_executable "$candidate" 2>/dev/null)"; then
      printf '%s\n' "$resolved"
      return
    fi
  done

  resolved="$(command -v xberg 2>/dev/null || true)"
  if [ -n "$resolved" ] && resolved="$(canonical_executable "$resolved")"; then
    printf '%s\n' "$resolved"
    return
  fi

  echo "[bench:local] xberg CLI not found (checked XBERG_CLI_BINARY, target/release, target/debug, and PATH)." >&2
  return 1
}

frameworks_include_xberg() {
  local framework remaining="$1"

  while [ -n "$remaining" ]; do
    case "$remaining" in
    *,*)
      framework="${remaining%%,*}"
      remaining="${remaining#*,}"
      ;;
    *)
      framework="$remaining"
      remaining=""
      ;;
    esac
    case "$framework" in
    xberg-*) return 0 ;;
    esac
  done
  return 1
}

validate_benchmark_profile_inputs() {
  local framework remaining

  if [ "$BENCH_PROFILE_LABEL" = "pdf-heuristic" ] \
    && { [ -n "$OCR_FIXTURES" ] || [ -n "$BATCH_OCR_FIXTURES" ]; }; then
    echo "[bench:local] pdf-heuristic cannot run OCR cohorts; use XBERG_BENCH_PROFILE=pdf-ocr." >&2
    return 1
  fi

  case "$BENCH_PROFILE_LABEL" in
  pdf-heuristic | pdf-ocr) ;;
  *) return ;;
  esac

  remaining="$FRAMEWORKS${BATCH_FRAMEWORKS:+,$BATCH_FRAMEWORKS}"
  while [ -n "$remaining" ]; do
    case "$remaining" in
    *,*)
      framework="${remaining%%,*}"
      remaining="${remaining#*,}"
      ;;
    *)
      framework="$remaining"
      remaining=""
      ;;
    esac
    case "$framework" in
    xberg-markdown-baseline | xberg-markdown-baseline-batch | "") ;;
    xberg-*)
      echo "[bench:local] profile '$BENCH_PROFILE_LABEL' does not support framework '$framework'." >&2
      echo "[bench:local] lean PDF profiles support the Xberg baseline pipeline only." >&2
      return 1
      ;;
    esac
  done
}

build_xberg_profile() {
  echo "[bench:local] Building xberg CLI profile '$BENCH_PROFILE_LABEL' in $BENCH_PROFILE_TARGET_DIR…"
  CARGO_TARGET_DIR="$BENCH_PROFILE_TARGET_DIR" \
    cargo build --locked --release -p xberg-cli "${BENCH_PROFILE_CARGO_ARGS[@]}"
}

activate_xberg_profile() {
  local resolved

  if [ -z "${XBERG_BENCH_PROFILE:-}" ]; then
    resolved="$(resolve_default_xberg_binary)" || return 1
    BENCH_PROFILE_BINARY="$resolved"
    export XBERG_CLI_BINARY="$resolved"
    return
  fi

  if [ ! -x "$BENCH_PROFILE_BINARY" ]; then
    echo "[bench:local] xberg profile binary is not executable: $BENCH_PROFILE_BINARY" >&2
    if [ "${SKIP_BUILD:-0}" = "1" ]; then
      echo "[bench:local] unset SKIP_BUILD or build XBERG_BENCH_PROFILE=$BENCH_PROFILE_LABEL first." >&2
    fi
    return 1
  fi

  resolved="$(canonical_executable "$BENCH_PROFILE_BINARY")" || return 1
  BENCH_PROFILE_BINARY="$resolved"
  export XBERG_CLI_BINARY="$resolved"
}

binary_sha256() {
  local binary="$1"
  python3 - "$binary" <<'PY'
import hashlib
import pathlib
import sys

digest = hashlib.sha256()
with pathlib.Path(sys.argv[1]).open("rb") as binary:
    for chunk in iter(lambda: binary.read(1024 * 1024), b""):
        digest.update(chunk)
print(digest.hexdigest())
PY
}

git_diff_sha256() {
  git -C "$REPO_ROOT" diff --binary HEAD | python3 -c \
    'import hashlib, sys; print(hashlib.sha256(sys.stdin.buffer.read()).hexdigest())'
}

validate_ocr_cohort() {
  local fixture_root="$1"
  local manifest="$2"
  local expected="$3"
  python3 - "$fixture_root" "$manifest" "$expected" <<'PY'
import json
import pathlib
import sys

root = pathlib.Path(sys.argv[1])
manifest_path = pathlib.Path(sys.argv[2]) if sys.argv[2] else None
expected = sys.argv[3] == "true"
image_types = {"png", "jpg", "jpeg", "gif", "bmp", "tiff", "tif", "webp", "jp2", "jpx", "jpm", "mj2"}
if manifest_path is not None:
    try:
        manifest_data = json.loads(manifest_path.read_text(encoding="utf-8"))
        listed_fixtures = manifest_data["fixtures"]
        if not isinstance(listed_fixtures, list) or not listed_fixtures:
            raise ValueError("fixtures must be a non-empty list")
        relative_fixtures = [pathlib.Path(item) for item in listed_fixtures]
        if any(path.is_absolute() or ".." in path.parts for path in relative_fixtures):
            raise ValueError("fixtures must contain normalized relative paths")
        fixture_paths = [root / path for path in relative_fixtures]
    except (OSError, UnicodeError, json.JSONDecodeError, KeyError, TypeError, ValueError) as error:
        raise SystemExit(f"invalid cohort manifest {manifest_path}: {error}") from error
else:
    fixture_paths = sorted(root.rglob("*.json")) if root.is_dir() else [root]

bad = []
for path in fixture_paths:
    try:
        fixture = json.loads(path.read_text(encoding="utf-8"))
        metadata_value = fixture.get("metadata", {}).get("requires_ocr")
        if isinstance(metadata_value, bool):
            requires_ocr = metadata_value
        else:
            file_type = str(fixture.get("file_type", "")).lower()
            document_type = pathlib.Path(str(fixture.get("document", ""))).suffix.lstrip(".").lower()
            requires_ocr = file_type in image_types or document_type in image_types
        if requires_ocr != expected:
            bad.append(str(path))
    except (OSError, UnicodeError, json.JSONDecodeError, AttributeError) as error:
        bad.append(f"{path} ({error})")

if not fixture_paths:
    raise SystemExit(f"cohort contains no fixture JSON files: {root}")
if bad:
    label = "OCR-required" if expected else "non-OCR"
    preview = "\n  - ".join(bad[:10])
    raise SystemExit(f"cohort must contain only {label} fixtures; mismatches:\n  - {preview}")
PY
}

write_benchmark_profile_provenance() {
  local output="$1"
  local frameworks="$2"
  local binary_hash git_diff_hash git_dirty git_sha

  frameworks_include_xberg "$frameworks" || return 0

  if [ ! -x "$BENCH_PROFILE_BINARY" ]; then
    echo "[bench:local] cannot record Xberg provenance; binary is not executable: $BENCH_PROFILE_BINARY" >&2
    return 1
  fi

  if ! binary_hash="$(binary_sha256 "$BENCH_PROFILE_BINARY")" || [ -z "$binary_hash" ]; then
    echo "[bench:local] failed to hash Xberg binary: $BENCH_PROFILE_BINARY" >&2
    return 1
  fi
  git_diff_hash="$(git_diff_sha256)"
  if [ -n "$(git -C "$REPO_ROOT" status --porcelain=v1 --untracked-files=all)" ]; then
    git_dirty=true
  else
    git_dirty=false
  fi
  git_sha="$(git -C "$REPO_ROOT" rev-parse HEAD)"
  python3 - \
    "$output/benchmark-profile.json" \
    "$BENCH_PROFILE_LABEL" \
    "$BENCH_PROFILE_CARGO_FEATURES" \
    "$BENCH_PROFILE_DEFAULT_FEATURES" \
    "$BENCH_PROFILE_BINARY" \
    "$binary_hash" \
    "$git_sha" \
    "$git_dirty" \
    "$git_diff_hash" <<'PY'
import json
import pathlib
import sys

path = pathlib.Path(sys.argv[1])
metadata = {
    "profile": sys.argv[2],
    "cargo_features": sys.argv[3].split(","),
    "cargo_default_features": sys.argv[4] == "true",
    "binary_path": sys.argv[5],
    "binary_sha256": sys.argv[6],
    "git_sha": sys.argv[7],
    "git_dirty": sys.argv[8] == "true",
    "git_diff_sha256": sys.argv[9],
}
path.write_text(json.dumps(metadata, indent=2, sort_keys=True) + "\n", encoding="utf-8")
PY
}
