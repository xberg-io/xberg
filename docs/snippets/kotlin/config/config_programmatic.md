```kotlin title="Kotlin"
import io.xberg.*
import java.nio.file.Paths
import java.util.Optional

fun main() {
    val tesseract = TesseractConfig.builder()
        .withPsm(6)
        .build()

    val ocr = OcrConfig.builder()
        .withBackend("tesseract")
        .withLanguage("eng+deu")
        .withTesseractConfig(Optional.of(tesseract))
        .build()

    val chunking = ChunkingConfig.builder()
        .withMaxCharacters(1000L)
        .withOverlap(200L)
        .build()

    val config = ExtractionConfig.builder()
        .withUseCache(true)
        .withOcr(Optional.of(ocr))
        .withChunking(Optional.of(chunking))
        .withEnableQualityProcessing(true)
        .build()

    val result = Xberg.extractSync(Paths.get("document.pdf"), null, config)
    println("Content length: ${result.content().length}")
}
```
