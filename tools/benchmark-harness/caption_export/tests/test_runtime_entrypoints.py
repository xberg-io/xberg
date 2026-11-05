"""Exercise runtime admission and real CLI/diagnostic entrypoint guards."""

import importlib
import importlib.metadata
import json
import sys
from pathlib import Path

import pytest
import test_local_cli
import test_subblock_plan
from conftest import write_json
from test_local_cli import export_arguments
from test_subblock_plan import checksum

local_source = test_local_cli.local_source
evidence = test_subblock_plan.evidence

RUNTIME = ("numpy", "onnx", "onnxruntime", "tokenizers", "torch", "transformers")


def cli_arguments(cli, source, inventory, output):
    return cli.parser().parse_args([str(argument) for argument in export_arguments(source, inventory, output)])


def mismatch(monkeypatch, name, value):
    version = importlib.metadata.version
    monkeypatch.setattr(importlib.metadata, "version", lambda package: value if package == name else version(package))


@pytest.mark.parametrize("name", RUNTIME)
def test_export_rejects_unlocked_runtime_before_loading_any_model(monkeypatch, local_source, tmp_path, name):
    from caption_export import cli

    source, inventory, _ = local_source
    arguments = cli_arguments(cli, source, inventory, tmp_path / "bundle")
    cli.verify_source(source, inventory, arguments.repository, arguments.revision)
    mismatch(monkeypatch, name, "2.5.3" if name == "numpy" else "0.0.0")

    def forbidden():
        raise AssertionError("unlocked runtime reached the model producer")

    monkeypatch.setattr(cli, "offline_runtime", forbidden)
    with pytest.raises(ValueError, match=rf"{name}.*uv\.lock"):
        cli.export_local(arguments)
    assert not arguments.output.exists()


@pytest.mark.parametrize(("name", "value"), [("numpy", "2.5.3"), ("transformers", "5.20.1")])
def test_subblock_plan_rejects_unlocked_runtime_before_pinning_conv1d(monkeypatch, evidence, name, value):
    from caption_export import subblock_plan

    prospective, original, _ = evidence
    control = subblock_plan.freeze_plan(prospective, checksum(prospective), original)
    assert Path(control["dependency_sources"]["conv1d"]["path"]).is_file()
    mismatch(monkeypatch, name, value)
    with pytest.raises(ValueError, match=rf"{name}.*uv\.lock"):
        subblock_plan.freeze_plan(prospective, checksum(prospective), original)


def runner_plan(module, evidence, path):
    from caption_export.first_divergence_plan import freeze_plan
    from caption_export.subblock_plan import freeze_plan as freeze_subblock

    prospective, original, _ = evidence
    plan = (
        freeze_plan(original)
        if module == "first_divergence_run"
        else freeze_subblock(prospective, checksum(prospective), original)
    )
    write_json(path, plan)
    return original


@pytest.mark.parametrize("module", ["first_divergence_run", "subblock_run"])
@pytest.mark.parametrize("mode", ["complete", "execution_error", "changed_original"])
def test_runner_main_preserves_failed_qualification_and_reraises(monkeypatch, evidence, tmp_path, module, mode):
    path, output = tmp_path / "execution-plan.json", tmp_path / "run-output"
    original = runner_plan(module, evidence, path)
    runner = importlib.import_module(f"caption_export.{module}")
    calls = []

    def execute(*_args, **_kwargs):
        calls.append("execute")
        if mode == "execution_error":
            raise RuntimeError("measured diagnostic failure")
        if mode == "changed_original":
            with (original / "parity.py").open("a") as stream:
                stream.write("\n# changed after execution\n")
        return {"diagnostic_complete": True}

    monkeypatch.setattr(runner, "execute", execute)
    monkeypatch.setattr(
        sys, "argv", [module, "--plan", str(path), "--plan-sha256", checksum(path), "--output", str(output)]
    )
    if mode == "complete":
        assert runner.main() == 0
    else:
        error = RuntimeError if mode == "execution_error" else ValueError
        with pytest.raises(error):
            runner.main()
    assert calls == ["execute"]
    report = json.loads((output / "result.json").read_text())
    assert report["qualified"] is False
    assert report["diagnostic_complete"] is (mode == "complete")
    assert report["status"] == (
        "DIAGNOSTIC_COMPLETE_WITH_ORIGINAL_QUALIFICATION_FAILED" if mode == "complete" else "INCOMPLETE"
    )
    assert report["original_result"]["status"] == "FAIL"
    assert (output / "plan.json").read_bytes() == path.read_bytes()


def test_export_rechecks_source_after_real_graph_export(monkeypatch, local_source, tiny_model, tmp_path):
    from caption_export import cli, exporter

    source, inventory, _ = local_source
    arguments = cli_arguments(cli, source, inventory, tmp_path / "bundle")
    real_export, calls = exporter.export_graphs, []
    monkeypatch.setattr(cli, "offline_runtime", lambda: type(tiny_model[0]))

    def export(model, output):
        real_export(model, output)
        calls.append("exported")
        with (source / "README.md").open("a") as stream:
            stream.write("changed after actual graph export\n")

    def forbidden(*_args):
        raise AssertionError("post-export source mutation reached provenance")

    monkeypatch.setattr(exporter, "export_graphs", export)
    monkeypatch.setattr(cli, "write_provenance", forbidden)
    with pytest.raises(ValueError, match=r"source sha256/size mismatch: README\.md"):
        cli.export_local(arguments)
    assert calls == ["exported"]


def test_export_validates_final_bundle_after_real_provenance(monkeypatch, local_source, tiny_model, tmp_path):
    from caption_export import cli

    source, inventory, _ = local_source
    arguments = cli_arguments(cli, source, inventory, tmp_path / "bundle")
    real_provenance, calls = cli.write_provenance, []
    monkeypatch.setattr(cli, "offline_runtime", lambda: type(tiny_model[0]))

    def provenance(output, *args):
        real_provenance(output, *args)
        calls.append("provenance")
        receipt = json.loads((output / "provenance.json").read_text())
        receipt["artifact_sha256"]["decoder.onnx"] = "0" * 64
        write_json(output / "provenance.json", receipt)

    monkeypatch.setattr(cli, "write_provenance", provenance)
    with pytest.raises(ValueError, match="bundle artifact manifest differs from provenance"):
        cli.export_local(arguments)
    assert calls == ["provenance"]
