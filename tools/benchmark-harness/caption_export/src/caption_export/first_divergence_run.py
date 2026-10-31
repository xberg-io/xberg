"""Run a frozen T1/T2 decoder diagnostic; never export or qualify a model."""

import argparse
import hashlib
import importlib
import json
import os
from collections.abc import Callable
from pathlib import Path
from types import ModuleType

import numpy as np
import onnx

from .assets import digest, read_json
from .first_divergence import (
    ATOL,
    RTOL,
    STAGES,
    compare_taps,
    compare_tensor,
    instrument_graph,
    resolve_taps,
    summarize,
)
from .first_divergence_evidence import (
    PROTOCOL,
    hook_modules,
    require,
    runtime_receipt,
    verify_original,
    verify_output,
    verify_runtime,
)


def offline_environment() -> None:
    """Disable remote loading and bound CPU parallelism before importing Torch."""
    for key in (
        "HF_HUB_OFFLINE",
        "TRANSFORMERS_OFFLINE",
        "HF_HUB_DISABLE_TELEMETRY",
        "OMP_NUM_THREADS",
        "MKL_NUM_THREADS",
    ):
        os.environ[key] = "1"
    os.environ["TOKENIZERS_PARALLELISM"] = "false"


def session(ort: ModuleType, path: Path, outputs: list[str]) -> object:
    """Keep original and instrumented sessions on the same predeclared protocol."""
    options = ort.SessionOptions()
    options.intra_op_num_threads = options.inter_op_num_threads = 1
    options.execution_mode = ort.ExecutionMode.ORT_SEQUENTIAL
    options.graph_optimization_level = ort.GraphOptimizationLevel.ORT_DISABLE_ALL
    options.enable_profiling = False
    require(options.optimized_model_filepath == "", "optimized model writes are forbidden")
    result = ort.InferenceSession(str(path), options, providers=["CPUExecutionProvider"])
    require(result.get_providers() == ["CPUExecutionProvider"], "unexpected ORT execution provider")
    require([port.name for port in result.get_inputs()] == ["input_ids", "encoder_hidden_states"], "decoder IO changed")
    require([port.name for port in result.get_outputs()] == outputs, "decoder outlets changed")
    return result


def frozen_inputs(previous: dict) -> tuple[np.ndarray, np.ndarray]:
    """Reproduce exact existing development inputs, without selecting new data."""
    ids = np.random.default_rng(2337).integers(0, 50257, size=(1, 16), dtype=np.int64)
    ids[0, 0] = 50256
    pixels = np.zeros((1, 3, 224, 224), dtype=np.float32)
    for name, value in (("token_prefix", ids), ("zeros", pixels)):
        require(
            hashlib.sha256(value.tobytes()).hexdigest() == previous["input_sha256"][name], "frozen input SHA mismatch"
        )
    return ids, pixels


def capture_native(probe: object, prefix: np.ndarray, context: np.ndarray) -> dict:
    """Observe one wrapper call with exactly one invocation at every module seam."""
    captures, calls, handles = {}, {}, []

    def hook(stage: str) -> Callable:
        def observe(_module: object, _arguments: tuple, value: object) -> None:
            calls[stage] = calls.get(stage, 0) + 1
            require(isinstance(value, probe.torch.Tensor), f"non-tensor hook value: {stage}")
            captures[stage] = value.detach().cpu().contiguous().numpy().copy()

        return observe

    for stage, module in hook_modules(probe.model).items():
        handles.append(module.register_forward_hook(hook(stage)))
    try:
        decoder_type = importlib.import_module("caption_export.wrappers").Decoder
        output = decoder_type(probe.model)(probe.torch.from_numpy(prefix), probe.torch.from_numpy(context))
        require(calls == dict.fromkeys(STAGES, 1), "hook count must be exactly one per stage")
        require(output.numpy().tobytes() == captures["logits"].tobytes(), "logit hook differs from returned output")
    finally:
        for handle in handles:
            handle.remove()
    return captures


def negative_controls(expected: dict, actual: dict) -> list[dict]:
    """Prove wrong values and missing work fail at their intended checks."""
    poisoned = {stage: value.copy() for stage, value in expected.items()}
    poisoned["embedding"][0, 0, 0] += np.float32(1.0)
    rows = compare_taps(expected, poisoned, order=STAGES)
    failures = [row for row in rows if not row["passed"]]
    require(len(failures) == 1 and failures[0]["stage"] == "embedding", "poison value failed at wrong stage")
    require(
        failures[0]["failed_elements"] == 1 and failures[0]["first_failed_index"] == [0, 0, 0],
        "poison value failed at wrong coordinate",
    )
    missing = dict(actual)
    del missing["block.4"]
    try:
        compare_taps(expected, missing, order=STAGES)
    except ValueError as error:
        require("missing" in str(error), "missing-stage control failed for wrong reason")
        return [{"control": "poisoned_element", **failures[0]}, {"control": "missing_stage", "error": str(error)}]
    raise ValueError("missing-stage negative control unexpectedly passed")


