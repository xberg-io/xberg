#!/usr/bin/env bash

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
source "$REPO_ROOT/tools/benchmark-harness/scripts/bench_local_profiles.sh"

fail() {
  echo "bench_local_profiles_test: $1" >&2
  exit 1
}

configure_benchmark_profile ""
[ "$BENCH_PROFILE_LABEL" = "default" ] || fail "default profile label"
[ "${BENCH_PROFILE_CARGO_ARGS[*]}" = "--features all" ] || fail "default Cargo arguments"

configure_benchmark_profile full
full_target="$BENCH_PROFILE_TARGET_DIR"
[ "${BENCH_PROFILE_CARGO_ARGS[*]}" = "--features all" ] || fail "full Cargo arguments"

configure_benchmark_profile pdf-heuristic
heuristic_target="$BENCH_PROFILE_TARGET_DIR"
[ "${BENCH_PROFILE_CARGO_ARGS[*]}" = "--no-default-features --features pdf-heuristic" ] \
  || fail "pdf-heuristic Cargo arguments"

configure_benchmark_profile pdf-ocr
[ "${BENCH_PROFILE_CARGO_ARGS[*]}" = "--no-default-features --features pdf-ocr" ] \
  || fail "pdf-ocr Cargo arguments"
[ "$full_target" != "$heuristic_target" ] || fail "full and heuristic targets overlap"
[ "$heuristic_target" != "$BENCH_PROFILE_TARGET_DIR" ] || fail "heuristic and OCR targets overlap"

if configure_benchmark_profile invalid >/dev/null 2>&1; then
  fail "invalid profile accepted"
fi

XBERG_BENCH_PROFILE=pdf-heuristic
configure_benchmark_profile "$XBERG_BENCH_PROFILE"
FRAMEWORKS_EXPLICIT=0
FRAMEWORKS=unchanged
OUT=results/local
apply_benchmark_profile_defaults
[ "$FRAMEWORKS" = "xberg-markdown-baseline,liteparse" ] || fail "lean framework defaults"
[ "$OUT" = "results/local/pdf-heuristic" ] || fail "profile output isolation"

OUT=custom-results
apply_benchmark_profile_defaults
[ "$OUT" = "custom-results/pdf-heuristic" ] || fail "explicit output profile isolation"

FRAMEWORKS_EXPLICIT=1
FRAMEWORKS=xberg-markdown-layout
OCR_FIXTURES=""
BATCH_OCR_FIXTURES=""
BATCH_FRAMEWORKS=""
if validate_benchmark_profile_inputs >/dev/null 2>&1; then
  fail "incompatible layout framework accepted"
fi

FRAMEWORKS=xberg-markdown-baseline
OCR_FIXTURES=ocr-fixtures
if validate_benchmark_profile_inputs >/dev/null 2>&1; then
  fail "OCR cohort accepted by heuristic profile"
fi

test_directory="$(mktemp -d "${TMPDIR:-/tmp}/xberg-bench-profile-test.XXXXXX")"
test_directory="$(cd "$test_directory" && pwd -P)"
cleanup() {
  rm -rf -- "$test_directory"
}
trap cleanup EXIT
original_repo_root="$REPO_ROOT"
REPO_ROOT="$test_directory/repository"
mkdir -p "$REPO_ROOT/target/release" "$REPO_ROOT/target/debug" "$test_directory/path-bin"
printf 'explicit' >"$test_directory/explicit-xberg"
printf 'release' >"$REPO_ROOT/target/release/xberg"
printf 'debug' >"$REPO_ROOT/target/debug/xberg"
printf 'path' >"$test_directory/path-bin/xberg"
chmod +x \
  "$test_directory/explicit-xberg" \
  "$REPO_ROOT/target/release/xberg" \
  "$REPO_ROOT/target/debug/xberg" \
  "$test_directory/path-bin/xberg"

XBERG_CLI_BINARY="$test_directory/explicit-xberg"
[ "$(resolve_default_xberg_binary)" = "$test_directory/explicit-xberg" ] || fail "explicit binary priority"
XBERG_CLI_BINARY="$test_directory/missing-explicit-xberg"
if resolve_default_xberg_binary >/dev/null 2>&1; then
  fail "invalid explicit binary fell through to release"
