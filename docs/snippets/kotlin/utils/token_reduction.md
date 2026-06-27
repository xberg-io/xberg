```kotlin title="Kotlin"
import io.xberg.*
import java.util.Optional

fun main() {
    val tokenReduction = TokenReductionOptions.builder()
        .withMode("moderate")
        .withPreserveImportantWords(true)
        .build()

    val config = ExtractionConfig.builder()
        .withTokenReduction(Optional.of(tokenReduction))
        .build()

    val resultOutput = Xberg.extract(
        ExtractInput(kind = ExtractInputKind.URI, uri = "document.pdf"),
        config,
    )
    val result = resultOutput.results().first()
    println(result.content())
}
```
