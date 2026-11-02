"""Run only the frozen selected-block diagnostic; never re-export or qualify."""

import argparse
import hashlib
import importlib
import inspect
import json
from pathlib import Path

import numpy as np
import onnx
from onnx.external_data_helper import _get_all_tensors

from .assets import digest, read_json
from .first_divergence import ATOL, RTOL, compare_tensor
from .first_divergence_evidence import require
from .first_divergence_run import frozen_inputs, offline_environment, session
from .provenance import verify_runtime_versions
from .subblock import (
    MULTIPLIERS,
    STAGES,
    case_lengths,
    compare_taps,
    instrument_graph,
    resolve_taps,
    summarize,
    validate_controls,
)
from .subblock_hooks import capture_native
from .subblock_plan import PROTOCOL, verify_destination, verify_plan


def _reference_code(model: object, plan: dict) -> dict:
    block = model.decoder.transformer.h[plan["protocol"].get("block_index", 0)]
    result = {}
    for name, forward in (("gpt2", block.forward), ("conv1d", block.attn.c_attn.forward)):
        actual = inspect.getsourcefile(forward)
        pin = plan["dependency_sources"][name]
        require(
            actual is not None and Path(actual).resolve() == Path(pin["path"]).resolve(),
            f"actual {name} forward comes from a different source",
        )
        require(digest(Path(actual)) == pin["sha256"], f"actual {name} implementation checksum mismatch")
        result[name] = {**pin, "forward": f"{forward.__module__}.{forward.__qualname__}"}
    return result


def _prepare_graph(original_plan: dict, previous: dict, output: Path, *, block_index: int = 0) -> tuple:
    source = Path(original_plan["bundle"]) / "model/decoder.onnx"
    graph = onnx.load(source, load_external_data=False)
    require(
        not any(tensor.data_location == onnx.TensorProto.EXTERNAL for tensor in _get_all_tensors(graph)),
        "external diagnostic tensors unsupported",
    )
    taps = resolve_taps(graph, block_index=block_index, layer_count=original_plan["geometry"]["layers"])
    anchors = {tap["stage"]: tap["tensor"] for tap in previous["taps"]}
    incoming = "embedding" if block_index == 0 else f"block.{block_index - 1}"
    require(
        taps[0]["tensor"] == anchors[incoming] and taps[-1]["tensor"] == anchors[f"block.{block_index}"],
        "subblock anchors differ from the prior diagnostic",
    )
    diagnostic = instrument_graph(graph, taps)
    outlets = list(diagnostic.graph.output)
    del diagnostic.graph.output[:]
    diagnostic.graph.output.extend(graph.graph.output)
    require(diagnostic.SerializeToString() == graph.SerializeToString(), "diagnostic changed original protobuf")
    del diagnostic.graph.output[:]
    diagnostic.graph.output.extend(outlets)
    destination = output / "decoder-subblock-taps.onnx"
    onnx.save_model(diagnostic, destination, save_as_external_data=False)
    onnx.checker.check_model(str(destination))
    return destination, taps


def _negative_controls(expected: dict, actual: dict) -> list[dict]:
    length = expected["mlp_fc"].shape[1]
    coordinate = (0, length - 1, 3)
    poisoned = {name: value.copy() for name, value in expected.items()}
    poisoned["mlp_fc"][coordinate] += np.float32(1.0)
    rows = compare_taps(expected, poisoned, prefix_length=length)
    failed = [row for row in rows if not row["passed"]]
    require(
        len(failed) == 1
        and failed[0]["stage"] == "mlp_fc"
        and failed[0]["failed_elements"] == 1
        and failed[0]["first_failed_index"] == list(coordinate),
        "poisoned stage failed for the wrong reason",
    )
    missing = dict(actual)
    del missing["cross_query"]
    try:
        compare_taps(expected, missing, prefix_length=length)
    except ValueError as error:
        require("missing" in str(error), "missing stage failed for the wrong reason")
        return [{"control": "poisoned_element", **failed[0]}, {"control": "missing_stage", "error": str(error)}]
    raise ValueError("missing stage negative control unexpectedly passed")


