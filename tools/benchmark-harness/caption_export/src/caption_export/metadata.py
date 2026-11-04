"""Geometry and no-cache metadata for the ViT/GPT-2 five-interface contract."""

import math
from dataclasses import dataclass
from pathlib import Path

from tokenizers import Tokenizer

from .assets import COMPANIONS, read_json, require_files

MAX_NATIVE_DIMENSION = (1 << 31) - 1


def positive_integer(value: object, field: str, *, maximum: int | None = MAX_NATIVE_DIMENSION) -> int:
    """Keep booleans and absent geometry out of tensor dimensions."""
    if type(value) is not int or value <= 0 or (maximum is not None and value > maximum):
        raise ValueError(f"{field} must be a positive integer")
    return value


@dataclass(frozen=True)
class Geometry:
    """Static image and encoder context dimensions; only token length varies."""

    height: int
    width: int
    context: int
    hidden: int
    vocabulary: int
    positions: int

    @classmethod
    def from_config(cls, config: dict) -> "Geometry":
        """Derive the SDK's square RGB ViT context from native configuration."""
        encoder, decoder = config.get("encoder", {}), config.get("decoder", {})
        if not isinstance(encoder, dict) or not isinstance(decoder, dict):
            raise ValueError("config.json requires encoder and decoder objects")
        if config.get("model_type") != "vision-encoder-decoder":
            raise ValueError("config.json requires a vision-encoder-decoder model")
        if encoder.get("model_type") != "vit" or decoder.get("model_type") != "gpt2":
            raise ValueError("config.json requires the ViT/GPT-2 architecture contract")
        image_size = positive_integer(encoder.get("image_size"), "encoder.image_size")
        patch_size = positive_integer(encoder.get("patch_size"), "encoder.patch_size")
        hidden = positive_integer(encoder.get("hidden_size"), "encoder.hidden_size")
        decoder_hidden = positive_integer(decoder.get("n_embd"), "decoder.n_embd")
        if hidden != decoder_hidden or decoder.get("cross_attention_hidden_size") not in (None, hidden):
            raise ValueError("enc_to_dec_proj requires a different encoder context contract")
        if encoder.get("num_channels") != 3 or image_size % patch_size:
            raise ValueError("encoder shape contract requires RGB images divisible by patch size")
        return cls(
            image_size,
            image_size,
            (image_size // patch_size) ** 2 + 1,
            hidden,
            positive_integer(decoder.get("vocab_size"), "decoder.vocab_size"),
            positive_integer(decoder.get("n_positions"), "decoder.n_positions"),
        )


def validate_metadata(directory: Path) -> Geometry:
    """Require complete local metadata and full-prefix greedy decoding."""
    require_files(directory, COMPANIONS)
    values = {name: read_json(directory / name) for name in COMPANIONS}
    config, generation = values["config.json"], values["generation_config.json"]
    geometry = Geometry.from_config(config)
    validate_architecture(config)
    if config["decoder"].get("use_cache") is not False or generation.get("use_cache") is not False:
        raise ValueError("config.json and generation_config.json must disable cache")
    if generation.get("do_sample") is not False or generation.get("num_beams") != 1:
        raise ValueError("generation_config.json must specify greedy decoding")
    validate_processor(values["preprocessor_config.json"], geometry)
    for name in ("decoder_start_token_id", "bos_token_id", "eos_token_id", "pad_token_id"):
        token = generation.get(name)
        if type(token) is not int or not 0 <= token < geometry.vocabulary or config.get(name) != token:
            raise ValueError(f"generation_config.json token contract disagrees on {name}")
    validate_tokenizer(directory, values, config, geometry)
    return geometry


def validate_architecture(config: dict) -> None:
    """Match the architecture policy enforced by native preparation."""
    for value, expected in [
        (config, "VisionEncoderDecoderModel"),
        (config["encoder"], "ViTModel"),
        (config["decoder"], "GPT2LMHeadModel"),
    ]:
        if value.get("architectures") != [expected]:
            raise ValueError(f"config.json architecture contract requires {expected}")
    if config["decoder"].get("add_cross_attention") is not True or config["decoder"].get("is_decoder") is not True:
        raise ValueError("config.json requires decoder cross-attention")


def validate_processor(processor: dict, geometry: Geometry) -> None:
    """Require the SDK's explicit bilinear RGB preprocessing contract."""
    if processor.get("size") != {"height": geometry.height, "width": geometry.width}:
        raise ValueError("preprocessor_config.json image shape contract disagrees with encoder")
    if any(processor.get(name) is not True for name in ("do_resize", "do_rescale", "do_normalize")):
        raise ValueError("preprocessor_config.json must resize, rescale and normalize")
    if processor.get("resample") != 2:
        raise ValueError("preprocessor_config.json requires bilinear resampling")
    factor = processor.get("rescale_factor")
    if type(factor) not in (int, float) or not math.isfinite(factor) or factor <= 0:
        raise ValueError("preprocessor_config.json requires a finite positive rescale factor")
    for name in ("image_mean", "image_std"):
        vector = processor.get(name)
        if not isinstance(vector, list) or len(vector) != 3:
            raise ValueError(f"preprocessor_config.json requires three {name} values")
        if any(type(item) not in (int, float) or not math.isfinite(item) for item in vector):
            raise ValueError(f"preprocessor_config.json requires finite {name}")
        if name == "image_std" and any(item <= 0 for item in vector):
            raise ValueError("preprocessor_config.json requires positive image_std")


def validate_tokenizer(directory: Path, values: dict, config: dict, geometry: Geometry) -> None:
    """Bind tokenizer vocabulary and special tokens to the native decoder."""
    try:
        tokenizer = Tokenizer.from_file(str(directory / "tokenizer.json"))
    except Exception as error:
        raise ValueError(f"invalid tokenizer.json: {error}") from error
    if tokenizer.get_vocab_size(with_added_tokens=True) != geometry.vocabulary:
        raise ValueError("tokenizer.json vocabulary differs from decoder config")
    tokenizer_config = values["tokenizer_config.json"]
    positive_integer(tokenizer_config.get("model_max_length"), "tokenizer_config.json model_max_length", maximum=None)
    for name in ("bos_token", "eos_token", "pad_token"):
        token = tokenizer_config.get(name)
        if not isinstance(token, str) or values["special_tokens_map.json"].get(name) != token:
            raise ValueError(f"special_tokens_map.json differs from tokenizer_config.json on {name}")
        if tokenizer.token_to_id(token) != config[f"{name}_id"]:
            raise ValueError(f"tokenizer.json special token contract differs on {name}")
