"""Validate real freeze/run entrypoints using synthetic evidence, never a model."""

import copy
import hashlib
import json
import os
import subprocess
import sys
from pathlib import Path

import pytest

from conftest import hashes, write_json
from subblock_fixtures import MULTIPLIERS, STAGES, feature

CONTEXT_SHA = "1652e285e42f8f5c997ef53b51bcd78ca978a17d33a6c061351b04f4bc82272f"
PROTOCOL = {
    "atol": 1e-5,
    "rtol": 1e-4,
    "cases": ["zeros_t1", "zeros_t2"],
    "context_sha256": CONTEXT_SHA,
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
COUNTS = {
    "cases": 2,
    "compared_stage_elements": 55296,
    "hooks_per_case": 16,
    "ort_stage_outlets": 16,
    "ort_total_outlets_including_observer_logits": 17,
    "paired_stage_comparisons": 32,
    "stage_elements_T1": 18432,
    "stage_elements_T2": 36864,
}


def checksum(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def synthetic_original(directory):
    original = directory / "original"
    original.mkdir()
    for name in ("source", "bundle"):
        (directory / name).mkdir()
        (directory / name / "synthetic.txt").write_text("no model asset\n")
    plan = {
        "atol": 1e-5,
        "rtol": 1e-4,
        "expected_counts": {"prefix": 8},
        "geometry": {"image": 224, "context": 197, "hidden": 768, "vocabulary": 50257, "layers": 12},
        "source": str(directory / "source"),
        "bundle": str(directory / "bundle"),
    }
    result = {
        "status": "FAIL",
        "qualified": False,
        "actual_counts": {"prefix": 8},
        "prefix": [
            {"image": image, "length": length, "passed": length == 1}
            for image in ("zeros", "uniform")
            for length in (1, 2, 8, 16)
        ],
    }
    write_json(original / "plan.json", plan)
    write_json(original / "result.json", result)
    (original / "parity.py").write_text(
        'PRODUCER = "synthetic-pinned-source"\nPREFLIGHT_CALLS = 0\n'
        "def preflight(plan, directory):\n"
        '    global PREFLIGHT_CALLS\n    assert plan["expected_counts"]["prefix"] == 8\n'
        "    PREFLIGHT_CALLS += 1\n"
    )
    return original, result


@pytest.fixture
def evidence(tmp_path):
    original, result = synthetic_original(tmp_path)
    prior = tmp_path / "prior-result.json"
    write_json(
        prior,
        {
            "qualified": False,
            "diagnostic_complete": True,
            "status": "DIAGNOSTIC_COMPLETE_WITH_ORIGINAL_QUALIFICATION_FAILED",
            "actual_counts": {"cases": 2, "stages": 30, "compared_elements": 183027},
            "actual_controls": {"mask": 2, "negative": 2, "observer": 2},
            "context_sha256": CONTEXT_SHA,
            "original_result": result,
        },
    )
    gpt2 = tmp_path / "modeling_gpt2.py"
    gpt2.write_text("# Synthetic source-file pin, not a model implementation.\n")
    package = Path(__file__).resolve().parents[1] / "src" / "caption_export"
    sources = {p.name: checksum(p) for p in package.glob("*.py") if not p.name.startswith("subblock")}
    assert len(sources) == 15
    prospective = {
        "version": 1,
        "status": "PROSPECTIVE_PROTOCOL_ONLY_NOT_IMPLEMENTED_OR_EXECUTED",
        "producer_head": "e2312ecbad2a4692d09956480f5e0468f73aad10",
        "authorized_receipt": {"path": str(prior), "sha256": checksum(prior)},
        "original_qualification_must_remain": False,
        "original_failed_prefixes": [[image, length] for image in ("zeros", "uniform") for length in (2, 8, 16)],
        "prior_controls": {"mask": 2, "negative": 2, "observer": 2},
        "prior_counts": {"cases": 2, "compared_elements": 183027, "stages": 30},
        "protocol": PROTOCOL,
        "expected_counts": COUNTS,
        "expected_controls": {
            "HF_default_mask_vs_wrapper_mask_bit_equal": 2,
            "missing_stage_correct_rejection": 1,
            "original_vs_instrumented_ort_logits_bit_equal": 2,
            "poisoned_element_correct_failure": 1,
            "unhooked_HF_vs_hooked_wrapper_logits_bit_equal": 2,
        },
        "gpt2_source": {"path": str(gpt2), "sha256": checksum(gpt2)},
        "runtime_sources": sources,
        "stages": [
            {"stage": stage, "shape": [1, "T", 768 * width]} for stage, width in zip(STAGES, MULTIPLIERS, strict=True)
        ],
    }
    path = tmp_path / "prospective.json"
    write_json(path, prospective)
    assert len(prospective["stages"]) == 16
    assert sum(stage["shape"][2] for stage in prospective["stages"]) * 3 == 55296
    return path, original, prospective


def command(module, *arguments):
    project = Path(__file__).resolve().parents[1]
    environment = dict(os.environ, PYTHONPATH=str(project / "src"), PYTHONDONTWRITEBYTECODE="1")
    return subprocess.run(
        [sys.executable, "-m", f"caption_export.{module}", *map(str, arguments)],
        cwd=project,
        env=environment,
        capture_output=True,
        text=True,
        timeout=30,
        check=False,
    )


def test_freeze_cli_binds_new_sources_and_keeps_evidence_protocol_separate(monkeypatch, evidence, tmp_path):
    prospective, original, _ = evidence
    before = hashes(original)
    api = feature(monkeypatch, "subblock_plan", ("freeze_plan",))
    plan = api.freeze_plan(prospective, checksum(prospective), original)
    assert plan["protocol"] == PROTOCOL and plan["expected_counts"] == COUNTS
    assert plan["original_evidence"]["protocol"]["expected_counts"]["stages"] == 30
    assert plan["original_evidence"]["protocol"] != plan["protocol"]
    assert plan["source_root"] == str(Path(api.__file__).resolve().parent)
    for name in ("subblock.py", "subblock_hooks.py", "subblock_plan.py", "subblock_run.py"):
        assert plan["runtime_sources"][name] == checksum(Path(plan["source_root"]) / name)
    output = tmp_path / "frozen.json"
    arguments = (
        "--prospective",
        prospective,
        "--prospective-sha256",
        checksum(prospective),
        "--original",
        original,
        "--plan",
        output,
    )
    result = command("subblock_plan", *arguments)
    assert result.returncode == 0, result.stderr
    assert json.loads(output.read_text()) == plan and result.stdout.strip() == checksum(output)
    previous = output.read_bytes()
    repeated = command("subblock_plan", *arguments)
    assert repeated.returncode != 0 and "exist" in repeated.stderr.lower()
    assert output.read_bytes() == previous and hashes(original) == before


def test_freeze_checks_prospective_sha_before_loading_original_helper(monkeypatch, evidence):
    from caption_export.first_divergence_evidence import load_original_helper

    prospective, original, _ = evidence
    activity = original.parent / "helper-activity.txt"
    helper = original / "parity.py"
    instrumented = helper.read_text().replace(
        "    PREFLIGHT_CALLS += 1\n", "    PREFLIGHT_CALLS += 1\n    _observe('preflight')\n"
    )
    instrumented += (
        "from pathlib import Path\n"
        f"_ACTIVITY = Path({str(activity)!r})\n"
        "def _observe(event):\n"
        "    previous = _ACTIVITY.read_text() if _ACTIVITY.exists() else ''\n"
        "    _ACTIVITY.write_text(previous + event + '\\n')\n"
        "_observe('load')\n"
    )
    helper.write_text(instrumented)
    control = load_original_helper(helper, expected_sha256=checksum(helper))
    control.preflight(json.loads((original / "plan.json").read_text()), original)
    assert activity.read_text().splitlines() == ["load", "preflight"]
    assert control.PREFLIGHT_CALLS == 1
    activity.unlink()
    before = hashes(original)
    api = feature(monkeypatch, "subblock_plan", ("freeze_plan",))
    with pytest.raises(ValueError, match=r"(?i)prospective.*(sha|checksum)"):
        api.freeze_plan(prospective, "0" * 64, original)
    assert not activity.exists(), "reject wrong prospective SHA before loading or invoking the original helper"
    assert hashes(original) == before


@pytest.mark.parametrize("broken", ["stage_count", "stage_width"])
def test_new_protocol_rejects_incomplete_stages_or_wrong_width_before_capture(monkeypatch, evidence, broken):
    prospective, original, data = evidence
    data = copy.deepcopy(data)
    if broken == "stage_count":
        data["stages"].pop()
    else:
        data["stages"][2]["shape"][2] = 768
    write_json(prospective, data)
    api = feature(monkeypatch, "subblock_plan", ("freeze_plan",))
    with pytest.raises(ValueError, match=r"(?i)(stage|width|shape|protocol)"):
        api.freeze_plan(prospective, checksum(prospective), original)


def test_common_original_guard_reads_source_without_pyc_and_binds_actual_new_runner(monkeypatch, evidence):
    from caption_export.first_divergence_evidence import verify_original

    prospective, original, _ = evidence
    before = hashes(original)
    api = feature(monkeypatch, "subblock_plan", ("freeze_plan",))
    plan = api.freeze_plan(prospective, checksum(prospective), original)
    base, _, previous = verify_original(plan["original_evidence"])
    assert base.PRODUCER == "synthetic-pinned-source" and base.PREFLIGHT_CALLS == 1
    assert previous["status"] == "FAIL" and hashes(original) == before
    altered = copy.deepcopy(plan["original_evidence"])
    altered["runtime_sources"]["subblock_run.py"] = "0" * 64
    with pytest.raises(ValueError, match=r"(?i)runtime.*(inventory|source)"):
        verify_original(altered)
    assert hashes(original) == before and not (original / "__pycache__").exists()


@pytest.mark.parametrize("broken", ["existing", "inside_original"])
def test_run_cli_refuses_output_mutation_before_any_model_execution(monkeypatch, evidence, tmp_path, broken):
    prospective, original, _ = evidence
    api = feature(monkeypatch, "subblock_plan", ("freeze_plan",))
    plan = api.freeze_plan(prospective, checksum(prospective), original)
    frozen = tmp_path / "frozen.json"
    write_json(frozen, plan)
    output = tmp_path / "occupied" if broken == "existing" else original / "new-output"
    if broken == "existing":
        output.mkdir()
        (output / "sentinel").write_text("retain me\n")
    before = hashes(original)
    occupied = hashes(output) if output.exists() else None
    feature(monkeypatch, "subblock_run", ("main",))
    result = command("subblock_run", "--plan", frozen, "--plan-sha256", checksum(frozen), "--output", output)
    assert result.returncode != 0
    assert ("already exists" if broken == "existing" else "overlaps immutable") in result.stderr
    assert hashes(original) == before
    if occupied is None:
        assert not output.exists()
    else:
        assert hashes(output) == occupied
