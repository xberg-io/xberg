```kotlin title="Kotlin"
import io.xberg.*
import java.util.Optional

fun main() {
    val ocr = OcrConfig.builder()
        .withBackend("paddleocr")
        .withLanguage("en")
        .build()

    val config = ExtractionConfig.builder()
        .withOcr(Optional.of(ocr))
        .build()

    val resultOutput = Xberg.extract(
        ExtractInput(kind = ExtractInputKind.URI, uri = "document.pdf"),
        config,
    )
    val result = resultOutput.results().first()
    println("Extracted text: ${result.content()}")
}
```
