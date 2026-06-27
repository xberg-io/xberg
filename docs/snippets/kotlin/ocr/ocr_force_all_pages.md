```kotlin title="Kotlin"
import io.xberg.*
import java.util.Optional

fun main() {
    val ocr = OcrConfig.builder()
        .withBackend("tesseract")
        .withLanguage("eng")
        .build()

    val config = ExtractionConfig.builder()
        .withOcr(Optional.of(ocr))
        .withForceOcr(true)
        .build()

    val resultOutput = Xberg.extract(
        ExtractInput(kind = ExtractInputKind.URI, uri = "document.pdf"),
        config,
    )
    val result = resultOutput.results().first()
    println(result.content())
}
```
