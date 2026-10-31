"""Offline entry points for an explicitly selected hash-bound local source."""

import argparse
import importlib
import os
import sys
from pathlib import Path

from .assets import digest, read_json
from .companions import prepare_companions
from .provenance import write_provenance
from .source import inventory_files, verify_source


def offline_runtime() -> type:
    """Initialize the existing environment only after source verification."""
    for name in (
        "HF_HUB_OFFLINE",
        "TRANSFORMERS_OFFLINE",
        "HF_HUB_DISABLE_TELEMETRY",
        "OMP_NUM_THREADS",
        "MKL_NUM_THREADS",
    ):
        os.environ[name] = "1"
    os.environ["TOKENIZERS_PARALLELISM"] = "false"
    torch = importlib.import_module("torch")
    torch.set_num_threads(1)
    torch.set_num_interop_threads(1)
    transformers = importlib.import_module("transformers")
    return transformers.VisionEncoderDecoderModel


def export_local(arguments: argparse.Namespace) -> None:
    """Create a new bundle, preserving all source files and existing destinations."""
    source, output = arguments.source, arguments.output
    if output.exists() or output.is_symlink():
        raise ValueError("output already exists; choose a new bundle directory")
    if output.resolve().is_relative_to(source.resolve()):
        raise ValueError("output must be outside the immutable source directory")
    inventory = verify_source(source, arguments.inventory, arguments.repository, arguments.revision)
    output.mkdir(parents=True, exist_ok=False)
    model_directory = output / "model"
    model_directory.mkdir()
    prepare_companions(source, model_directory)
    model_class = offline_runtime()
    model = model_class.from_pretrained(
        str(source),
        local_files_only=True,
        trust_remote_code=False,
        weights_only=True,
        attn_implementation="eager",
        dtype="float32",
    )
    exporter = importlib.import_module("caption_export.exporter")
    exporter.export_graphs(model, model_directory)
    exporter.validate_export(model_directory)
    verify_source(source, arguments.inventory, arguments.repository, arguments.revision)
    write_provenance(output, source, arguments.inventory, inventory)
    validate_bundle(output)


def validate_bundle(output: Path) -> None:
    """Validate graph bytes and metadata, plus the recorded artifact/source receipts."""
    provenance = read_json(output / "provenance.json")
    from_receipt = read_json(output / "source-inventory.json")
    if digest(output / "source-inventory.json") != provenance.get("source_inventory_sha256") or from_receipt.get(
        "source_files"
    ) != provenance.get("source_files"):
        raise ValueError("bundle source inventory checksum differs from provenance")
    if (from_receipt.get("source_repository"), from_receipt.get("source_revision")) != (
        provenance.get("source_repository"),
        provenance.get("source_revision"),
    ):
        raise ValueError("bundle source identity differs from provenance")
    notices = {
        name: record
        for name, record in inventory_files(from_receipt).items()
        if name.upper().startswith(("README", "LICENSE", "NOTICE", "COPYING"))
    }
    if {p.name for p in (output / "source-notices").iterdir()} != notices.keys():
        raise ValueError("bundle source notices differ from inventory")
    for name, record in notices.items():
        path = output / "source-notices" / name
        if path.is_symlink() or not path.is_file() or digest(path) != record["sha256"]:
            raise ValueError(f"bundle source notice checksum mismatch: {name}")
    exporter = importlib.import_module("caption_export.exporter")
    exporter.validate_export(output / "model")
    if provenance.get("artifact_sha256") != read_json(output / "model/sha256.json"):
        raise ValueError("bundle artifact manifest differs from provenance")


def parser() -> argparse.ArgumentParser:
    """Require explicit source identity rather than selecting a remote default."""
    result = argparse.ArgumentParser(description=__doc__)
    commands = result.add_subparsers(dest="command", required=True)
    export = commands.add_parser("export-local", help="export a verified local ViT/GPT-2 source into a new bundle")
    for name in ("source", "inventory", "output"):
        export.add_argument(f"--{name}", required=True, type=Path)
    for name in ("repository", "revision"):
        export.add_argument(f"--{name}", required=True)
    validate = commands.add_parser("validate", help="validate a completed local export bundle")
    validate.add_argument("output", type=Path)
    return result


def main() -> int:
    """Report actionable local failures without swallowing export errors."""
    arguments = parser().parse_args()
    try:
        if arguments.command == "export-local":
            export_local(arguments)
        else:
            validate_bundle(arguments.output)
    except (OSError, ValueError, RuntimeError) as error:
        sys.stderr.write(f"caption-export: {error}\n")
        return 1
    sys.stdout.write(f"Validated local caption export: {arguments.output}\n")
    return 0
