#!/usr/bin/env bash
# Build and run an ONNX Runtime session with a staged ort-dynamic CLI (ADR-0017), so an archive
# whose shipped runtime is missing, untrusted or does not load fails before upload. The binary
# does not link ONNX Runtime; it loads the copy shipped beside it on the first model use. The
# same stage without that library must fail with the resolver's not-found error, proving the
# embedding needed it and that nothing else on the runner satisfied the search.
set -euo pipefail

stage="${1:?usage: smoke-cli-ort.sh <staged-cli-directory>}"

log() { echo "smoke-cli-ort: $*" >&2; }
die() {
  log "$*"
  exit 1
}

xberg="$stage/xberg"
[ -x "$xberg" ] || die "no executable xberg in $stage"
compgen -G "$stage/libonnxruntime.*" >/dev/null || die "no ONNX Runtime library staged in $stage"

workdir="$(mktemp -d)"
[ -n "$workdir" ] && [ -d "$workdir" ] || exit 90
trap 'rm -rf "$workdir"' EXIT

# No override and no loader search path, so only the library beside the binary can satisfy the
# load. The model download is isolated in the scratch cache. ~keep
run_embed() {
  local cli="$1"
  env -u ORT_DYLIB_PATH -u LD_LIBRARY_PATH -u DYLD_LIBRARY_PATH -u DYLD_FALLBACK_LIBRARY_PATH \
    XBERG_CACHE_DIR="$workdir/cache" RUST_LOG=xberg::ort_discovery=debug \
    "$cli" embed --text "xberg release smoke" --preset fast --format json
}

run_embed "$xberg" >"$workdir/embedded.json" 2>"$workdir/embedded.err" || {
  cat "$workdir/embedded.err" >&2
  die "the staged CLI could not build an ONNX Runtime session"
}
python3 - "$workdir/embedded.json" <<'PY' || die "the staged CLI did not return an embedding"
import json
import sys

with open(sys.argv[1], encoding="utf-8") as handle:
    output = json.load(handle)
if output.get("count") != 1 or not output.get("dimensions") or len(output["embeddings"][0]) != output["dimensions"]:
    raise SystemExit(f"unexpected embedding output: count={output.get('count')} dimensions={output.get('dimensions')}")
PY
real_stage="$(python3 -c 'import os, sys; print(os.path.realpath(sys.argv[1]))' "$stage")"
grep 'Loaded ONNX Runtime' "$workdir/embedded.err" | grep -qF "$real_stage/libonnxruntime." || {
  cat "$workdir/embedded.err" >&2
  die "the session did not load the ONNX Runtime shipped in $stage"
}

# The whole stage minus the runtime keeps every other library the binary needs. ~keep
without="$workdir/without-ort"
cp -R "$stage" "$without"
rm -f "$without"/libonnxruntime.*
if run_embed "$without/xberg" >"$workdir/absent.out" 2>&1; then
  die "embedding succeeded without the shipped ONNX Runtime; the check cannot tell the shipped library apart"
fi
grep -q 'ONNX Runtime library not found' "$workdir/absent.out" || {
  cat "$workdir/absent.out" >&2
  die "a missing ONNX Runtime failed for another reason than the library search"
}
log "built an ONNX Runtime session through the runtime shipped in $stage"
