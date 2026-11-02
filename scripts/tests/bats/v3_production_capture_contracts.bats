#!/usr/bin/env bats

setup() {
	bats_load_library xberg-bats
	xberg_setup
	REPOSITORY_ROOT="$(cd "$BATS_TEST_DIRNAME/../../.." && pwd -P)"
	export XBERG_V3_PARITY_ROOT="$XBERG_WORK/parity"
	export XBERG_V3_PARITY_MODEL="$XBERG_WORK/model.onnx"
	export XBERG_V3_CAPTURE_OUT="$XBERG_WORK/capture"
	export V3_CAPTURE_CASE="complete"
	cat >"$XBERG_STUB_BIN/cargo" <<'CARGO'
#!/usr/bin/env bash
set -euo pipefail
printf '%s\n' "$*" >>"$XBERG_TRACE"
name=layout::models::pp_doclayout_v3::production_parity::should_capture_two_production_v3_forwards
if [[ " $* " == *' --list '* ]]; then
  case "$V3_CAPTURE_CASE" in
    zero-scope) printf '0 tests, 0 benchmarks\n' ;;
    wrong-scope) printf 'another::capture: test\n' ;;
    duplicate-scope) printf '%s: test\n%s: test\n' "$name" "$name" ;;
    *) printf '%s: test\n' "$name" ;;
  esac
  exit 0
fi
if [[ "$V3_CAPTURE_CASE" == cargo-error ]]; then
  exit 19
fi
if [[ "$V3_CAPTURE_CASE" == zero-runtime ]]; then
  printf 'test result: ok. 0 passed; 0 failed; 0 ignored; 1 filtered out\n'
  exit 0
fi
printf 'test result: ok. 1 passed; 0 failed; 0 ignored; 0 filtered out\n'
case "$V3_CAPTURE_CASE" in
  zero-scope|wrong-scope|duplicate-scope|missing-manifest) exit 0 ;;
esac
python3 - <<'PY'
import hashlib
import json
import os
import struct
from pathlib import Path

root = Path(os.environ["XBERG_V3_CAPTURE_OUT"])
root.mkdir()
rows = []
for name in ["document-1", "document-2"]:
    tensors = {
        "inputs": [
            ("im_shape", "<f4", [1, 2], struct.pack("<ff", 800.0, 800.0)),
            ("image", "<f4", [1, 3, 800, 800], bytes(1 * 3 * 800 * 800 * 4)),
            ("scale_factor", "<f4", [1, 2], struct.pack("<ff", 1.0, 1.0)),
        ],
        "outputs": [
            ("fetch_name_0", "<f4", [1, 7], struct.pack("<7f", 22.0, 0.9, 0.0, 0.0, 800.0, 800.0, 0.0)),
            ("fetch_name_1", "<i4", [1], struct.pack("<i", 1)),
            ("fetch_name_2", "<i4", [1, 200, 200], bytes(1 * 200 * 200 * 4)),
        ],
    }
    row = {"input": name}
    for side, values in tensors.items():
        directory = root / name / side
        directory.mkdir(parents=True)
        records = []
        for tensor, dtype, shape, data in values:
            filename = tensor + ".bin"
            (directory / filename).write_bytes(data)
            records.append({"name": tensor, "dtype": dtype, "shape": shape, "file": filename,
                            "bytes": len(data), "sha256": hashlib.sha256(data).hexdigest()})
        row[side] = records
    rows.append(row)
manifest = {"schema_version": 1, "qualification_complete": False,
            "real_forward_passes": 2, "input_tensors": 6, "output_tensors": 6, "rows": rows}
if os.environ["V3_CAPTURE_CASE"] == "incomplete-manifest":
    manifest["real_forward_passes"] = 0
(root / "manifest.json").write_text(json.dumps(manifest))
PY
CARGO
	chmod +x "$XBERG_STUB_BIN/cargo"
}

run_capture() {
	export V3_CAPTURE_CASE="$1"
	run "$BASH" "$REPOSITORY_ROOT/mise-tasks/benchmark/v3-production-capture.sh"
	[ -s "$XBERG_TRACE" ]
	grep -Fq -- '--features layout-detection,ort-dynamic' "$XBERG_TRACE"
	grep -Fq 'layout::models::pp_doclayout_v3::production_parity::should_capture_two_production_v3_forwards' "$XBERG_TRACE"
}

@test "V3 capture task should reject zero selected tests" {
	run_capture zero-scope
	xberg_assert_status 1
	xberg_assert_output_contains 'expected exactly one capture test'
	[ ! -e "$XBERG_V3_CAPTURE_OUT" ]
}

@test "V3 capture task should reject a different selected test" {
	run_capture wrong-scope
	xberg_assert_status 1
	xberg_assert_output_contains 'expected exactly one capture test'
}

@test "V3 capture task should reject duplicate selected tests" {
	run_capture duplicate-scope
	xberg_assert_status 1
	xberg_assert_output_contains 'expected exactly one capture test'
}

@test "V3 capture task should propagate a failed Cargo invocation" {
	run_capture cargo-error
	xberg_assert_status 19
	[ ! -e "$XBERG_V3_CAPTURE_OUT" ]
}

@test "V3 capture task should reject zero runtime work" {
	run_capture zero-runtime
	xberg_assert_status 1
	xberg_assert_output_contains 'capture test did not execute exactly once'
}

@test "V3 capture task should reject success without its manifest" {
	run_capture missing-manifest
	xberg_assert_status 1
	xberg_assert_output_contains 'capture manifest missing'
}

@test "V3 capture task should reject an incomplete forward manifest" {
	run_capture incomplete-manifest
	xberg_assert_status 1
	xberg_assert_output_contains 'capture manifest incomplete'
}

@test "V3 capture task should accept a complete synthetic shell-contract fixture" {
	run_capture complete
	xberg_assert_status 0
	[ -s "$XBERG_V3_CAPTURE_OUT/manifest.json" ]
}
