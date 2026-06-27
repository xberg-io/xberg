```kotlin title="Kotlin"
import io.xberg.*
import java.nio.file.Paths
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

    val result = Xberg.extractSync(Paths.get("document.pdf"), null, config)
    println("Processed content: ${result.content()}")
}
```
