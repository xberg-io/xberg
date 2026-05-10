```kotlin title="Kotlin"
import dev.kreuzberg.*
import java.nio.file.Path

class CloudOcrBackend(
    private val apiKey: String,
    private val supportedLangs: List<String>,
) : IOcrBackend {

    override fun name(): String = "cloud-ocr"

    override fun version(): String = "1.0.0"

    override fun process_image(image_bytes: ByteArray, config: OcrConfig): ExtractionResult {
        val text = callCloudApi(image_bytes, config.language())
        return ExtractionResult.builder()
            .withContent(text)
            .withMimeType("text/plain")
            .withMetadata(Metadata.builder().build())
            .build()
    }

    override fun process_image_file(path: Path, config: OcrConfig): ExtractionResult {
        return process_image(java.nio.file.Files.readAllBytes(path), config)
    }

    override fun supports_language(lang: String): Boolean = supportedLangs.contains(lang)

    override fun backend_type(): OcrBackendType = OcrBackendType.Custom

    override fun supported_languages(): List<String> = supportedLangs

    override fun supports_table_detection(): Boolean = false

    override fun supports_document_processing(): Boolean = false

    override fun process_document(_path: Path, _config: OcrConfig): ExtractionResult {
        throw UnsupportedOperationException("document processing not supported")
    }

    private fun callCloudApi(image: ByteArray, language: String): String {
        return "Extracted text"
    }
}
```
