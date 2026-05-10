<!-- snippet:skip -->

!!! note "Not Supported"

    The Java binding is a Panama FFM wrapper and does not currently support custom document extractors — custom plugins must be implemented in Rust. See the [Rust plugin documentation](../rust/plugin_extractor.md) for details on creating custom document extractors.

    Java currently supports the following plugin types:

    - **PostProcessor** (`IPostProcessor` / `PostProcessorBridge`) — transform extraction results
    - **Validator** (`IValidator` / `ValidatorBridge`) — validate extraction results
    - **OcrBackend** (`IOcrBackend` / `OcrBackendBridge`) — custom OCR implementations
    - **EmbeddingBackend** (`IEmbeddingBackend` / `EmbeddingBackendBridge`) — custom embedding backends
