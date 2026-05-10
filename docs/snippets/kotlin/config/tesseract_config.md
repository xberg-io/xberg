```kotlin title="Kotlin"
import dev.kreuzberg.*
import java.nio.file.Paths
import java.util.Optional

fun main() {
    val tesseract = TesseractConfig.builder()
        .withLanguage("eng+deu")
        .withPsm(6)
        .withOem(3)
        .build()

    val ocr = OcrConfig.builder()
        .withBackend("tesseract")
        .withLanguage("eng+deu")
        .withTesseractConfig(Optional.of(tesseract))
        .build()

    val config = ExtractionConfig.builder()
        .withOcr(Optional.of(ocr))
        .build()

    val result = Kreuzberg.extractFileSync(Paths.get("scanned.pdf"), null, config)
    println("OCR text: ${result.content()}")
}
```
