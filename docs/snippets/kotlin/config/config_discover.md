```kotlin title="Kotlin"
import io.xberg.*
import java.nio.file.Paths
import java.util.Optional

fun main() {
    // Java/Kotlin bindings construct configuration explicitly via the builder.
    // Equivalent to ExtractionConfig::discover() in Rust: load defaults and override
    // any fields you want to override.
    val config = ExtractionConfig.builder()
        .withUseCache(true)
        .build()

    val result = Xberg.extractSync(Paths.get("document.pdf"), null, config)
    println(result.content())
}
```
