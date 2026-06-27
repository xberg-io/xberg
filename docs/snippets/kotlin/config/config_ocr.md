```kotlin title="Kotlin"
import io.xberg.*
import java.nio.file.Paths
import java.util.Optional

fun main() {
    val ocr = OcrConfig.builder()
        .withBackend("tesseract")
        .withLanguage("eng")
        .build()

    val config = ExtractionConfig.builder()
        .withOcr(Optional.of(ocr))
        .build()

    val result = Xberg.extractSync(Paths.get("scanned.pdf"), null, config)
    println("Content length: ${result.content().length}")
    println("Tables detected: ${result.tables()?.size ?: 0}")
}
```
