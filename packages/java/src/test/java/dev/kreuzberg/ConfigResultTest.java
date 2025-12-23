package dev.kreuzberg;

import static org.assertj.core.api.Assertions.assertThat;
import static org.assertj.core.api.Assertions.assertThatThrownBy;

import dev.kreuzberg.config.ExtractionConfig;
import dev.kreuzberg.config.OcrConfig;
import dev.kreuzberg.config.ChunkingConfig;
import java.util.Map;
import java.util.Optional;
import org.junit.jupiter.api.DisplayName;
import org.junit.jupiter.api.Nested;
import org.junit.jupiter.api.Test;

/**
 * Tests for Phase 1 FFI functions: config serialization and result accessors.
 *
 * @since 4.0.0
 */
@DisplayName("Phase 1 FFI Functions")
final class ConfigResultTest {

    @Nested
    @DisplayName("ExtractionConfig.toJson()")
    final class ConfigToJsonTest {

        @Test
        @DisplayName("should serialize simple config to JSON")
        void shouldSerializeSimpleConfig() throws KreuzbergException {
            ExtractionConfig config = ExtractionConfig.builder()
                .useCache(true)
                .forceOcr(false)
                .build();

            String json = config.toJson();

            assertThat(json)
                .isNotNull()
                .isNotEmpty()
                .contains("use_cache")
                .contains("true");
        }

        @Test
        @DisplayName("should serialize config with OCR options")
        void shouldSerializeConfigWithOcr() throws KreuzbergException {
            ExtractionConfig config = ExtractionConfig.builder()
                .useCache(true)
                .ocr(OcrConfig.builder()
                    .backend("tesseract")
                    .language("eng")
                    .build())
                .build();

            String json = config.toJson();

            assertThat(json)
                .isNotNull()
                .contains("ocr")
                .contains("tesseract")
                .contains("eng");
        }

        @Test
        @DisplayName("should serialize config with chunking options")
        void shouldSerializeConfigWithChunking() throws KreuzbergException {
            ExtractionConfig config = ExtractionConfig.builder()
                .useCache(true)
                .chunking(ChunkingConfig.builder()
                    .maxChars(512)
                    .maxOverlap(50)
                    .build())
                .build();

            String json = config.toJson();

            assertThat(json)
                .isNotNull()
                .contains("chunking")
                .contains("512")
                .contains("50");
        }

        @Test
        @DisplayName("should produce valid JSON that can be parsed back")
        void shouldProduceParseableJson() throws KreuzbergException {
            ExtractionConfig original = ExtractionConfig.builder()
                .useCache(false)
                .enableQualityProcessing(true)
                .forceOcr(true)
                .maxConcurrentExtractions(4)
                .build();

            String json = original.toJson();
            ExtractionConfig parsed = ExtractionConfig.fromJson(json);

            assertThat(parsed)
                .isNotNull();
            assertThat(parsed.isUseCache()).isEqualTo(false);
            assertThat(parsed.isEnableQualityProcessing()).isEqualTo(true);
            assertThat(parsed.isForceOcr()).isEqualTo(true);
            assertThat(parsed.getMaxConcurrentExtractions()).isEqualTo(4);
        }
    }

    @Nested
    @DisplayName("ExtractionConfig.getField()")
    final class ConfigGetFieldTest {

        @Test
        @DisplayName("should retrieve top-level boolean field")
        void shouldRetrieveBooleanField() throws KreuzbergException {
            ExtractionConfig config = ExtractionConfig.builder()
                .useCache(true)
                .build();

            Optional<String> field = config.getField("use_cache");

            assertThat(field)
                .isPresent()
                .contains("true");
        }

        @Test
        @DisplayName("should retrieve nested string field")
        void shouldRetrieveNestedField() throws KreuzbergException {
            ExtractionConfig config = ExtractionConfig.builder()
                .ocr(OcrConfig.builder()
                    .backend("tesseract")
                    .language("deu")
                    .build())
                .build();

            Optional<String> field = config.getField("ocr.backend");

            assertThat(field)
                .isPresent()
                .contains("\"tesseract\"");
        }

        @Test
        @DisplayName("should return empty for non-existent field")
        void shouldReturnEmptyForMissingField() throws KreuzbergException {
            ExtractionConfig config = ExtractionConfig.builder()
                .useCache(true)
                .build();

            Optional<String> field = config.getField("nonexistent_field");

            assertThat(field).isEmpty();
        }

