from __future__ import annotations

from kreuzberg import CancellationToken, ExtractionConfig


def test_extraction_config_accepts_cancel_token_kwarg() -> None:
    token = CancellationToken()
    config = ExtractionConfig(cancel_token=token)
    assert config.cancel_token is not None
    assert not config.cancel_token.is_cancelled()


def test_extraction_config_cancel_token_defaults_to_none() -> None:
    config = ExtractionConfig()
    assert config.cancel_token is None


def test_extraction_config_cancel_token_round_trip_via_setter() -> None:
    token = CancellationToken()
    config = ExtractionConfig()
    config.cancel_token = token
    token.cancel()
    assert config.cancel_token is not None
    assert config.cancel_token.is_cancelled()