def probe_case(
    probe: object, graph: object, tapped: object, taps: list[dict], *, prefix: np.ndarray, context: np.ndarray
) -> dict:
    """Separate baseline, wrapper, observer and stage-level numerical evidence."""
    expected = capture_native(probe, prefix, context)
    native = probe.native_decoder(prefix, context)
    mask = compare_tensor(native, expected["logits"], atol=ATOL, rtol=RTOL)
    require(mask["bitwise_equal"], "default-HF versus wrapper mask control is not bit equal")
    feed = {"input_ids": prefix, "encoder_hidden_states": context}
    original = graph.run(["logits"], feed)[0]
    values = tapped.run([tap["tensor"] for tap in taps], feed)
    require(len(values) == len(STAGES), "ORT tap count differs from expected")
    actual = dict(zip(STAGES, values, strict=True))
    observer = compare_tensor(original, actual["logits"], atol=ATOL, rtol=RTOL)
    require(observer["bitwise_equal"], "added outlets changed original ORT logits")
    baseline = compare_tensor(native, original, atol=ATOL, rtol=RTOL)
    length = prefix.shape[1]
    require(baseline["passed"] == (length == 1), "BASELINE_NOT_REPRODUCED: expected T1 pass and T2 failure")
    for stage in STAGES:
        shape = (1, length, 50257 if stage == "logits" else 768)
        probe.tensor(expected[stage], shape)
        probe.tensor(actual[stage], shape)
    rows = compare_taps(expected, actual, order=STAGES)
    result = {"case": f"zeros_t{length}", "comparisons": rows, "baseline": baseline, "mask": mask, "observer": observer}
    if length == 2:
        result["negative_controls"] = negative_controls(expected, actual)
    return result


def prepare_graph(original_plan: dict, output: Path) -> tuple[Path, list[dict]]:
    """Append existing tensor outlets to a new graph only; never re-export."""
    source = Path(original_plan["bundle"]) / "model/decoder.onnx"
    graph = onnx.load(source, load_external_data=False)
    require(
        not any(tensor.data_location == onnx.TensorProto.EXTERNAL for tensor in graph.graph.initializer),
        "external diagnostic weights unsupported",
    )
    taps = resolve_taps(graph)
    instrumented = instrument_graph(graph, taps, hidden_size=PROTOCOL["hidden"])
    require(
        [node.SerializeToString() for node in graph.graph.node]
        == [node.SerializeToString() for node in instrumented.graph.node],
        "diagnostic nodes changed",
    )
    require(
        [tensor.SerializeToString() for tensor in graph.graph.initializer]
        == [tensor.SerializeToString() for tensor in instrumented.graph.initializer],
        "diagnostic weights changed",
    )
    destination = output / "decoder-taps.onnx"
    onnx.save_model(instrumented, destination, save_as_external_data=False)
    onnx.checker.check_model(str(destination))
    return destination, taps


def execute(plan: dict, output: Path, base: ModuleType, original_plan: dict, previous: dict) -> dict:
    """Run the two frozen prefixes with one HF context and two fixed sessions."""
    probe = base.Probe(original_plan, {})
    probe.load_reference()
    diagnostic_graph, taps = prepare_graph(original_plan, output)
    graph = session(probe.ort, Path(original_plan["bundle"]) / "model/decoder.onnx", ["logits"])
    tapped = session(probe.ort, diagnostic_graph, [tap["tensor"] for tap in taps])
    ids, pixels = frozen_inputs(previous)
    with probe.torch.inference_mode():
        context = probe.model.encoder(
            pixel_values=probe.torch.from_numpy(pixels), return_dict=True
        ).last_hidden_state.numpy()
        probe.tensor(context, (1, 197, 768))
        cases = [
            probe_case(probe, graph, tapped, taps, prefix=ids[:, :length].copy(), context=context) for length in (1, 2)
        ]
    summary = summarize(cases, original_result=previous, expected_cases=2, expected_stages=STAGES)
    require(
        summary["actual_counts"] == PROTOCOL["expected_counts"], "diagnostic element counts differ from frozen plan"
    )
    controls = {
        "mask": sum(case["mask"]["bitwise_equal"] for case in cases),
        "observer": sum(case["observer"]["bitwise_equal"] for case in cases),
        "negative": sum(len(case.get("negative_controls", [])) for case in cases),
    }
    require(controls == PROTOCOL["expected_controls"], "diagnostic control counts differ from frozen plan")
    verify_runtime(plan)
    base.preflight(original_plan, Path(plan["original_directory"]))
    return {
        **summary,
        "cases": cases,
        "actual_controls": controls,
        "reference_loading": probe.report,
        "hook_implementations": runtime_receipt(probe.model),
        "taps": taps,
        "diagnostic_graph_sha256": digest(diagnostic_graph),
        "context_sha256": hashlib.sha256(context.tobytes()).hexdigest(),
    }


def main() -> int:
    """Accept an externally frozen plan and preserve incomplete diagnostic receipts."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--plan", required=True, type=Path)
    parser.add_argument("--plan-sha256", required=True)
    parser.add_argument("--output", required=True, type=Path)
    arguments = parser.parse_args()
    require(digest(arguments.plan) == arguments.plan_sha256, "diagnostic plan SHA mismatch")
    plan = read_json(arguments.plan)
    offline_environment()
    base, original_plan, previous = verify_original(plan)
    verify_output(arguments.output, plan, original_plan, arguments.plan)
    arguments.output.mkdir(parents=True, exist_ok=False)
    with (arguments.output / "plan.json").open("xb") as stream:
        stream.write(arguments.plan.read_bytes())
    require(digest(arguments.output / "plan.json") == arguments.plan_sha256, "copied plan SHA mismatch")
    report = {
        "status": "INCOMPLETE",
        "qualified": False,
        "diagnostic_complete": False,
        "plan_sha256": arguments.plan_sha256,
        "protocol": PROTOCOL,
        "original_result": previous,
        "original_files": plan["original_files"],
    }
    try:
        report.update(execute(plan, arguments.output, base, original_plan, previous))
        verify_original(plan)
        report["status"] = "DIAGNOSTIC_COMPLETE_WITH_ORIGINAL_QUALIFICATION_FAILED"
    except Exception as error:
        report.update(status="INCOMPLETE", diagnostic_complete=False, error=f"{type(error).__name__}: {error}")
        raise
    finally:
        with (arguments.output / "result.json").open("x") as stream:
            json.dump(report, stream, indent=2, sort_keys=True)
            stream.write("\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
