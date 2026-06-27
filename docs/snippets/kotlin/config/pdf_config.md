```kotlin title="Kotlin"
import io.xberg.*
import java.util.Optional

fun main() {
    val hierarchy = HierarchyConfig.builder()
        .withEnabled(true)
        .build()

    val pdf = PdfConfig.builder()
        .withExtractImages(true)
        .withPasswords(Optional.of(listOf("password123")))
        .withExtractMetadata(true)
        .withHierarchy(Optional.of(hierarchy))
        .build()

    val config = ExtractionConfig.builder()
        .withPdfOptions(Optional.of(pdf))
        .build()

    val resultOutput = Xberg.extract(
        ExtractInput(kind = ExtractInputKind.URI, uri = "encrypted.pdf"),
        config,
    )
    val result = resultOutput.results().first()
    println("Title: ${result.metadata().title()}")
    println("Authors: ${result.metadata().authors()}")
}
```
