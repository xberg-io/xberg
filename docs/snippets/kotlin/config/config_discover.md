```kotlin title="Kotlin"
import io.xberg.*
import java.util.Optional

fun main() {
    // Java/Kotlin bindings construct configuration explicitly via the builder.
    // Equivalent to ExtractionConfig::discover() in Rust: load defaults and override
    // any fields you want to override.
    val config = ExtractionConfig.builder()
        .withUseCache(true)
        .build()

    val resultOutput = Xberg.extract(
        ExtractInput(kind = ExtractInputKind.URI, uri = "document.pdf"),
        config,
    )
    val result = resultOutput.results().first()
    println(result.content())
}
```
