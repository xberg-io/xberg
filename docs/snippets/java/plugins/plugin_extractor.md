```markdown title="Markdown"
!!! note "Not Supported"
The Java binding is a Panama FFM wrapper and does not currently support
custom document extractors. Custom plugins must be implemented in Rust.

    See the [Rust plugin documentation](../rust/plugin_extractor.md) for details on creating custom document extractors.

    Java currently supports:
    - **PostProcessor** (`IPostProcessor` / `PostProcessorBridge`) - Transform extraction results
    - **Validator** (`IValidator` / `ValidatorBridge`) - Validate extraction results
    - **OcrBackend** (`IOcrBackend` / `OcrBackendBridge`) - Custom OCR implementations
    - **EmbeddingBackend** (`IEmbeddingBackend` / `EmbeddingBackendBridge`) - Custom embedding backends
```
