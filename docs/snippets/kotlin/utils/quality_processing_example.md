```kotlin title="Kotlin"
import io.xberg.*
import java.nio.file.Paths
import java.util.Optional

fun main() {
    val config = ExtractionConfig.builder()
        .withEnableQualityProcessing(true)
        .build()

    val result = Xberg.extractSync(Paths.get("scanned_document.pdf"), null, config)

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
