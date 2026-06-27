```kotlin title="Kotlin"
import io.xberg.*
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

    val resultOutput = Xberg.extract(
        ExtractInput(kind = ExtractInputKind.URI, uri = "scanned.pdf"),
        config,
    )
    val result = resultOutput.results().first()
    println("OCR text: ${result.content()}")
}
```
