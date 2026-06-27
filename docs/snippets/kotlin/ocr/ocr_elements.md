```kotlin title="Kotlin"
import io.xberg.*
import java.nio.file.Paths
import java.util.Optional

fun main() {
    val elementConfig = OcrElementConfig.builder()
        .withIncludeElements(true)
        .build()

    val ocr = OcrConfig.builder()
        .withBackend("paddleocr")
        .withLanguage("en")
        .withElementConfig(Optional.of(elementConfig))
        .build()

    val config = ExtractionConfig.builder()
        .withOcr(Optional.of(ocr))
        .build()

    val result = Xberg.extractSync(Paths.get("scanned.pdf"), null, config)

    result.ocrElements()?.forEach { element ->
        println("Text: ${element.text()}")
        println("Confidence: ${element.confidence().recognition()}")
        println("Geometry: ${element.geometry()}")
        element.rotation()?.let { println("Rotation: ${it}") }
        println()
    }
}
```
