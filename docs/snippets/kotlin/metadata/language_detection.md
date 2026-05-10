```kotlin title="Kotlin"
import dev.kreuzberg.*
import java.nio.file.Paths
import java.util.Optional

fun main() {
    val languageDetection = LanguageDetectionConfig.builder()
        .withEnabled(true)
        .withMinConfidence(0.9)
        .withDetectMultiple(false)
        .build()

    val config = ExtractionConfig.builder()
        .withLanguageDetection(Optional.of(languageDetection))
        .build()

    val result = Kreuzberg.extractFileSync(Paths.get("document.pdf"), null, config)
    println("Detected languages: ${result.detectedLanguages()}")
}
```
