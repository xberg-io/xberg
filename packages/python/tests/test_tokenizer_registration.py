"""Exercise tokenizer registration through the public Python binding."""

import unittest

import xberg


class _FailingTokenizer:
    def __init__(self) -> None:
        self.calls: list[str] = []

    def name(self) -> str:
        return "tokenizer-registration-error-test"

    def version(self) -> str:
        return "1.0.0"

    def initialize(self) -> None:
        self.calls.append("initialize")

    def count_tokens(self, text: str) -> int:
        assert text == "a"
        self.calls.append("count_tokens")
        raise RuntimeError("tokenizer callback probe failed")

    def shutdown(self) -> None:
        self.calls.append("shutdown")


class TokenizerRegistrationTests(unittest.TestCase):
    """Check the registration probe against the native callback bridge."""

    def test_registration_retains_callback_error_and_shuts_down(self) -> None:
        """Keep the backend cause and release failed registration state."""
        backend = _FailingTokenizer()
        name = backend.name()
        assert name not in xberg.list_tokenizer_backends()
        caught = None
        try:
            try:
                xberg.register_tokenizer_backend(backend)
            except RuntimeError as error:
                caught = error
            assert caught is not None
            assert "tokenizer callback probe failed" in str(caught)
            assert name not in xberg.list_tokenizer_backends()
            assert backend.calls == ["initialize", "count_tokens", "shutdown"]
        finally:
            if name in xberg.list_tokenizer_backends():
                xberg.unregister_tokenizer_backend(name)
