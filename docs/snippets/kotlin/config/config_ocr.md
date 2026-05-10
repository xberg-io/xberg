```kotlin title="Kotlin"
import dev.kreuzberg.*
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

    val result = Kreuzberg.extractFileSync(Paths.get("scanned.pdf"), null, config)
    println("Content length: ${result.content().length}")
    println("Tables detected: ${result.tables()?.size ?: 0}")
}
```
