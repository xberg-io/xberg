"""Freeze new subblock execution separately from immutable original evidence."""

import argparse
import copy
import importlib.metadata
import json
import sys
from pathlib import Path

from .assets import digest, read_json
from .first_divergence_evidence import require, verify_original, verify_output, verify_runtime
from .first_divergence_plan import freeze_plan as freeze_original_evidence
from .provenance import verify_runtime_versions
from .subblock import MULTIPLIERS, STAGES, case_lengths, validate_block

PROTOCOL = {
    "atol": 1e-5,
    "rtol": 1e-4,
    "cases": ["zeros_t1", "zeros_t2"],
    "context_sha256": "1652e285e42f8f5c997ef53b51bcd78ca978a17d33a6c061351b04f4bc82272f",
    "hidden": 768,
    "mlp": 3072,
    "image": "same frozen zeros",
    "inter_threads": 1,
    "intra_threads": 1,
    "ort_execution": "ORT_SEQUENTIAL",
    "ort_optimization": "ORT_DISABLE_ALL",
    "prefix_lengths": [1, 2],
    "provider": "CPUExecutionProvider",
    "reference": "same verified CPU FP32 eager/no-cache HF model",
    "token_rng_seed": 2337,
}
EXPECTED_COUNTS = {
    "cases": 2,
    "compared_stage_elements": 55296,
    "hooks_per_case": 16,
    "ort_stage_outlets": 16,
    "ort_total_outlets_including_observer_logits": 17,
    "paired_stage_comparisons": 32,
    "stage_elements_T1": 18432,
    "stage_elements_T2": 36864,
}
EXPECTED_CONTROLS = {
    "HF_default_mask_vs_wrapper_mask_bit_equal": 2,
    "missing_stage_correct_rejection": 1,
    "original_vs_instrumented_ort_logits_bit_equal": 2,
    "poisoned_element_correct_failure": 1,
    "unhooked_HF_vs_hooked_wrapper_logits_bit_equal": 2,
}
PRIOR_COUNTS = {"cases": 2, "stages": 30, "compared_elements": 183027}
PRIOR_CONTROLS = {"mask": 2, "negative": 2, "observer": 2}


def selection_counts(protocol: dict) -> tuple[dict, dict]:
    """Validate selection without changing any frozen numerical or runtime policy."""
    lengths = case_lengths(protocol.get("cases"))
    block = protocol.get("block_index", 0)
    require(type(block) is int and block >= 0, "invalid decoder block selection")
    expected = copy.deepcopy(PROTOCOL)
    expected.update(cases=[f"zeros_t{length}" for length in lengths], prefix_lengths=lengths)
    if "block_index" in protocol:
        expected["block_index"] = block
    require(protocol == expected, "unsupported subblock protocol or case prefix selection")
    require(all(type(length) is int for length in protocol["prefix_lengths"]), "invalid case prefix metadata")
    elements = sum(MULTIPLIERS) * PROTOCOL["hidden"]
    counts = {
        **EXPECTED_COUNTS,
        "cases": len(lengths),
        "paired_stage_comparisons": len(lengths) * len(STAGES),
        "compared_stage_elements": sum(lengths) * elements,
        "stage_elements_T1": elements if 1 in lengths else 0,
        "stage_elements_T2": 2 * elements if 2 in lengths else 0,
    }
    controls = {
        name: 1 if name in {"missing_stage_correct_rejection", "poisoned_element_correct_failure"} else len(lengths)
        for name in EXPECTED_CONTROLS
    }
    return counts, controls


def _protocol_counts(data: dict) -> None:
    counts, controls = selection_counts(data["protocol"])
    require(
        data["expected_counts"] == counts
        and data["expected_controls"] == controls
        and all(
            type(value) is int for value in (*data["expected_counts"].values(), *data["expected_controls"].values())
        ),
        "subblock protocol counts changed",
    )


def _layer_bounds(protocol: dict, original_plan: dict) -> None:
    layers = original_plan["geometry"]["layers"]
    validate_block(protocol.get("block_index", 0), layers)


def _verify_file(pin: dict, label: str) -> Path:
    path = Path(pin["path"])
    require(path.is_file() and not path.is_symlink(), f"missing regular {label} file")
    require(digest(path) == pin["sha256"], f"{label} source checksum mismatch")
    return path


def _prospective(path: Path, checksum: str) -> dict:
    require(path.is_file() and not path.is_symlink() and digest(path) == checksum, "prospective SHA mismatch")
    data = read_json(path)
    require(data["version"] == 1, "unsupported subblock protocol version")
    _protocol_counts(data)
    stages = data["stages"]
    require([stage["stage"] for stage in stages] == list(STAGES), "subblock stage inventory changed")
    require(
        [stage["shape"] for stage in stages] == [[1, "T", width * 768] for width in MULTIPLIERS],
        "subblock stage width/shape changed",
    )
    require(data["original_qualification_must_remain"] is False, "original qualification cannot change")
    failed = [[image, length] for image in ("zeros", "uniform") for length in (2, 8, 16)]
    require(data["original_failed_prefixes"] == failed, "original six failed prefixes changed")
    require(
        data["prior_counts"] == PRIOR_COUNTS and data["prior_controls"] == PRIOR_CONTROLS,
        "prior diagnostic protocol changed",
    )
    root = Path(__file__).resolve().parent
    original_names = {
        str(path.relative_to(root)) for path in root.rglob("*.py") if not path.name.startswith("subblock")
    }
    require(
        len(data["runtime_sources"]) == 15 and set(data["runtime_sources"]) == original_names,
        "original runtime inventory incomplete",
    )
    for name, value in data["runtime_sources"].items():
        target = root / name
        require(
            target.resolve().is_relative_to(root) and not target.is_symlink() and digest(target) == value,
            "original runtime source changed",
        )
    _verify_file(data["gpt2_source"], "GPT2")
    return data


