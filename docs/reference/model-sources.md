# Model Sources

Xberg downloads ML models from the HuggingFace Hub on first use and caches them
under the platform cache directory (`~/.cache/xberg/` on Linux/macOS,
`%LOCALAPPDATA%/xberg/` on Windows), or under `XBERG_CACHE_DIR` when set.

Models that Xberg hosts itself live under the
[`xberg-io`](https://huggingface.co/xberg-io) organization. Everything else is
pulled from its upstream publisher. ONNX weights are SHA256-verified after
download where a checksum is pinned.

## Self-hosted (`xberg-io`)

| Capability | Repository | Notes |
| ---------- | ---------- | ----- |
| PaddleOCR — detection, classification, recognition (per-script + unified), text-line / document orientation | [`xberg-io/paddleocr-onnx-models`](https://huggingface.co/xberg-io/paddleocr-onnx-models) | PP-OCRv5 ONNX exports. |
| Table structure — SLANeXt (wired/wireless), SLANet+, table classifier | [`xberg-io/paddleocr-onnx-models`](https://huggingface.co/xberg-io/paddleocr-onnx-models) | `v2/table/*`, `v2/classifiers/*`. |
| Layout detection — RT-DETR, TATR, PP-DocLayout-V3 | [`xberg-io/layout-models`](https://huggingface.co/xberg-io/layout-models) | |
| Named-entity recognition (GLiNER) | [`xberg-io/gliner-models`](https://huggingface.co/xberg-io/gliner-models) | xberg-managed span-mode ONNX exports and tokenizer files. Source model lineage is [`gliner-community`](https://huggingface.co/gliner-community). |

For GLiNER, Xberg downloads only the exported artifacts listed in
`xberg-io/gliner-models`. If the repository is private or not publicly readable,
configure Hugging Face credentials supported by `hf-hub` before warming the
cache or running inference.

## Third-party

| Capability | Repositories |
| ---------- | ------------ |
| Embeddings | [`Xenova/all-MiniLM-L6-v2`](https://huggingface.co/Xenova/all-MiniLM-L6-v2), [`Xenova/bge-base-en-v1.5`](https://huggingface.co/Xenova/bge-base-en-v1.5), [`Xenova/bge-large-en-v1.5`](https://huggingface.co/Xenova/bge-large-en-v1.5), [`intfloat/multilingual-e5-base`](https://huggingface.co/intfloat/multilingual-e5-base) |
| Reranking | [`BAAI/bge-reranker-base`](https://huggingface.co/BAAI/bge-reranker-base), [`rozgo/bge-reranker-v2-m3`](https://huggingface.co/rozgo/bge-reranker-v2-m3), [`jinaai/jina-reranker-v1-turbo-en`](https://huggingface.co/jinaai/jina-reranker-v1-turbo-en), [`jinaai/jina-reranker-v2-base-multilingual`](https://huggingface.co/jinaai/jina-reranker-v2-base-multilingual) |
| Transcription (Whisper) | [`onnx-community/whisper-tiny`](https://huggingface.co/onnx-community/whisper-tiny), [`onnx-community/whisper-base`](https://huggingface.co/onnx-community/whisper-base), [`onnx-community/whisper-small`](https://huggingface.co/onnx-community/whisper-small), [`Xenova/whisper-medium`](https://huggingface.co/Xenova/whisper-medium), [`Xenova/whisper-large-v3`](https://huggingface.co/Xenova/whisper-large-v3) |
| Tokenizers (token counting / chunk sizing) | [`Xenova/gpt-4o`](https://huggingface.co/Xenova/gpt-4o), [`thenlper/gte-small`](https://huggingface.co/thenlper/gte-small) |

You can point the reranker and embedding `Custom` presets at any compatible
ONNX repository on the Hub; see the [reranking](../guides/reranking.md) guide.
