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
        return self.decoder(
            input_ids=input_ids,
            encoder_hidden_states=encoder_hidden_states,
            use_cache=False,
            return_dict=True,
        ).logits
