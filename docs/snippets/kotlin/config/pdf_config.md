```kotlin title="Kotlin"
import dev.kreuzberg.*
import java.nio.file.Paths
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

    val result = Kreuzberg.extractFileSync(Paths.get("encrypted.pdf"), null, config)
    println("Title: ${result.metadata().title()}")
    println("Authors: ${result.metadata().authors()}")
}
```
