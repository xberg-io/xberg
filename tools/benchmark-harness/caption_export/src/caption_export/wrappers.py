"""Explicit CPU FP32 eager encoder and full-prefix decoder export boundaries."""

import torch


class Encoder(torch.nn.Module):
    """Expose exactly one image input and the native encoder context."""

    def __init__(self, model: torch.nn.Module) -> None:
        super().__init__()
        self.encoder = model.encoder

    def forward(self, pixel_values: torch.Tensor) -> torch.Tensor:
        """Return unprojected native ViT hidden states."""
        return self.encoder(pixel_values=pixel_values, return_dict=True).last_hidden_state


class Decoder(torch.nn.Module):
    """Decode a complete prefix without past/present key-value cache ports."""

    def __init__(self, model: torch.nn.Module) -> None:
        super().__init__()
        self.decoder = model.decoder

    def forward(self, input_ids: torch.Tensor, encoder_hidden_states: torch.Tensor) -> torch.Tensor:
        """Return full-prefix logits with caching explicitly disabled."""
        # A prepared 4D mask bypasses Transformers' packed-position torch.diff, unsupported by legacy ONNX. ~keep
        positions = torch.arange(input_ids.shape[1], device=input_ids.device)
        future = positions.unsqueeze(0) > positions.unsqueeze(1)
        mask = torch.zeros_like(future, dtype=encoder_hidden_states.dtype)
        mask = mask.masked_fill(future, torch.finfo(encoder_hidden_states.dtype).min)
        return self.decoder(
            input_ids=input_ids,
            encoder_hidden_states=encoder_hidden_states,
            attention_mask=mask.unsqueeze(0).unsqueeze(0),
            use_cache=False,
            return_dict=True,
        ).logits