fi
unset XBERG_CLI_BINARY
[ "$(resolve_default_xberg_binary)" = "$REPO_ROOT/target/release/xberg" ] || fail "release binary priority"
mv "$REPO_ROOT/target/release/xberg" "$test_directory/release-xberg"
[ "$(resolve_default_xberg_binary)" = "$REPO_ROOT/target/debug/xberg" ] || fail "debug binary priority"
mv "$REPO_ROOT/target/debug/xberg" "$test_directory/debug-xberg"
PATH="$test_directory/path-bin:$PATH"
[ "$(resolve_default_xberg_binary)" = "$test_directory/path-bin/xberg" ] || fail "PATH binary fallback"
frameworks_include_xberg "liteparse,xberg-markdown-baseline" || fail "Xberg framework detection"
if frameworks_include_xberg "liteparse,docling"; then
  fail "non-Xberg framework false positive"
fi

XBERG_BENCH_PROFILE=""
configure_benchmark_profile ""
activate_xberg_profile
[ "$XBERG_CLI_BINARY" = "$test_directory/path-bin/xberg" ] || fail "default binary pinning"

fixture_root="$test_directory/fixtures"
mkdir -p "$fixture_root"
printf '%s\n' '{"metadata":{"requires_ocr":false}}' >"$fixture_root/heuristic.json"
printf '%s\n' '{"metadata":{"requires_ocr":true}}' >"$fixture_root/ocr.json"
printf '%s\n' '{"schema_version":1,"name":"heuristic","batch_size":1,"fixtures":["heuristic.json"]}' \
  >"$test_directory/heuristic-cohort.json"
printf '%s\n' '{"schema_version":1,"name":"ocr","batch_size":1,"fixtures":["ocr.json"]}' \
  >"$test_directory/ocr-cohort.json"
validate_ocr_cohort "$fixture_root" "$test_directory/heuristic-cohort.json" false
validate_ocr_cohort "$fixture_root" "$test_directory/ocr-cohort.json" true
if validate_ocr_cohort "$fixture_root" "" false >/dev/null 2>&1; then
  fail "legacy fixture-root validation ignored mixed OCR fixtures"
fi
if validate_ocr_cohort "$fixture_root" "$test_directory/ocr-cohort.json" false >/dev/null 2>&1; then
  fail "manifest-listed OCR mismatch accepted"
fi

REPO_ROOT="$original_repo_root"
BENCH_PROFILE_LABEL=pdf-ocr
BENCH_PROFILE_CARGO_FEATURES=pdf-ocr
BENCH_PROFILE_DEFAULT_FEATURES=false
BENCH_PROFILE_BINARY="$test_directory/xberg binary"
printf 'xberg' >"$BENCH_PROFILE_BINARY"
chmod +x "$BENCH_PROFILE_BINARY"
XBERG_BENCH_PROFILE=pdf-ocr
activate_xberg_profile
[ "$XBERG_CLI_BINARY" = "$BENCH_PROFILE_BINARY" ] || fail "explicit binary override"
write_benchmark_profile_provenance "$test_directory" "xberg-markdown-baseline"

python3 - "$test_directory/benchmark-profile.json" <<'PY'
import json
import pathlib
import sys

metadata = json.loads(pathlib.Path(sys.argv[1]).read_text(encoding="utf-8"))
assert metadata["profile"] == "pdf-ocr", metadata
assert metadata["cargo_features"] == ["pdf-ocr"], metadata
assert metadata["cargo_default_features"] is False, metadata
expected_path = str((pathlib.Path(sys.argv[1]).parent / "xberg binary").resolve())
assert metadata["binary_path"] == expected_path, (metadata["binary_path"], expected_path)
assert metadata["binary_sha256"] == "092b366007207df5bd1f13ae5bf5d0606ff781c99c104700ae8e14d4218732de", metadata
assert len(metadata["git_sha"]) == 40
assert isinstance(metadata["git_dirty"], bool)
assert len(metadata["git_diff_sha256"]) == 64
PY

BENCH_PROFILE_BINARY="$test_directory/missing-xberg"
if write_benchmark_profile_provenance "$test_directory" "xberg-markdown-baseline" >/dev/null 2>&1; then
  fail "missing Xberg provenance binary accepted"
fi
write_benchmark_profile_provenance "$test_directory/no-xberg" "liteparse"

echo "bench_local_profiles_test: passed"
