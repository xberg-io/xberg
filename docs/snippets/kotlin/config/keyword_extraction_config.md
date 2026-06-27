```kotlin title="Kotlin"
import io.xberg.*

fun main() {
    val keywords = KeywordConfig(
        algorithm = KeywordAlgorithm.YAKE,
        maxKeywords = 10L,
        minScore = 0.1f,
        ngramRange = listOf(1L, 3L),
        language = "en",
    )

    val config = ExtractionConfig(keywords = keywords)

    val output = Xberg.extract(ExtractInput(kind = ExtractInputKind.URI, uri = "document.pdf"), config)
    val result = output.results.first()
    println("Keywords: ${result.extractedKeywords}")
}
```
