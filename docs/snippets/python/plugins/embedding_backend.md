```python title="Python"
from kreuzberg import register_embedding_backend, EmbeddingConfig, embed_texts
from sentence_transformers import SentenceTransformer

# Wrap an already-loaded embedder (e.g. sentence-transformers, llama-cpp-python,
# or a tuned ONNX session) so kreuzberg can call back into it during chunking
# and standalone embed requests.
class MyEmbedder:
    def __init__(self):
        self._model = SentenceTransformer("BAAI/bge-base-en-v1.5")

    # Plugin trait hooks
    def name(self) -> str:
        return "my-embedder"

    def version(self) -> str:
        return "1.0.0"

    def initialize(self) -> None:
        # Optional warm-up; runs once at registration before dimensions() is cached.
        pass

    def shutdown(self) -> None:
        pass

    # EmbeddingBackend hooks
    def dimensions(self) -> int:
        # Captured once at registration; the dispatcher uses this for shape validation.
        return self._model.get_sentence_embedding_dimension()

    def embed(self, texts: list[str]) -> list[list[float]]:
        return self._model.encode(texts).tolist()


# Register once at startup. Reference by name in config.
register_embedding_backend(MyEmbedder())

config: EmbeddingConfig = {
    "model": {"type": "plugin", "name": "my-embedder"},
    # Optional: bound the wait on a hung backend (default: 60s; None disables)
    "max_embed_duration_secs": 30,
}
vectors = embed_texts(["Hello, world!", "Second text"], config)
```
