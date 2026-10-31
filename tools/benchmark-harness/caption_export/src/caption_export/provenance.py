"""Keep source rights and the export recipe beside the strict model asset set."""

import importlib.metadata
import shutil
from pathlib import Path

from .assets import digest, read_json, write_json


def write_provenance(output: Path, source: Path, receipt: Path, inventory: dict) -> None:
    """Record observed inputs and fresh outputs without claiming publication rights or quality."""
    project = Path(__file__).resolve().parents[2]
    code = {
        str(path.relative_to(project)): digest(path) for path in sorted((project / "src/caption_export").glob("*.py"))
    }
    recipe = {name: digest(project / name) for name in ("pyproject.toml", "uv.lock")}
    versions = {
        name: importlib.metadata.version(name)
        for name in ("torch", "transformers", "tokenizers", "onnx", "onnxruntime", "numpy")
    }
    shutil.copyfile(receipt, output / "source-inventory.json")
    notices = output / "source-notices"
    notices.mkdir()
    for record in inventory["source_files"]:
        name = record["file"]
        if name.upper().startswith(("README", "LICENSE", "NOTICE", "COPYING")):
            shutil.copyfile(source / name, notices / name)
    write_json(
        output / "provenance.json",
        {
            "source_repository": inventory["source_repository"],
            "source_revision": inventory["source_revision"],
            "source_inventory_sha256": digest(receipt),
            "source_files": inventory["source_files"],
            "artifact_sha256": read_json(output / "model/sha256.json"),
            "producer_source_sha256": code,
            "producer_recipe_sha256": recipe,
            "producer_versions": versions,
            "export_contract": {
                "device": "cpu",
                "dtype": "float32",
                "attention": "eager",
                "opset": 17,
                "dynamo": False,
                "use_cache": False,
                "dynamic_axes": ["token_length"],
                "decoding": "greedy full-prefix",
                "batch_size": 1,
            },
            "metadata_normalization": [
                "decoder cache disabled for new graphs",
                "explicit greedy generation",
                "special-token names from existing config IDs; vocabulary unchanged",
                "scalar ViT size expanded; omitted ViT rescale defaults made explicit",
            ],
            "qualification": "Local export only; no SDK, real-model parity, caption quality or publication approval.",
        },
    )
