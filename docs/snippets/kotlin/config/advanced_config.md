```kotlin title="Kotlin"
import io.xberg.*
import java.nio.file.Paths
import java.util.Optional

fun main() {
    val ocr = OcrConfig.builder()
        .withBackend("tesseract")
        .withLanguage("eng")
        .build()

    val embedding = EmbeddingConfig.builder()
        .withModel(EmbeddingModelType.Preset("balanced"))
        .withBatchSize(32L)
        .withNormalize(true)
        .build()

    val chunking = ChunkingConfig.builder()
        .withMaxCharacters(1000L)
        .withOverlap(200L)
        .withEmbedding(Optional.of(embedding))
        .build()

    val languageDetection = LanguageDetectionConfig.builder()
        .withEnabled(true)
        .withMinConfidence(0.8)
        .withDetectMultiple(false)
        .build()

    val keywords = KeywordConfig.builder()
        .withAlgorithm(KeywordAlgorithm.Yake)
        .withMaxKeywords(10L)
        .withMinScore(0.1f)
        .withNgramRange(listOf(1L, 3L))
        .withLanguage(Optional.of("en"))
        .build()

    val tokenReduction = TokenReductionOptions.builder()
        .withMode("moderate")
        .withPreserveImportantWords(true)
        .build()

    val postprocessor = PostProcessorConfig.builder()
        .withEnabled(true)
        .build()

    val config = ExtractionConfig.builder()
        .withUseCache(true)
        .withEnableQualityProcessing(true)
        .withOcr(Optional.of(ocr))
        .withChunking(Optional.of(chunking))
        .withLanguageDetection(Optional.of(languageDetection))
        .withKeywords(Optional.of(keywords))
        .withTokenReduction(Optional.of(tokenReduction))
        .withPostprocessor(Optional.of(postprocessor))
        .build()

    val result = Xberg.extractSync(Paths.get("document.pdf"), null, config)
    println("Content: ${result.content()}")
    result.detectedLanguages()?.let { println("Languages: $it") }
    println("Chunks: ${result.chunks()?.size ?: 0}")
}
```
