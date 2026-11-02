"""Export and validate a local, no-cache ViT/GPT-2 caption artifact bundle."""

from pathlib import Path

import torch

from .assets import ASSETS, COMPANIONS, read_json, require_files, verify_integrity, write_json, write_manifest
from .graphs import GraphInterface, fix_output_dimensions, interfaces, validate_graph
from .metadata import Geometry, validate_metadata
from .wrappers import Decoder, Encoder


def validate_export(output: str | Path) -> None:
    """Check complete asset integrity before loading either graph."""
    directory = Path(output)
    verify_integrity(directory)
    geometry = validate_metadata(directory)
    for name, expected in interfaces(geometry).items():
        validate_graph(directory / name, expected)
    if set(read_json(directory / "sha256.json")) != set(ASSETS):
        raise ValueError("export contract supports exactly eight inline assets")


def prepare_model(model: torch.nn.Module, geometry: Geometry) -> None:
    """Disable cache and fused attention on the actual exported submodels."""
    if Geometry.from_config(model.config.to_dict()) != geometry:
        raise ValueError("model/config.json geometry contract mismatch")
    if getattr(model, "enc_to_dec_proj", None) is not None:
        raise ValueError("enc_to_dec_proj is unsupported by the native context contract")
    model.cpu().float().eval()
    model.config.decoder.use_cache = False
    model.decoder.config.use_cache = False
    model.generation_config.use_cache = False
    model.generation_config.do_sample = False
    model.generation_config.num_beams = 1
    model.encoder.set_attn_implementation("eager")
    model.decoder.set_attn_implementation("eager")


def prepare_metadata(directory: Path) -> Geometry:
    """Normalize no-cache metadata in the destination, never the source checkpoint."""
    require_files(directory, COMPANIONS)
    config = read_json(directory / "config.json")
    generation = read_json(directory / "generation_config.json")
    geometry = Geometry.from_config(config)
    config["decoder"]["use_cache"] = False
    generation.update(use_cache=False, do_sample=False, num_beams=1)
    write_json(directory / "config.json", config)
    write_json(directory / "generation_config.json", generation)
    validate_metadata(directory)
    return geometry


def export_graph(wrapper: torch.nn.Module, arguments: tuple, path: Path, expected: GraphInterface) -> None:
    """Use the legacy opset-17 exporter with inline weights and token-only dynamics."""
    inputs, outputs = expected
    dynamic = {
        name: {index: axis for index, axis in enumerate(shape) if isinstance(axis, str)}
        for name, _, shape in (*inputs, *outputs)
    }
    torch.onnx.export(
        wrapper,
        arguments,
        str(path),
        input_names=[name for name, _, _ in inputs],
        output_names=[name for name, _, _ in outputs],
        dynamic_axes={name: axes for name, axes in dynamic.items() if axes},
        opset_version=17,
        dynamo=False,
        external_data=False,
        export_params=True,
        keep_initializers_as_inputs=False,
        training=torch.onnx.TrainingMode.EVAL,
    )
    fix_output_dimensions(path, outputs)
    validate_graph(path, expected)


def export_graphs(model: torch.nn.Module, output: str | Path) -> None:
    """Export a supplied local model beside six already-staged companion files."""
    directory = Path(output)
    geometry = prepare_metadata(directory)
    prepare_model(model, geometry)
    encoder, decoder = Encoder(model).eval(), Decoder(model).eval()
    pixels = torch.zeros((1, 3, geometry.height, geometry.width), dtype=torch.float32, device="cpu")
    start = read_json(directory / "generation_config.json")["decoder_start_token_id"]
    # Trace a multi-token prefix so first-token attention shortcuts cannot erase the causal mask. ~keep
    ids = torch.full((1, min(2, geometry.positions)), start, dtype=torch.int64, device="cpu")
    specs = interfaces(geometry)
    with torch.inference_mode():
        context = encoder(pixels)
        logits = decoder(ids, context)
        if tuple(context.shape) != (1, geometry.context, geometry.hidden):
            raise ValueError("eager encoder output shape violates the native context contract")
        if tuple(logits.shape) != (1, ids.shape[1], geometry.vocabulary):
            raise ValueError("eager decoder output shape violates the logits contract")
        export_graph(encoder, (pixels,), directory / "encoder.onnx", specs["encoder.onnx"])
        export_graph(decoder, (ids, context), directory / "decoder.onnx", specs["decoder.onnx"])
    write_manifest(directory)
    validate_export(directory)
