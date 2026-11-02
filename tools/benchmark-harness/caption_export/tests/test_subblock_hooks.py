"""Observe the actual seeded cross-attention GPT2 fixture, never a fake forward."""

import importlib

import numpy as np
import pytest

from conftest import HIDDEN_SIZE, VOCABULARY, hashes, native_logits
from subblock_fixtures import MULTIPLIERS, STAGES, feature


def module_state(model):
    return {
        name: (module.forward.__func__, tuple(module._forward_hooks), tuple(module._forward_pre_hooks))
        for name, module in model.named_modules()
    }


def test_actual_first_block_hooks_fire_once_without_changing_forward_or_logits(monkeypatch, tiny_model):
    import torch

    model, pixels, source, source_hashes = tiny_model
    ids, context, native = native_logits(model, pixels, 2)
    assert model.config.decoder.add_cross_attention and not model.training
    assert len(model.decoder.transformer.h) == 1
    assert context.numel() > 0 and native.shape == (1, 2, VOCABULARY)
    state = module_state(model)
    api = feature(monkeypatch, "subblock_hooks", ("capture_native",))
    wrapper = importlib.import_module("caption_export.wrappers").Decoder(model)
    with torch.inference_mode():
        unhooked = wrapper(ids, context).numpy().copy()
        result = api.capture_native(model, ids.numpy(), context.numpy())
    assert result["calls"] == dict.fromkeys(STAGES, 1)
    assert tuple(result["captures"]) == STAGES
    for stage, width in zip(STAGES, MULTIPLIERS, strict=True):
        value = result["captures"][stage]
        assert isinstance(value, np.ndarray) and value.dtype == np.float32
        assert value.shape == (1, 2, HIDDEN_SIZE * width) and np.isfinite(value).all()
    assert sum(value.size for value in result["captures"].values()) == 384
    assert result["logits"].shape == (1, 2, VOCABULARY)
    assert result["logits"].tobytes() == unhooked.tobytes() == native.numpy().tobytes()
    assert module_state(model) == state, "remove all hooks and retain every original forward implementation"
    assert hashes(source) == source_hashes


@pytest.mark.parametrize("broken", ["missing_cross_norm", "training"])
def test_hook_policy_rejects_missing_real_module_or_training_before_forward(monkeypatch, tiny_model, broken):
    model, pixels, source, source_hashes = tiny_model
    ids, context, _ = native_logits(model, pixels, 1)
    assert ids.numel() == 1 and context.numel() > 0
    api = feature(monkeypatch, "subblock_hooks", ("capture_native",))
    if broken == "missing_cross_norm":
        monkeypatch.setattr(model.decoder.transformer.h[0], "ln_cross_attn", None)
        pattern = "(?i)(missing|unsupported).*ln_cross_attn"
    else:
        model.train()
        pattern = "(?i)eval"
    state = module_state(model)
    with pytest.raises(ValueError, match=pattern):
        api.capture_native(model, ids.numpy(), context.numpy())
    assert module_state(model) == state
    assert hashes(source) == source_hashes
