```kotlin title="Kotlin"
import dev.kreuzberg.*
import java.util.Optional

fun main() {
    val config = EmbeddingConfig.builder()
        .withModel(EmbeddingModelType.Preset("balanced"))
        .withNormalize(true)
        .build()

    val texts = listOf("Hello, world!", "Kreuzberg is fast")
    val embeddings = Kreuzberg.embedTexts(texts, config)

    println("Texts embedded: ${embeddings.size}")
    println("Dimensions: ${embeddings[0].size}")
}
```
