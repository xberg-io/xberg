```kotlin title="Kotlin"
import io.xberg.*
import java.nio.file.Paths
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

    val result = Xberg.extractSync(Paths.get("document.pdf"), null, config)
    println("Detected languages: ${result.detectedLanguages()}")
}
```
