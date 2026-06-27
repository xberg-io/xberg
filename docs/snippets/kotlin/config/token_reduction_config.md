```kotlin title="Kotlin"
import io.xberg.*
import java.nio.file.Paths
import java.util.Optional

fun main() {
    val tokenReduction = TokenReductionOptions.builder()
        .withMode("moderate")
        .withPreserveImportantWords(true)
        .build()

    val config = ExtractionConfig.builder()
        .withTokenReduction(Optional.of(tokenReduction))
        .build()

    val result = Xberg.extractSync(Paths.get("document.pdf"), null, config)
    println("Reduced content: ${result.content()}")
}
```