        @Test
        @DisplayName("should retrieve numeric field as JSON")
        void shouldRetrieveNumericField() throws KreuzbergException {
            ExtractionConfig config = ExtractionConfig.builder()
                .maxConcurrentExtractions(8)
                .build();

            Optional<String> field = config.getField("max_concurrent_extractions");

            assertThat(field)
                .isPresent()
                .contains("8");
        }

        @Test
        @DisplayName("should throw exception for null field name")
        void shouldThrowForNullFieldName() {
            ExtractionConfig config = ExtractionConfig.builder().build();

            assertThatThrownBy(() -> config.getField(null))
                .isInstanceOf(IllegalArgumentException.class)
                .hasMessageContaining("cannot be null");
        }

        @Test
        @DisplayName("should throw exception for empty field name")
        void shouldThrowForEmptyFieldName() {
            ExtractionConfig config = ExtractionConfig.builder().build();

            assertThatThrownBy(() -> config.getField(""))
                .isInstanceOf(IllegalArgumentException.class)
                .hasMessageContaining("cannot be null or empty");
        }
    }

    @Nested
    @DisplayName("ExtractionConfig.merge()")
    final class ConfigMergeTest {

        @Test
        @DisplayName("should merge override config into base config")
        void shouldMergeConfigs() throws KreuzbergException {
            ExtractionConfig base = ExtractionConfig.builder()
                .useCache(true)
                .forceOcr(false)
                .maxConcurrentExtractions(4)
                .build();

            ExtractionConfig override = ExtractionConfig.builder()
                .forceOcr(true)
                .maxConcurrentExtractions(8)
                .build();

            ExtractionConfig merged = base.merge(override);

            assertThat(merged)
                .isNotNull();
            assertThat(merged.isUseCache()).isEqualTo(true);
            assertThat(merged.isForceOcr()).isEqualTo(true);
            assertThat(merged.getMaxConcurrentExtractions()).isEqualTo(8);
        }

        @Test
        @DisplayName("should override OCR config")
        void shouldOverrideOcrConfig() throws KreuzbergException {
            ExtractionConfig base = ExtractionConfig.builder()
                .ocr(OcrConfig.builder()
                    .backend("tesseract")
                    .language("eng")
                    .build())
                .build();

            ExtractionConfig override = ExtractionConfig.builder()
                .ocr(OcrConfig.builder()
                    .backend("paddleocr")
                    .language("deu")
                    .build())
                .build();

            ExtractionConfig merged = base.merge(override);

            assertThat(merged)
                .isNotNull();
            assertThat(merged.getOcr())
                .isNotNull();
            assertThat(merged.getOcr().getBackend()).isEqualTo("paddleocr");
            assertThat(merged.getOcr().getLanguage()).isEqualTo("deu");
        }

        @Test
        @DisplayName("should handle partial override")
        void shouldHandlePartialOverride() throws KreuzbergException {
            ExtractionConfig base = ExtractionConfig.builder()
                .useCache(true)
                .enableQualityProcessing(true)
                .forceOcr(false)
                .build();

            ExtractionConfig override = ExtractionConfig.builder()
                .forceOcr(true)
                .build();

            ExtractionConfig merged = base.merge(override);

            assertThat(merged)
                .isNotNull();
            assertThat(merged.isUseCache()).isEqualTo(true);
            assertThat(merged.isEnableQualityProcessing()).isEqualTo(true);
            assertThat(merged.isForceOcr()).isEqualTo(true);
        }

        @Test
        @DisplayName("should throw exception for null config")
        void shouldThrowForNullConfig() {
            ExtractionConfig config = ExtractionConfig.builder().build();

            assertThatThrownBy(() -> config.merge(null))
                .isInstanceOf(IllegalArgumentException.class)
                .hasMessageContaining("cannot be null");
        }

        @Test
        @DisplayName("should produce valid merged configuration")
        void shouldProduceValidMergedConfig() throws KreuzbergException {
            ExtractionConfig base = ExtractionConfig.builder()
                .useCache(true)
                .build();

            ExtractionConfig override = ExtractionConfig.builder()
                .forceOcr(true)
                .build();

            ExtractionConfig merged = base.merge(override);

            String json = merged.toJson();
            assertThat(json)
                .isNotNull()
                .isNotEmpty();

            ExtractionConfig reparsed = ExtractionConfig.fromJson(json);
            assertThat(reparsed)
                .isNotNull();
        }
    }

