```kotlin title="Kotlin"
import io.xberg.*
import java.nio.file.Paths
import java.util.Optional

fun main() {
    val keywords = KeywordConfig.builder()
        .withAlgorithm(KeywordAlgorithm.Yake)
        .withMaxKeywords(10L)
        .withMinScore(0.3f)
        .build()

    val config = ExtractionConfig.builder()
        .withKeywords(Optional.of(keywords))
        .build()

    val result = Xberg.extractSync(Paths.get("research_paper.pdf"), null, config)
    result.extractedKeywords()?.let { extracted ->
        println("Keywords: $extracted")
    }
}
```
