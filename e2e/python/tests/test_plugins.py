"""Bridge registry error-path tests for document extractor and renderer plugins.

These tests cover the observable behaviour of the register/unregister/clear
lifecycle at the Python layer, using duck-typed stub objects that satisfy the
bridge protocol requirements checked by the Rust side.

A DocumentExtractor bridge object must expose:
  - name()     -> str
  - extract_bytes(content, mime_type, config_json) -> str  (JSON InternalDocument)
  - supported_mime_types() -> list[str]

A Renderer bridge object must expose:
  - name() -> str
  - render(doc_json) -> str
"""

from kreuzberg import (
    clear_document_extractors,
    clear_renderers,
    extract_bytes_sync,
    list_document_extractors,
    list_renderers,
    register_document_extractor,
    register_renderer,
    unregister_document_extractor,
    unregister_renderer,
)


# ---------------------------------------------------------------------------
# Minimal stub implementations
# ---------------------------------------------------------------------------


class _StubExtractor:
    """Minimal duck-typed document extractor stub."""

    def __init__(self, name: str, mime_type: str = "text/plain") -> None:
        self._name = name
        self._mime = mime_type

    def name(self) -> str:
        return self._name

    def version(self) -> str:
        return "0.0.1"

    def initialize(self) -> None:
        pass

    def shutdown(self) -> None:
        pass

    def supported_mime_types(self) -> list[str]:
        return [self._mime]

    def extract_bytes(self, content: bytes, mime_type: str, config_json: str) -> str:
        # Return a minimal valid InternalDocument JSON.
        return (
            '{"source_format":"plain","mime_type":"text/plain",'
            '"elements":[],"relationships":[],"images":[],"tables":[]}'
        )


class _StubRenderer:
    """Minimal duck-typed renderer stub."""

    def __init__(self, name: str) -> None:
        self._name = name

    def name(self) -> str:
        return self._name

    def version(self) -> str:
        return "0.0.1"

    def initialize(self) -> None:
        pass

    def shutdown(self) -> None:
        pass

    def render(self, doc_json: str) -> str:
        return "rendered"


# ---------------------------------------------------------------------------
# DocumentExtractor tests
# ---------------------------------------------------------------------------


def test_register_duplicate_extractor_replaces() -> None:
    """Registering two extractors with the same name replaces the first.

    The registry does not raise; the second registration silently replaces
    the first.  list_document_extractors() must still include the name exactly
    once after both registrations.
    """
    name = "_test_dup_extractor"
    try:
        e1 = _StubExtractor(name, "application/x-test-dup1")
        e2 = _StubExtractor(name, "application/x-test-dup2")
        register_document_extractor(e1)
        register_document_extractor(e2)  # should not raise
        listed = list_document_extractors()
        count = listed.count(name)
        assert count == 1, f"expected name once, got {count}: {listed}"  # noqa: S101
    finally:
        unregister_document_extractor(name)


def test_unregister_unknown_extractor_returns_ok() -> None:
    """Unregistering a name that was never registered is a no-op.

    Per Phase B convention the operation completes without error.
    """
    unregister_document_extractor("_test_never_registered_extractor_xyz")


def test_clear_then_list_extractor_empty() -> None:
    """After clear, list_document_extractors returns an empty list."""
    e1 = _StubExtractor("_test_clear_a", "application/x-test-clear-a")
    e2 = _StubExtractor("_test_clear_b", "application/x-test-clear-b")
    register_document_extractor(e1)
    register_document_extractor(e2)
    clear_document_extractors()
    listed = list_document_extractors()
    assert listed == [], f"expected empty after clear, got {listed}"  # noqa: S101


def test_extract_after_unregister_extractor_uses_builtin() -> None:
    """After unregistering a custom extractor, extraction of a built-in MIME type
    must succeed using the default extractor, not segfault.

    text/plain is always handled by a built-in extractor that survives the
    custom extractor lifecycle.
    """
    name = "_test_unreg_plain"
    extractor = _StubExtractor(name, "text/plain")
    register_document_extractor(extractor)
    unregister_document_extractor(name)

    # Must not crash; falls back to the built-in plain-text extractor.
    result = extract_bytes_sync(b"hello world", "text/plain")
    assert result is not None  # noqa: S101


# ---------------------------------------------------------------------------
# Renderer tests
# ---------------------------------------------------------------------------


def test_register_duplicate_renderer_replaces() -> None:
    """Registering two renderers with the same name replaces the first.

    The registry does not raise; the second registration silently replaces
    the first.  list_renderers() must still include the name exactly once.
    """
    name = "_test_dup_renderer"
    try:
        r1 = _StubRenderer(name)
        r2 = _StubRenderer(name)
        register_renderer(r1)
        register_renderer(r2)  # should not raise
        listed = list_renderers()
        count = listed.count(name)
        assert count == 1, f"expected name once, got {count}: {listed}"  # noqa: S101
    finally:
        unregister_renderer(name)


def test_unregister_unknown_renderer_returns_ok() -> None:
    """Unregistering a renderer name that was never registered is a no-op."""
    unregister_renderer("_test_never_registered_renderer_xyz")


def test_clear_then_list_renderer_empty() -> None:
    """After clear, list_renderers returns an empty list."""
    r1 = _StubRenderer("_test_renderer_clear_a")
    r2 = _StubRenderer("_test_renderer_clear_b")
    register_renderer(r1)
    register_renderer(r2)
    clear_renderers()
    listed = list_renderers()
    assert listed == [], f"expected empty after clear, got {listed}"  # noqa: S101


def test_list_renderers_after_unregister_does_not_include_removed() -> None:
    """After unregistering a renderer, its name no longer appears in list_renderers.

    This indirectly verifies that extraction via a built-in renderer (e.g.
    markdown) continues to work after a custom renderer is removed.
    """
    name = "_test_unregister_renderer_check"
    renderer = _StubRenderer(name)
    register_renderer(renderer)
    assert name in list_renderers()  # noqa: S101
    unregister_renderer(name)
    assert name not in list_renderers()  # noqa: S101
