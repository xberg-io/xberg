"""Observe real first-block module inputs/outputs without replacing forwards."""

import importlib
from collections.abc import Callable

import numpy as np

from .first_divergence_evidence import require
from .subblock import MULTIPLIERS, STAGES

POINTS = (
    ("ln_1", "pre"),
    ("ln_1", "post"),
    ("attn.c_attn", "post"),
    ("attn.c_proj", "pre"),
    ("attn.c_proj", "post"),
    ("ln_cross_attn", "pre"),
    ("ln_cross_attn", "post"),
    ("crossattention.q_attn", "post"),
    ("crossattention.c_proj", "pre"),
    ("crossattention.c_proj", "post"),
    ("ln_2", "pre"),
    ("ln_2", "post"),
    ("mlp.c_fc", "post"),
    ("mlp.c_proj", "pre"),
    ("mlp.c_proj", "post"),
    ("", "post"),
)


def _points(model: object) -> list:
    transformer = model.decoder.transformer
    require(len(transformer.h) > 0, "missing first GPT2 block")
    result = []
    for stage, (path, phase) in zip(STAGES, POINTS, strict=True):
        module = transformer.h[0]
        for part in path.split(".") if path else ():
            module = getattr(module, part, None)
        require(
            module is not None and callable(getattr(module, "register_forward_hook", None)),
            f"missing module {path} for {stage}",
        )
        result.append((stage, phase, module))
    require(len({(id(module), phase) for _, phase, module in result}) == 16, "duplicate physical subblock hook seam")
    return result


def _inputs(ids: np.ndarray, context: np.ndarray) -> None:
    require(
        isinstance(ids, np.ndarray)
        and ids.dtype == np.int64
        and ids.ndim == 2
        and ids.shape[0] == 1
        and ids.shape[1] in (1, 2),
        "unsupported subblock prefix",
    )
    require(
        isinstance(context, np.ndarray)
        and context.dtype == np.float32
        and context.ndim == 3
        and context.shape[0] == 1
        and context.size > 0
        and np.isfinite(context).all(),
        "unsupported subblock context",
    )


def capture_native(model: object, input_ids: np.ndarray, context: np.ndarray) -> dict:
    """Run one ordinary wrapper call and record exactly sixteen nonempty seams."""
    torch = importlib.import_module("torch")

    _inputs(input_ids, context)
    require(all(not module.training for module in model.modules()), "all reference modules must be eval")
    points = _points(model)
    hidden = model.decoder.config.n_embd
    widths = dict(zip(STAGES, (hidden * value for value in MULTIPLIERS), strict=True))
    captures, calls, handles = {}, {}, []

    def save(stage: str, value: object) -> None:
        calls[stage] = calls.get(stage, 0) + 1
        require(calls[stage] == 1, f"duplicate native hook call: {stage}")
        require(
            isinstance(value, torch.Tensor) and value.dtype == torch.float32 and value.device.type == "cpu",
            f"non-CPU-FP32 hook: {stage}",
        )
        require(
            tuple(value.shape) == (1, input_ids.shape[1], widths[stage]) and torch.isfinite(value).all().item(),
            f"invalid native hook shape/value: {stage}",
        )
        captures[stage] = value.detach().contiguous().numpy().copy()

    def pre(stage: str) -> Callable:
        return lambda _module, arguments: save(stage, arguments[0])

    def post(stage: str) -> Callable:
        return lambda _module, _arguments, value: save(stage, value)

    try:
        for stage, phase, module in points:
            handles.append(
                module.register_forward_pre_hook(pre(stage))
                if phase == "pre"
                else module.register_forward_hook(post(stage))
            )
        wrapper = importlib.import_module("caption_export.wrappers").Decoder(model)
        with torch.inference_mode():
            output = wrapper(torch.from_numpy(input_ids), torch.from_numpy(context))
        require(calls == dict.fromkeys(STAGES, 1), "missing or repeated subblock hook call")
        logits = output.detach().contiguous().numpy().copy()
    finally:
        for handle in handles:
            handle.remove()
    return {
        "captures": {stage: captures[stage] for stage in STAGES},
        "calls": calls,
        "logits": logits,
        "module_receipt": {
            stage: {
                "class": f"{type(module).__module__}.{type(module).__qualname__}",
                "forward": f"{module.forward.__module__}.{module.forward.__qualname__}",
                "hook": phase,
            }
            for stage, phase, module in points
        },
    }
