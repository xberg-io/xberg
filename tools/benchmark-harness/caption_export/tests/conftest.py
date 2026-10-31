"""Real seeded architecture smoke fixtures; these weights establish no caption quality."""

import hashlib
import importlib
import json
from pathlib import Path

import pytest

IMAGE_SIZE = 16
HIDDEN_SIZE = 8
PATCH_SIZE = 8
CONTEXT_LENGTH = (IMAGE_SIZE // PATCH_SIZE) ** 2 + 1
VOCABULARY = 17
PREFIX_LENGTHS = (1, 2, 8, 16)
ATOL = 1e-5
RTOL = 1e-4
COMPANIONS = (
    "config.json",
    "generation_config.json",
    "preprocessor_config.json",
    "tokenizer.json",
    "tokenizer_config.json",
    "special_tokens_map.json",
)


def write_json(path: Path, value: dict) -> None:
    path.write_text(json.dumps(value, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def hashes(directory: Path) -> dict[str, str]:
    return {
        str(path.relative_to(directory)): hashlib.sha256(path.read_bytes()).hexdigest()
        for path in sorted(directory.rglob("*"))
        if path.is_file()
    }


def refresh_manifest(directory: Path) -> None:
    values = hashes(directory)
    values.pop("sha256.json", None)
    write_json(directory / "sha256.json", values)


def native_logits(model, pixels, length: int):
    import torch

    ids = (torch.arange(length, dtype=torch.int64).reshape(1, length) % (VOCABULARY - 3)) + 3
    ids[0, 0] = 1
    with torch.inference_mode():
        context = model.encoder(pixel_values=pixels, return_dict=True).last_hidden_state
        logits = model.decoder(input_ids=ids, encoder_hidden_states=context, use_cache=False, return_dict=True).logits
    assert tuple(context.shape) == (1, CONTEXT_LENGTH, HIDDEN_SIZE)
    assert tuple(logits.shape) == (1, length, VOCABULARY)
    assert torch.isfinite(context).all() and torch.isfinite(logits).all()
    return ids, context, logits


@pytest.fixture
def tiny_model(tmp_path):
    import torch
    from transformers import GPT2Config, GPT2LMHeadModel, ViTConfig, ViTModel, VisionEncoderDecoderModel

    torch.manual_seed(2336)
    torch.set_num_threads(1)
    encoder = ViTConfig(
        image_size=IMAGE_SIZE,
        patch_size=PATCH_SIZE,
        num_channels=3,
        hidden_size=HIDDEN_SIZE,
        num_hidden_layers=1,
        num_attention_heads=2,
        intermediate_size=16,
    )
    decoder = GPT2Config(
        vocab_size=VOCABULARY,
        n_positions=32,
        n_embd=HIDDEN_SIZE,
        n_layer=1,
        n_head=2,
        add_cross_attention=True,
        is_decoder=True,
        use_cache=True,
        bos_token_id=1,
        eos_token_id=2,
        pad_token_id=0,
    )
    encoder._attn_implementation = "eager"
    decoder._attn_implementation = "eager"
    model = VisionEncoderDecoderModel(encoder=ViTModel(encoder), decoder=GPT2LMHeadModel(decoder)).eval().cpu()
    model.config.decoder_start_token_id = 1
    model.config.bos_token_id = 1
    model.config.eos_token_id = 2
    model.config.pad_token_id = 0
    source = tmp_path / "source"
    model.save_pretrained(source, safe_serialization=True)
    before = hashes(source)
    pixels = torch.linspace(-1, 1, 3 * IMAGE_SIZE * IMAGE_SIZE).reshape(1, 3, IMAGE_SIZE, IMAGE_SIZE)
    # ~keep Validate the real model at every prefix before importing the not-yet-implemented export API.
    for length in PREFIX_LENGTHS:
        native_logits(model, pixels, length)
    yield model, pixels, source, before
    assert hashes(source) == before, "export must not overwrite the source checkpoint or source configurations"


def write_companions(model, directory: Path) -> None:
    from tokenizers import Tokenizer
    from tokenizers.models import WordLevel

    metadata = model.config.to_dict()
    metadata["architectures"] = ["VisionEncoderDecoderModel"]
    metadata["encoder"]["architectures"] = ["ViTModel"]
    metadata["decoder"]["architectures"] = ["GPT2LMHeadModel"]
    metadata["decoder"]["use_cache"] = False
    write_json(directory / "config.json", metadata)
    special = {"pad_token": "token0", "bos_token": "token1", "eos_token": "token2"}
    write_json(directory / "special_tokens_map.json", special)
    write_json(directory / "tokenizer_config.json", {**special, "model_max_length": 32})
    tokenizer = Tokenizer(WordLevel({f"token{i}": i for i in range(VOCABULARY)}, unk_token="token3"))
    tokenizer.save(str(directory / "tokenizer.json"))
    write_json(
        directory / "generation_config.json",
        {
            "do_sample": False,
            "num_beams": 1,
            "use_cache": False,
            "decoder_start_token_id": 1,
            "bos_token_id": 1,
            "eos_token_id": 2,
            "pad_token_id": 0,
        },
    )
    write_json(
        directory / "preprocessor_config.json",
        {
            "do_resize": True,
            "do_rescale": True,
            "do_normalize": True,
            "resample": 2,
            "size": {"width": IMAGE_SIZE, "height": IMAGE_SIZE},
            "rescale_factor": 1 / 255,
            "image_mean": [0.5] * 3,
            "image_std": [0.5] * 3,
        },
    )


@pytest.fixture
def export_case(tiny_model, tmp_path):
    model, pixels, source, before = tiny_model
    output = tmp_path / "export"
    output.mkdir()
    write_companions(model, output)

    def export():
        api = importlib.import_module("caption_export.exporter")
        api.export_graphs(model, output)
        assert hashes(source) == before
        return api, output, model, pixels

    return export
