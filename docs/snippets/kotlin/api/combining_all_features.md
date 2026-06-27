```kotlin title="Kotlin"
import io.xberg.*
import java.util.Optional

fun main() {
    val ocr = OcrConfig.builder()
        .withBackend("tesseract")
        .withLanguage("eng")
        .build()

    val chunking = ChunkingConfig.builder()
        .withMaxCharacters(800L)
        .withOverlap(100L)
        .withChunkerType(ChunkerType.MARKDOWN)
        .withPrependHeadingContext(true)
        .build()

    val images = ImageExtractionConfig.builder()
        .withExtractImages(true)
        .build()

    val config = ExtractionConfig.builder()
        .withOcr(Optional.of(ocr))
        .withForceOcr(false)
        .withChunking(Optional.of(chunking))
        .withOutputFormat(OutputFormat.MARKDOWN)
        .withIncludeDocumentStructure(true)
        .withImages(Optional.of(images))
        .withUseCache(true)
        .withEnableQualityProcessing(true)
        .build()

    val resultOutput = Xberg.extract(
        ExtractInput(kind = ExtractInputKind.URI, uri = "report.pdf"),
        config,
    )
    val result = resultOutput.results().first()

    val content = result.content()
    println("Content (${content.length} chars):")
    println(content.take(200))

    result.chunks()?.let { println("\nChunks: ${it.size}") }
    println("Tables: ${result.tables()?.size ?: 0}")
    result.detectedLanguages()?.let { println("Languages: $it") }
    result.extractionMethod()?.let { println("Extraction method: $it") }
}
```
