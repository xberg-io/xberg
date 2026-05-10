```kotlin title="Kotlin"
import dev.kreuzberg.*
import java.nio.file.Paths
import java.util.Optional

fun main() {
    val ocr = OcrConfig.builder()
        .withBackend("tesseract")
        .withLanguage("eng+deu")
        .build()

    val config = ExtractionConfig.builder()
        .withOcr(Optional.of(ocr))
        .build()

    val result = Kreuzberg.extractFileSync(Paths.get("multilingual.pdf"), null, config)
    println(result.content())
}
```
