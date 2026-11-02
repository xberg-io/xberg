"""Reject diagnostic runtime drift before evidence work; no model fixtures or downloads."""

import importlib
import importlib.metadata
import json
import sys
import tomllib
from pathlib import Path

import pytest
import test_subblock_plan
from conftest import hashes, write_json
from test_subblock_plan import checksum

from caption_export import provenance

RUNTIME = ("torch", "transformers", "tokenizers", "onnx", "onnxruntime", "numpy")
PROJECT = Path(__file__).resolve().parents[1]
evidence = test_subblock_plan.evidence


def locked_versions():
    packages = tomllib.loads((PROJECT / "uv.lock").read_text())["package"]
    selected = {name: [row["version"] for row in packages if row["name"] == name] for name in RUNTIME}
    assert all(len(versions) == 1 for versions in selected.values())
    expected = {name: versions[0] for name, versions in selected.items()}
    assert {name: importlib.metadata.version(name) for name in RUNTIME} == expected
    return expected


def drift(monkeypatch, name):
    expected = locked_versions()
    assert expected[name] != "0.0.0"
    real_version = importlib.metadata.version
    monkeypatch.setattr(
        importlib.metadata, "version", lambda package: "0.0.0" if package == name else real_version(package)
    )


def forbidden(*_args, **_kwargs):
    raise AssertionError("runtime admission reached forbidden evidence/model/output work")


@pytest.mark.parametrize("name", RUNTIME)
def test_first_divergence_freeze_rejects_drift_before_source_inventory(monkeypatch, evidence, name):
    from caption_export import first_divergence_plan

    _, original, _ = evidence
    control = first_divergence_plan.freeze_plan(original)
    assert control["original_directory"] == str(original.resolve())
    assert set(control["original_files"]) == {"plan.json", "parity.py", "result.json"}
    assert control["runtime_sources"]
    before = hashes(original)
    drift(monkeypatch, name)
    monkeypatch.setattr(first_divergence_plan, "digest", forbidden)
    with pytest.raises(ValueError, match=rf"{name}.*uv\.lock"):
        first_divergence_plan.freeze_plan(original)
    assert hashes(original) == before


def frozen_plan(module, evidence):
    from caption_export import first_divergence_plan, subblock_plan

    prospective, original, _ = evidence
    if module == "first_divergence_run":
        return first_divergence_plan.freeze_plan(original)
    return subblock_plan.freeze_plan(prospective, checksum(prospective), original)


@pytest.mark.parametrize("module", ["first_divergence_run", "subblock_run"])
@pytest.mark.parametrize("name", RUNTIME)
def test_runner_main_rejects_drift_before_any_evidence_or_output_work(monkeypatch, evidence, tmp_path, module, name):
    plan = frozen_plan(module, evidence)
    path, output = tmp_path / "frozen-plan.json", tmp_path / "forbidden-output"
    write_json(path, plan)
    plan_hash = checksum(path)
    runner = importlib.import_module(f"caption_export.{module}")
    assert Path(runner.__file__).resolve().is_relative_to(PROJECT / "src")
    before = hashes(tmp_path)
    drift(monkeypatch, name)
    for symbol in ("digest", "offline_environment", "execute"):
        monkeypatch.setattr(runner, symbol, forbidden)
    monkeypatch.setattr(runner, "verify_original" if module == "first_divergence_run" else "verify_plan", forbidden)
    monkeypatch.setattr(sys, "argv", [module, "--plan", str(path), "--plan-sha256", plan_hash, "--output", str(output)])
    with pytest.raises(ValueError, match=rf"{name}.*uv\.lock"):
        runner.main()
    assert not output.exists()
    assert hashes(tmp_path) == before


@pytest.mark.parametrize("name", RUNTIME)
def test_missing_distribution_is_a_named_runtime_error(monkeypatch, name):
    locked_versions()
    real_version = importlib.metadata.version

    def missing(package):
        if package == name:
            raise importlib.metadata.PackageNotFoundError(name)
        return real_version(package)

    monkeypatch.setattr(importlib.metadata, "version", missing)
    with pytest.raises(ValueError, match=rf"{name} is missing.*uv\.lock") as raised:
        provenance.verify_runtime_versions()
    assert isinstance(raised.value.__cause__, importlib.metadata.PackageNotFoundError)


@pytest.mark.parametrize("name", RUNTIME)
@pytest.mark.parametrize("copies", [0, 2])
def test_runtime_requires_exactly_one_selected_lock_entry(monkeypatch, name, copies):
    expected = locked_versions()
    original = tomllib.loads((PROJECT / "uv.lock").read_text())
    selected = [row for row in original["package"] if row["name"] != name]
    selected.extend({"name": name, "version": expected[name]} for _ in range(copies))
    assert sum(row["name"] == name for row in selected) == copies
    monkeypatch.setattr(provenance.tomllib, "load", lambda _stream: {**original, "package": selected})
    with pytest.raises(ValueError, match=rf"{name} must have exactly one selected version in uv\.lock"):
        provenance.verify_runtime_versions()


def provenance_inputs(tmp_path):
    source, output = tmp_path / "source", tmp_path / "output"
    source.mkdir()
    (source / "README.md").write_text("Synthetic notice; no model.\n")
    (output / "model").mkdir(parents=True)
    write_json(output / "model/sha256.json", {"synthetic.onnx": "a" * 64})
    inventory = {
        "source_repository": "local/synthetic",
        "source_revision": "b" * 40,
        "source_files": [{"file": "README.md", "sha256": checksum(source / "README.md")}],
    }
    receipt = tmp_path / "source-inventory.json"
    write_json(receipt, inventory)
    return output, source, receipt, inventory


def test_provenance_records_all_six_actual_selected_runtime_versions(tmp_path):
    expected = locked_versions()
    arguments = provenance_inputs(tmp_path)
    provenance.write_provenance(*arguments)
    saved = json.loads((arguments[0] / "provenance.json").read_text())
    assert set(saved["producer_versions"]) == set(RUNTIME)
    assert saved["producer_versions"] == expected
    assert (arguments[0] / "source-inventory.json").read_bytes() == arguments[2].read_bytes()
    assert (arguments[0] / "source-notices/README.md").read_bytes() == (arguments[1] / "README.md").read_bytes()


def test_provenance_rejects_runtime_drift_before_copying_or_writing(monkeypatch, tmp_path):
    arguments = provenance_inputs(tmp_path)
    before = hashes(tmp_path)
    drift(monkeypatch, "numpy")
    monkeypatch.setattr(provenance.shutil, "copyfile", forbidden)
    monkeypatch.setattr(provenance, "write_json", forbidden)
    with pytest.raises(ValueError, match=r"numpy.*uv\.lock"):
        provenance.write_provenance(*arguments)
    assert hashes(tmp_path) == before
