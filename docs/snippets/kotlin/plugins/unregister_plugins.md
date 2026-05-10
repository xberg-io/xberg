```kotlin title="Kotlin"
import dev.kreuzberg.*

fun unregisterPlugins() {
    // Each plugin type has a static unregister helper on its bridge class.
    // The string argument is the name returned by the plugin's name() method.
    PostProcessorBridge.unregisterPostProcessor("word-count")
    ValidatorBridge.unregisterValidator("min-length-validator")
    OcrBackendBridge.unregisterOcrBackend("my-ocr-backend")
    EmbeddingBackendBridge.unregisterEmbeddingBackend("my-embedder")
}
```
