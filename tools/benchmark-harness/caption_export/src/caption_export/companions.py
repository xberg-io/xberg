"""Make native-compatible metadata explicit without changing source assets."""

import shutil
from pathlib import Path

from tokenizers import Tokenizer

from .assets import read_json, write_json
from .metadata import Geometry, validate_metadata


def processor_metadata(source: Path) -> dict:
    """Expand the pinned Transformers ViT processor defaults, preserving explicit settings."""
    processor = read_json(source / "preprocessor_config.json")
    kind = processor.get("image_processor_type", processor.get("feature_extractor_type"))
    if kind not in ("ViTImageProcessor", "ViTFeatureExtractor"):
        raise ValueError("source processor must declare ViTImageProcessor or ViTFeatureExtractor")
    size = processor.get("size")
    if type(size) is int:
        processor["size"] = {"height": size, "width": size}
    processor.setdefault("do_rescale", True)
    processor.setdefault("rescale_factor", 1 / 255)
    return processor


def token_metadata(source: Path, output: Path, config: dict, geometry: Geometry) -> None:
    """Resolve special-token names from existing IDs; never add vocabulary entries."""
    tokenizer = Tokenizer.from_file(str(source / "tokenizer.json"))
    settings = read_json(source / "tokenizer_config.json")
    specials = read_json(source / "special_tokens_map.json")
    for name in ("bos_token", "eos_token", "pad_token"):
        identifier = config.get(f"{name}_id")
        if type(identifier) is not int or not 0 <= identifier < geometry.vocabulary:
            raise ValueError(f"source config requires valid {name}_id")
        token = tokenizer.id_to_token(identifier)
        if token is None:
            raise ValueError(f"source tokenizer has no token for {name}_id")
        for metadata in (settings, specials):
            existing = metadata.get(name)
            if existing is not None and existing != token:
                raise ValueError(f"source tokenizer metadata disagrees on {name}")
            metadata[name] = token
    settings.setdefault("model_max_length", geometry.positions)
    shutil.copyfile(source / "tokenizer.json", output / "tokenizer.json")
    write_json(output / "tokenizer_config.json", settings)
    write_json(output / "special_tokens_map.json", specials)


def prepare_companions(source: Path, output: Path) -> None:
    """Describe freshly exported full-prefix graphs, not a relabeled old artifact."""
    config = read_json(source / "config.json")
    geometry = Geometry.from_config(config)
    config["decoder"]["use_cache"] = False
    generation = {
        name: config.get(name) for name in ("decoder_start_token_id", "bos_token_id", "eos_token_id", "pad_token_id")
    }
    generation.update(use_cache=False, do_sample=False, num_beams=1)
    write_json(output / "config.json", config)
    write_json(output / "generation_config.json", generation)
    write_json(output / "preprocessor_config.json", processor_metadata(source))
    token_metadata(source, output, config, geometry)
    validate_metadata(output)
