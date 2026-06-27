```kotlin title="Kotlin"
import io.xberg.*
import java.util.Optional

fun main() {
    val postprocessor = PostProcessorConfig.builder()
        .withEnabled(true)
        .withEnabledProcessors(Optional.of(listOf(
            "whitespace_normalizer",
            "unicode_normalizer"
        )))
        .build()

    val config = ExtractionConfig.builder()
        .withPostprocessor(Optional.of(postprocessor))
        .build()

    val resultOutput = Xberg.extract(
        ExtractInput(kind = ExtractInputKind.URI, uri = "document.pdf"),
        config,
    )
    val result = resultOutput.results().first()
    println("Processed content: ${result.content()}")
}
```
