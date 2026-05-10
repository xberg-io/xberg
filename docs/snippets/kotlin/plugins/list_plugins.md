```kotlin title="Kotlin"
import dev.kreuzberg.*
import dev.kreuzberg.kt.Kreuzberg

fun listAllPlugins() {
    val extractors: List<String> = Kreuzberg.listDocumentExtractors()
    println("Registered extractors: $extractors")

    val processors: List<String> = Kreuzberg.listPostProcessors()
    println("Registered post-processors: $processors")

    val backends: List<String> = Kreuzberg.listOcrBackends()
    println("Registered OCR backends: $backends")

    val validators: List<String> = Kreuzberg.listValidators()
    println("Registered validators: $validators")
}
```
