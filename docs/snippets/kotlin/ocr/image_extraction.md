```kotlin title="Kotlin"
import io.xberg.*
import java.nio.file.Paths
import java.util.Optional

fun main() {
    val images = ImageExtractionConfig.builder()
        .withExtractImages(true)
        .build()

    val config = ExtractionConfig.builder()
        .withImages(Optional.of(images))
        .build()

    val result = Xberg.extractSync(Paths.get("document.pdf"), null, config)
    println("Extracted images: ${result.images()?.size ?: 0}")
}
```
