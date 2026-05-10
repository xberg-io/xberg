```kotlin title="Kotlin"
import dev.kreuzberg.*
import dev.kreuzberg.kt.Kreuzberg

fun clearAllPlugins() {
    // Note: there is no Kreuzberg.clearDocumentExtractors() — extractor
    // registration is not exposed through the Kotlin/Java plugin bridge.
    Kreuzberg.clearPostProcessors()
    Kreuzberg.clearOcrBackends()
    Kreuzberg.clearValidators()

    println("All post-processors, OCR backends, and validators cleared")
}
```
