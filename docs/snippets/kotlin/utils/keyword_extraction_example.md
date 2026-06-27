```kotlin title="Kotlin"
import io.xberg.*

fun main() {
    val keywords = KeywordConfig(
        algorithm = KeywordAlgorithm.YAKE,
        maxKeywords = 10L,
        minScore = 0.3f,
    )

    val config = ExtractionConfig(keywords = keywords)

    val output = Xberg.extract(ExtractInput(kind = ExtractInputKind.URI, uri = "research_paper.pdf"), config)
    val result = output.results.first()
    result.extractedKeywords?.let { extracted ->
        println("Keywords: $extracted")
    }
}
```
