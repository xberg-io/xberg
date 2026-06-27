```kotlin title="Kotlin"
import io.xberg.*
import java.util.Optional

fun main() {
    val images = ImageExtractionConfig.builder()
        .withExtractImages(true)
        .withTargetDpi(300)
        .withMaxImageDimension(4096)
        .build()

    val config = ExtractionConfig.builder()
        .withImages(Optional.of(images))
        .build()

    val resultOutput = Xberg.extract(
        ExtractInput(kind = ExtractInputKind.URI, uri = "document.pdf"),
        config,
    )
    val result = resultOutput.results().first()
    println("Extracted images: ${result.images()?.size ?: 0}")
}
```
