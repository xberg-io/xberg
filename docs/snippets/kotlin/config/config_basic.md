```kotlin title="Kotlin"
import io.xberg.*
import java.util.Optional

fun main() {
    val config = ExtractionConfig.builder()
        .withUseCache(true)
        .withEnableQualityProcessing(true)
        .build()

    val resultOutput = Xberg.extract(
        ExtractInput(kind = ExtractInputKind.URI, uri = "document.pdf"),
        config,
    )
    val result = resultOutput.results().first()
    println(result.content())
}
```
