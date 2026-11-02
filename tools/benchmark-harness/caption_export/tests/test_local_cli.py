"""Only seeded native checkpoints exercise the local exporter CLI here."""

import hashlib
import json
import os
import shutil
import subprocess
import sys
from pathlib import Path

import pytest
import torch
from conftest import COMPANIONS, hashes, write_companions, write_json

REPOSITORY = "local/seeded-vit-gpt2"
REVISION = "d" * 40
PROJECT = Path(__file__).resolve().parents[1]


@pytest.fixture
def local_source(tiny_model, tmp_path):
    model, _, checkpoint, _ = tiny_model
    source = tmp_path / "local-source"
    shutil.copytree(checkpoint, source)
    write_companions(model, source)
    config = json.loads((source / "config.json").read_text())
    config["decoder"]["use_cache"] = True
    write_json(source / "config.json", config)
    processor = json.loads((source / "preprocessor_config.json").read_text())
    processor.update(size=16, image_processor_type="ViTImageProcessor")
    processor.pop("do_rescale")
    processor.pop("rescale_factor")
    write_json(source / "preprocessor_config.json", processor)
    for name in ("tokenizer_config.json", "special_tokens_map.json"):
        value = json.loads((source / name).read_text())
        value.pop("pad_token")
        write_json(source / name, value)
    (source / "generation_config.json").unlink()
    (source / "README.md").write_text("---\nlicense: apache-2.0\n---\nSeeded fixture; no quality claim.\n")
    inventory = tmp_path / "source-inventory.json"
    write_json(
        inventory,
        {
            "source_repository": REPOSITORY,
            "source_revision": REVISION,
            "source_files": [
                {"file": name, "bytes": (source / name).stat().st_size, "sha256": digest}
                for name, digest in hashes(source).items()
            ],
        },
    )
    return source, inventory, hashes(source)


def invoke(*arguments):
    environment = dict(
        os.environ,
        PYTHONPATH=str(PROJECT / "src"),
        HF_HUB_OFFLINE="1",
        TRANSFORMERS_OFFLINE="1",
        OMP_NUM_THREADS="1",
        MKL_NUM_THREADS="1",
        TOKENIZERS_PARALLELISM="false",
        HF_HUB_DISABLE_TELEMETRY="1",
    )
    return subprocess.run(
        [sys.executable, "-m", "caption_export", *map(str, arguments)],
        env=environment,
        capture_output=True,
        text=True,
        timeout=120,
        check=False,
    )


def export_arguments(source, inventory, output, revision=REVISION):
    return (
        "export-local",
        "--source",
        source,
        "--inventory",
        inventory,
        "--repository",
        REPOSITORY,
        "--revision",
        revision,
        "--output",
        output,
    )


@pytest.fixture
def exported(local_source, tmp_path):
    source, inventory, before = local_source
    output = tmp_path / "bundle"
    result = invoke(*export_arguments(source, inventory, output))
    assert result.returncode == 0, result.stdout + result.stderr
    assert hashes(source) == before
    return output, source, inventory


def test_local_cli_exports_seeded_checkpoint_and_truthful_companions(exported):
    output, source, inventory = exported
    model = output / "model"
    assert {p.name for p in model.iterdir()} == {*COMPANIONS, "encoder.onnx", "decoder.onnx", "sha256.json"}
    assert json.loads((model / "config.json").read_text())["decoder"]["use_cache"] is False
    assert json.loads((source / "config.json").read_text())["decoder"]["use_cache"] is True
    assert json.loads((model / "generation_config.json").read_text())["use_cache"] is False
    processor = json.loads((model / "preprocessor_config.json").read_text())
    assert processor["size"] == {"height": 16, "width": 16}
    assert processor["do_rescale"] is True and processor["rescale_factor"] == 1 / 255
    assert json.loads((model / "tokenizer_config.json").read_text())["pad_token"] == "token0"
    assert (model / "tokenizer.json").read_bytes() == (source / "tokenizer.json").read_bytes()
    provenance = json.loads((output / "provenance.json").read_text())
    assert (provenance["source_repository"], provenance["source_revision"]) == (REPOSITORY, REVISION)
    assert provenance["source_inventory_sha256"] == hashlib.sha256(inventory.read_bytes()).hexdigest()
    assert provenance["artifact_sha256"] == json.loads((model / "sha256.json").read_text())
    assert (output / "source-notices/README.md").read_bytes() == (source / "README.md").read_bytes()
    checked = invoke("validate", output)
    assert checked.returncode == 0, checked.stdout + checked.stderr


