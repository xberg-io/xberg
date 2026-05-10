```kotlin title="Kotlin"
import dev.kreuzberg.*

// Wrap a host-language embedding model so kreuzberg can call back into it
// during chunking and standalone embed requests.
class MyEmbedder(private val dim: Long = 768L) : IEmbeddingBackend {
    override fun name(): String = "my-embedder"
    override fun version(): String = "1.0.0"

    override fun dimensions(): Long = dim

    override fun embed(texts: List<String>): List<List<Float>> {
        // Replace this with a real model invocation. Each inner list must
        // have exactly `dimensions()` elements — the bridge validates shape.
        return texts.map { List(dim.toInt()) { 0.0f } }
    }
}

fun registerMyEmbedder() {
    EmbeddingBackendBridge.registerEmbeddingBackend(MyEmbedder())
}
```