def _case(
    probe: object,
    graph: object,
    tapped: object,
    taps: list[dict],
    *,
    prefix: np.ndarray,
    context: np.ndarray,
    block_index: int = 0,
    include_negatives: bool = False,
) -> dict:
    wrapper = importlib.import_module("caption_export.wrappers").Decoder(probe.model)
    unhooked = wrapper(probe.torch.from_numpy(prefix), probe.torch.from_numpy(context)).numpy().copy()
    capture = capture_native(probe.model, prefix, context, block_index=block_index)
    native = probe.native_decoder(prefix, context)
    feed = {"input_ids": prefix, "encoder_hidden_states": context}
    original_logits = graph.run(["logits"], feed)[0]
    values = tapped.run([*[tap["tensor"] for tap in taps], "logits"], feed)
    require(len(values) == 17, "subblock ORT outlet count changed")
    actual = dict(zip(STAGES, values[:16], strict=True))
    length = prefix.shape[1]
    for stage, width in zip(STAGES, MULTIPLIERS, strict=True):
        for value in (capture["captures"][stage], actual[stage]):
            probe.tensor(value, (1, length, 768 * width))
    baseline = compare_tensor(native, original_logits, atol=ATOL, rtol=RTOL)
    require(baseline["passed"] == (length == 1), "BASELINE_NOT_REPRODUCED: expected T1 pass and T2 failure")
    result = {
        "case": f"zeros_t{length}",
        "comparisons": compare_taps(capture["captures"], actual, prefix_length=length),
        "baseline": baseline,
        "hook_calls": capture["calls"],
        "hook_implementations": capture["module_receipt"],
        "controls": {
            "mask": compare_tensor(native, unhooked, atol=ATOL, rtol=RTOL),
            "hf_observer": compare_tensor(unhooked, capture["logits"], atol=ATOL, rtol=RTOL),
            "ort_observer": compare_tensor(original_logits, values[-1], atol=ATOL, rtol=RTOL),
        },
    }
    if include_negatives:
        result["negative_controls"] = _negative_controls(capture["captures"], actual)
    return result


def _counts(cases: list[dict], taps: list[dict]) -> dict:
    case_lengths([case["case"] for case in cases])
    hooks = {sum(case["hook_calls"].values()) for case in cases}
    require(len(hooks) == 1, "native hook counts differ between cases")
    return {
        "cases": len(cases),
        "paired_stage_comparisons": sum(len(case["comparisons"]) for case in cases),
        "compared_stage_elements": sum(row["compared_elements"] for case in cases for row in case["comparisons"]),
        "hooks_per_case": hooks.pop(),
        "ort_stage_outlets": len(taps),
        "ort_total_outlets_including_observer_logits": len(taps) + 1,
        "stage_elements_T1": sum(
            row["compared_elements"] for case in cases if case["case"] == "zeros_t1" for row in case["comparisons"]
        ),
        "stage_elements_T2": sum(
            row["compared_elements"] for case in cases if case["case"] == "zeros_t2" for row in case["comparisons"]
        ),
    }


def _observe_cases(
    probe: object, graph: object, tapped: object, taps: list, *, original_result: dict, protocol: dict
) -> list:
    lengths = case_lengths(protocol["cases"])
    ids, pixels = frozen_inputs(original_result)
    with probe.torch.inference_mode():
        context = probe.model.encoder(
            pixel_values=probe.torch.from_numpy(pixels), return_dict=True
        ).last_hidden_state.numpy()
        probe.tensor(context, (1, 197, 768))
        require(
            hashlib.sha256(context.tobytes()).hexdigest() == PROTOCOL["context_sha256"],
            "frozen context SHA changed",
        )
        return [
            _case(
                probe,
                graph,
                tapped,
                taps,
                prefix=ids[:, :length].copy(),
                context=context,
                block_index=protocol.get("block_index", 0),
                include_negatives=length == lengths[-1],
            )
            for length in lengths
        ]


