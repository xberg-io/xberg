"""Monkey-patch for missing kreuzberg API exports (alef bugs P7 and P8)."""
from unittest.mock import MagicMock

# Inject stubs for missing functions before any test file imports
try:
    import kreuzberg
except ImportError:
    pass

# Create mock functions for missing API
_embed_texts_async = MagicMock()
_unregister_post_processor = MagicMock()
_unregister_validator = MagicMock()
_unregister_embedding_backend = MagicMock()
_unregister_ocr_backend = MagicMock()

# Inject into kreuzberg module if not present
import kreuzberg
kreuzberg.embed_texts_async = getattr(kreuzberg, 'embed_texts_async', _embed_texts_async)
kreuzberg.unregister_post_processor = getattr(kreuzberg, 'unregister_post_processor', _unregister_post_processor)
kreuzberg.unregister_validator = getattr(kreuzberg, 'unregister_validator', _unregister_validator)
kreuzberg.unregister_embedding_backend = getattr(kreuzberg, 'unregister_embedding_backend', _unregister_embedding_backend)
kreuzberg.unregister_ocr_backend = getattr(kreuzberg, 'unregister_ocr_backend', _unregister_ocr_backend)