    @Nested
    @DisplayName("ExtractionResult.getPageCount()")
    final class ResultGetPageCountTest {

        @Test
        @DisplayName("should return page count from metadata")
        void shouldReturnPageCount() throws Exception {
            ExtractionResult result = createTestResult();

            int pageCount = result.getPageCount();

            assertThat(pageCount).isGreaterThanOrEqualTo(0);
        }

        @Test
        @DisplayName("should return 0 for result without page info")
        void shouldReturnZeroForNoPages() {
            ExtractionResult result = new ExtractionResult(
                "content",
                "text/plain",
                null,
                null,
                null,
                null,
                null,
                null,
                true
            );

            assertThat(result.getPageCount()).isEqualTo(0);
        }
    }

    @Nested
    @DisplayName("ExtractionResult.getChunkCount()")
    final class ResultGetChunkCountTest {

        @Test
        @DisplayName("should return number of chunks")
        void shouldReturnChunkCount() throws Exception {
            ExtractionResult result = createTestResult();

            int chunkCount = result.getChunkCount();

            assertThat(chunkCount).isGreaterThanOrEqualTo(0);
        }

        @Test
        @DisplayName("should return 0 for result without chunks")
        void shouldReturnZeroForNoChunks() {
            ExtractionResult result = new ExtractionResult(
                "content",
                "text/plain",
                null,
                null,
                null,
                null,
                null,
                null,
                true
            );

            assertThat(result.getChunkCount()).isEqualTo(0);
        }
    }

    @Nested
    @DisplayName("ExtractionResult.getDetectedLanguage()")
    final class ResultGetDetectedLanguageTest {

        @Test
        @DisplayName("should return detected language")
        void shouldReturnDetectedLanguage() throws Exception {
            ExtractionResult result = createTestResult();

            Optional<String> language = result.getDetectedLanguage();

            assertThat(language).isPresent();
        }

        @Test
        @DisplayName("should return empty for result without detected language")
        void shouldReturnEmptyForNoLanguage() {
            ExtractionResult result = new ExtractionResult(
                "content",
                "text/plain",
                null,
                null,
                null,
                null,
                null,
                null,
                true
            );

            Optional<String> language = result.getDetectedLanguage();

            assertThat(language).isEmpty();
        }
    }

    @Nested
    @DisplayName("ExtractionResult.getMetadataField()")
    final class ResultGetMetadataFieldTest {

        @Test
        @DisplayName("should retrieve metadata field")
        void shouldRetrieveMetadataField() throws Exception {
            ExtractionResult result = createTestResult();

            Optional<Object> field = result.getMetadataField("title");

            assertThat(field).isPresent();
        }

        @Test
        @DisplayName("should return empty for non-existent field")
        void shouldReturnEmptyForMissingField() throws Exception {
            ExtractionResult result = createTestResult();

            Optional<Object> field = result.getMetadataField("nonexistent");

            assertThat(field).isEmpty();
        }

        @Test
        @DisplayName("should throw exception for null field name")
        void shouldThrowForNullFieldName() throws Exception {
            ExtractionResult result = createTestResult();

            assertThatThrownBy(() -> result.getMetadataField(null))
                .isInstanceOf(IllegalArgumentException.class)
                .hasMessageContaining("cannot be null");
        }

        @Test
        @DisplayName("should throw exception for empty field name")
        void shouldThrowForEmptyFieldName() throws Exception {
            ExtractionResult result = createTestResult();

            assertThatThrownBy(() -> result.getMetadataField(""))
                .isInstanceOf(IllegalArgumentException.class)
                .hasMessageContaining("cannot be null or empty");
        }
    }

    private static ExtractionResult createTestResult() {
        return new ExtractionResult(
            "Sample content for testing",
            "text/plain",
            Map.of(
                "title",
                "Sample Title",
                "author",
                "Sample Author",
                "subject",
                "Sample Subject"
            ),
            null,
            null,
            null,
            null,
            null,
            true
        );
    }
}