def execute(
    plan: dict, output: Path, base: object, original_plan: dict, *, original_result: dict, previous: dict
) -> dict:
    """Use one existing reference and only the frozen block and case selections."""
    probe = base.Probe(original_plan, {})
    probe.load_reference()
    layers = original_plan["geometry"]["layers"]
    require(
        len(probe.model.decoder.transformer.h) == probe.model.decoder.config.n_layer == layers,
        "reference decoder layer count differs from original metadata",
    )
    protocol = plan["protocol"]
    implementations = _reference_code(probe.model, plan)
    diagnostic_graph, taps = _prepare_graph(original_plan, previous, output, block_index=protocol.get("block_index", 0))
    graph = session(probe.ort, Path(original_plan["bundle"]) / "model/decoder.onnx", ["logits"])
    tapped = session(probe.ort, diagnostic_graph, [*[tap["tensor"] for tap in taps], "logits"])
    try:
        cases = _observe_cases(probe, graph, tapped, taps, original_result=original_result, protocol=protocol)
    finally:
        del graph, tapped
    controls = {name: [case["controls"][name] for case in cases] for name in ("mask", "hf_observer", "ort_observer")}
    validate_controls(controls, expected_cases=len(protocol["cases"]))
    summary = summarize(cases, original_result=original_result, case_names=protocol["cases"])
    counts = _counts(cases, taps)
    require(counts == plan["expected_counts"], "subblock actual counts differ from prospective protocol")
    negatives = cases[-1]["negative_controls"]
    actual_controls = {
        "HF_default_mask_vs_wrapper_mask_bit_equal": len(controls["mask"]),
        "unhooked_HF_vs_hooked_wrapper_logits_bit_equal": len(controls["hf_observer"]),
        "original_vs_instrumented_ort_logits_bit_equal": len(controls["ort_observer"]),
        "poisoned_element_correct_failure": sum(row["control"] == "poisoned_element" for row in negatives),
        "missing_stage_correct_rejection": sum(row["control"] == "missing_stage" for row in negatives),
    }
    require(actual_controls == plan["expected_controls"], "subblock actual controls differ from prospective protocol")
    return {
        **summary,
        "stage_comparison_counts": summary["actual_counts"],
        "actual_counts": counts,
        "actual_controls": actual_controls,
        "cases": cases,
        "taps": taps,
        "diagnostic_graph_sha256": digest(diagnostic_graph),
        "context_sha256": PROTOCOL["context_sha256"],
        "reference_loading": probe.report,
        "implementation_sources": implementations,
    }


def main() -> int:
    """Validate all source/output guards before loading any reference weights."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--plan", required=True, type=Path)
    parser.add_argument("--plan-sha256", required=True)
    parser.add_argument("--output", required=True, type=Path)
    arguments = parser.parse_args()
    verify_runtime_versions()
    require(digest(arguments.plan) == arguments.plan_sha256, "subblock plan SHA mismatch")
    plan = read_json(arguments.plan)
    offline_environment()
    base, original_plan, original_result, previous = verify_plan(plan)
    verify_destination(arguments.output, plan, original_plan, arguments.plan)
    arguments.output.mkdir(parents=True, exist_ok=False)
    (arguments.output / "plan.json").write_bytes(arguments.plan.read_bytes())
    require(digest(arguments.output / "plan.json") == arguments.plan_sha256, "copied plan SHA mismatch")
    report = {
        "status": "INCOMPLETE",
        "qualified": False,
        "diagnostic_complete": False,
        "plan_sha256": arguments.plan_sha256,
        "protocol": plan["protocol"],
        "original_result": copy_original(original_result),
    }
    try:
        report.update(
            execute(plan, arguments.output, base, original_plan, original_result=original_result, previous=previous)
        )
        report["status"] = "DIAGNOSTIC_COMPLETE_WITH_ORIGINAL_QUALIFICATION_FAILED"
    except Exception as error:
        report.update(status="INCOMPLETE", diagnostic_complete=False, error=f"{type(error).__name__}: {error}")
        raise
    finally:
        try:
            verify_plan(plan)
        except Exception as error:
            report.update(
                status="INCOMPLETE", diagnostic_complete=False, verification_error=f"{type(error).__name__}: {error}"
            )
            raise
        finally:
            with (arguments.output / "result.json").open("x") as stream:
                json.dump(report, stream, indent=2, sort_keys=True)
                stream.write("\n")
    return 0


def copy_original(original_result: dict) -> dict:
    """Retain a separate immutable JSON receipt even before execution finishes."""
    return json.loads(json.dumps(original_result))


if __name__ == "__main__":
    raise SystemExit(main())