def _prior(pin: dict) -> dict:
    previous = read_json(_verify_file(pin, "prior diagnostic"))
    require(
        previous["qualified"] is False and previous["diagnostic_complete"] is True,
        "prior diagnostic must remain complete and unqualified",
    )
    require(
        previous["status"] == "DIAGNOSTIC_COMPLETE_WITH_ORIGINAL_QUALIFICATION_FAILED",
        "prior diagnostic status changed",
    )
    require(
        previous["actual_counts"] == PRIOR_COUNTS and previous["actual_controls"] == PRIOR_CONTROLS,
        "prior diagnostic counts changed",
    )
    require(previous["context_sha256"] == PROTOCOL["context_sha256"], "frozen context SHA changed")
    require(previous["original_result"]["status"] == "FAIL", "original qualification must remain failed")
    return previous


def freeze_plan(prospective: Path, prospective_sha256: str, original: Path) -> dict:
    """Pin source and old receipts without importing the original helper or model."""
    data = _prospective(prospective, prospective_sha256)
    _layer_bounds(data["protocol"], read_json(original / "plan.json"))
    _prior(data["authorized_receipt"])
    original_evidence = freeze_original_evidence(original)
    verify_runtime_versions()
    conv = Path(importlib.metadata.distribution("transformers").locate_file("transformers/pytorch_utils.py"))
    require(conv.is_file() and not conv.is_symlink(), "missing regular Conv1D implementation source")
    return {
        "version": 1,
        "source_root": original_evidence["source_root"],
        "runtime_sources": dict(original_evidence["runtime_sources"]),
        "prospective": {"path": str(prospective.resolve()), "sha256": prospective_sha256},
        "protocol": copy.deepcopy(data["protocol"]),
        "expected_counts": dict(data["expected_counts"]),
        "expected_controls": dict(data["expected_controls"]),
        "prior_receipt": copy.deepcopy(data["authorized_receipt"]),
        "dependency_sources": {
            "gpt2": copy.deepcopy(data["gpt2_source"]),
            "conv1d": {"path": str(conv.resolve()), "sha256": digest(conv)},
        },
        "original_evidence_purpose": "immutable_original_verification_only_not_subblock_execution",
        "original_evidence": original_evidence,
    }


def verify_plan(plan: dict) -> tuple:
    """Validate the new execution protocol before the strict original-evidence view."""
    require(plan["version"] == 1, "unsupported subblock execution protocol version")
    _protocol_counts(plan)
    verify_runtime(plan)
    data = _prospective(Path(plan["prospective"]["path"]), plan["prospective"]["sha256"])
    require(plan["protocol"] == data["protocol"], "execution case/block selection differs from prospective pin")
    require(plan["prior_receipt"] == data["authorized_receipt"], "prior receipt differs from prospective pin")
    require(
        set(plan["dependency_sources"]) == {"gpt2", "conv1d"}
        and plan["dependency_sources"]["gpt2"] == data["gpt2_source"],
        "dependency source pins changed",
    )
    for label, pin in plan["dependency_sources"].items():
        _verify_file(pin, label)
    evidence = plan["original_evidence"]
    require(
        evidence["source_root"] == plan["source_root"] and evidence["runtime_sources"] == plan["runtime_sources"],
        "original evidence source view differs from actual runner",
    )
    previous = _prior(plan["prior_receipt"])
    base, original_plan, original_result = verify_original(evidence)
    _layer_bounds(plan["protocol"], original_plan)
    require(previous["original_result"] == original_result, "prior diagnostic original result changed")
    return base, original_plan, original_result, previous


def verify_destination(output: Path, plan: dict, original_plan: dict, plan_path: Path) -> None:
    """Protect original inputs, previous diagnostics and both frozen plans."""
    verify_output(output, plan["original_evidence"], original_plan, plan_path)
    for pin in (plan["prospective"], plan["prior_receipt"]):
        protected = Path(pin["path"]).resolve()
        require(
            not output.resolve().is_relative_to(protected) and not protected.is_relative_to(output.resolve()),
            "output overlaps immutable diagnostic evidence",
        )


def main() -> None:
    """Write a new source-bound plan exclusively; emit its exact SHA for execution."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--prospective", required=True, type=Path)
    parser.add_argument("--prospective-sha256", required=True)
    parser.add_argument("--original", required=True, type=Path)
    parser.add_argument("--plan", required=True, type=Path)
    arguments = parser.parse_args()
    plan = freeze_plan(arguments.prospective, arguments.prospective_sha256, arguments.original)
    original_plan = read_json(arguments.original / "plan.json")
    verify_destination(arguments.plan, plan, original_plan, arguments.prospective)
    with arguments.plan.open("x") as stream:
        json.dump(plan, stream, indent=2, sort_keys=True)
        stream.write("\n")
    sys.stdout.write(digest(arguments.plan) + "\n")


if __name__ == "__main__":
    main()
