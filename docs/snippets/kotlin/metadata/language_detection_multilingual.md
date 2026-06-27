```kotlin title="Kotlin"
import io.xberg.*
import java.util.Optional

fun main() {
    val languageDetection = LanguageDetectionConfig.builder()
        .withEnabled(true)
        .withMinConfidence(0.8)
        .withDetectMultiple(true)
        .build()

    val config = ExtractionConfig.builder()
        .withLanguageDetection(Optional.of(languageDetection))
        .build()

    val resultOutput = Xberg.extract(
        ExtractInput(kind = ExtractInputKind.URI, uri = "multilingual_document.pdf"),
        config,
    )
    val result = resultOutput.results().first()

    val detected = result.detectedLanguages() ?: emptyList()
    println("Detected languages: $detected")
    for (language in detected) {
        println("  - $language")
    }
}
```
