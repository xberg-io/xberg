```kotlin title="Kotlin"
import dev.kreuzberg.*
import java.nio.file.Paths
import java.util.Optional

fun main() {
    val keywords = KeywordConfig.builder()
        .withAlgorithm(KeywordAlgorithm.Yake)
        .withMaxKeywords(10L)
        .withMinScore(0.1f)
        .withNgramRange(listOf(1L, 3L))
        .withLanguage(Optional.of("en"))
        .build()

    val config = ExtractionConfig.builder()
        .withKeywords(Optional.of(keywords))
        .build()

    val result = Kreuzberg.extractFileSync(Paths.get("document.pdf"), null, config)
    println("Keywords: ${result.extractedKeywords()}")
}
```
