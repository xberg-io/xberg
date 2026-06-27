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
        ExtractInput(kind = ExtractInputKind.URI, uri = "verbose_document.pdf"),
        config,
    )
    val result = resultOutput.results().first()
    println("Reduced content length: ${result.content().length}")
}
```
