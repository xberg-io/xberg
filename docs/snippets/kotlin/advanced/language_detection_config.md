```kotlin title="Kotlin"
import io.xberg.*
import java.util.Optional

fun main() {
    val languageDetection = LanguageDetectionConfig.builder()
        .withEnabled(true)
        .withMinConfidence(0.8)
        .withDetectMultiple(false)
        .build()

    val config = ExtractionConfig.builder()
        .withLanguageDetection(Optional.of(languageDetection))
        .build()

    val resultOutput = Xberg.extract(
        ExtractInput(kind = ExtractInputKind.URI, uri = "document.pdf"),
        config,
    )
    val result = resultOutput.results().first()
    println("Detected languages: ${result.detectedLanguages()}")
}
```
