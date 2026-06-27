```kotlin title="Kotlin"
import io.xberg.*
import java.util.Optional

fun main() {
    val config = ExtractionConfig.builder()
        .withEnableQualityProcessing(true)
        .build()

    val resultOutput = Xberg.extract(
        ExtractInput(kind = ExtractInputKind.URI, uri = "scanned_document.pdf"),
        config,
    )
    val result = resultOutput.results().first()

    val score = result.qualityScore()
    if (score != null) {
        if (score < 0.5) {
            println("Warning: Low quality extraction (%.2f)".format(score))
        } else {
            println("Quality score: %.2f".format(score))
        }
    }
}
```
