```kotlin title="Kotlin"
import io.xberg.*
import java.nio.file.Paths
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

    val result = Xberg.extractSync(Paths.get("document.pdf"), null, config)
    val pages = result.pages().orEmpty()
    println("Pages: ${pages.size}")
}
```
