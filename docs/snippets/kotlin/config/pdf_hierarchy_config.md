```kotlin title="Kotlin"
import io.xberg.*
import java.util.Optional

fun main() {
    val hierarchy = HierarchyConfig.builder()
        .withEnabled(true)
        .withKClusters(5L)
        .withIncludeBbox(true)
        .withOcrCoverageThreshold(Optional.of(0.8f))
        .build()

    val pdf = PdfConfig.builder()
        .withHierarchy(Optional.of(hierarchy))
        .build()

    val config = ExtractionConfig.builder()
        .withPdfOptions(Optional.of(pdf))
        .build()

    val resultOutput = Xberg.extract(
        ExtractInput(kind = ExtractInputKind.URI, uri = "document.pdf"),
        config,
    )
    val result = resultOutput.results().first()
    val pages = result.pages().orEmpty()
    println("Pages: ${pages.size}")
}
```