@pytest.mark.parametrize("revision", ["main", "e" * 40])
def test_local_cli_rejects_wrong_or_floating_revision_before_loading(local_source, tmp_path, revision):
    source, inventory, _ = local_source
    output = tmp_path / "rejected"
    result = invoke(*export_arguments(source, inventory, output, revision))
    assert result.returncode != 0 and "revision" in result.stderr.lower(), result.stderr
    assert not output.exists()


def test_local_cli_rejects_checkpoint_corruption_before_loading(local_source, tmp_path):
    source, inventory, _ = local_source
    with (source / "model.safetensors").open("ab") as stream:
        stream.write(b"corruption")
    output = tmp_path / "rejected"
    result = invoke(*export_arguments(source, inventory, output))
    assert result.returncode != 0 and "sha256" in result.stderr.lower(), result.stderr
    assert not output.exists()


def test_local_cli_rejects_unmanifested_source_files(local_source, tmp_path):
    source, inventory, _ = local_source
    (source / "foreign.bin").write_bytes(b"not selected")
    result = invoke(*export_arguments(source, inventory, tmp_path / "rejected"))
    assert result.returncode != 0 and "inventory" in result.stderr.lower(), result.stderr


def test_local_cli_preserves_foreign_nonempty_output(local_source, tmp_path):
    source, inventory, _ = local_source
    output = tmp_path / "foreign"
    output.mkdir()
    (output / "keep.txt").write_text("owned elsewhere")
    result = invoke(*export_arguments(source, inventory, output))
    assert result.returncode != 0 and "exist" in result.stderr.lower(), result.stderr
    assert list(output.iterdir()) == [output / "keep.txt"]
    assert (output / "keep.txt").read_text() == "owned elsewhere"


def test_local_cli_validation_rejects_corrupted_export(exported):
    output, _, _ = exported
    with (output / "model/encoder.onnx").open("ab") as stream:
        stream.write(b"corruption")
    result = invoke("validate", output)
    assert result.returncode != 0 and "checksum" in result.stderr.lower(), result.stderr


def test_local_cli_exports_seeded_weights_only_pytorch_checkpoint(local_source, tiny_model, tmp_path):
    source, inventory, _ = local_source
    (source / "model.safetensors").unlink()
    torch.save(tiny_model[0].state_dict(), source / "pytorch_model.bin")
    receipt = json.loads(inventory.read_text())
    receipt["source_files"] = [
        {"file": name, "bytes": (source / name).stat().st_size, "sha256": digest}
        for name, digest in hashes(source).items()
    ]
    write_json(inventory, receipt)
    before = hashes(source)
    output = tmp_path / "pytorch-bundle"
    result = invoke(*export_arguments(source, inventory, output))
    assert result.returncode == 0, result.stdout + result.stderr
    assert hashes(source) == before
    assert invoke("validate", output).returncode == 0


def test_local_cli_validation_rejects_changed_source_receipt(exported):
    output, _, _ = exported
    receipt = output / "source-inventory.json"
    value = json.loads(receipt.read_text())
    value["unverified_extra_claim"] = "changed after export"
    write_json(receipt, value)
    result = invoke("validate", output)
    assert result.returncode != 0 and "source inventory checksum" in result.stderr.lower(), result.stderr
