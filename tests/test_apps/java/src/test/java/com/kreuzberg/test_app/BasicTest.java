package com.kreuzberg.test_app;

import static org.assertj.core.api.Assertions.*;

import dev.kreuzberg.config.*;
import org.junit.jupiter.api.DisplayName;
import org.junit.jupiter.api.Test;

/**
 * Basic tests for Kreuzberg Java API.
 *
 * <p>These tests verify that the core Kreuzberg Java bindings compile and provide access to
 * configuration builders and the main API.
 */
@DisplayName("Kreuzberg Java API Basic Tests")
class BasicTest {

  @Test
  @DisplayName("OcrConfig builder works")
  void testOcrConfigBuilder() {
    OcrConfig config = OcrConfig.builder().backend("tesseract").language("eng").build();

    assertThat(config).isNotNull();
    assertThat(config.getBackend()).isEqualTo("tesseract");
    assertThat(config.getLanguage()).isEqualTo("eng");
  }

  @Test
  @DisplayName("ChunkingConfig builder works")
  void testChunkingConfigBuilder() {
    ChunkingConfig config = ChunkingConfig.builder().maxChars(512).maxOverlap(50).build();

    assertThat(config).isNotNull();
    assertThat(config.getMaxChars()).isEqualTo(512);
    assertThat(config.getMaxOverlap()).isEqualTo(50);
  }

  @Test
  @DisplayName("KeywordConfig builder works")
  void testKeywordConfigBuilder() {
    KeywordConfig config =
        KeywordConfig.builder().algorithm("yake").maxKeywords(10).language("en").build();

    assertThat(config).isNotNull();
  }

  @Test
  @DisplayName("PdfConfig builder works")
  void testPdfConfigBuilder() {
    PdfConfig config = PdfConfig.builder().extractImages(true).extractMetadata(false).build();

    assertThat(config).isNotNull();
    assertThat(config.isExtractImages()).isTrue();
    assertThat(config.isExtractMetadata()).isFalse();
  }

  @Test
  @DisplayName("ExtractionConfig builder works")
  void testExtractionConfigBuilder() {
    ExtractionConfig config =
        ExtractionConfig.builder()
            .useCache(true)
            .enableQualityProcessing(true)
            .forceOcr(false)
            .outputFormat("markdown")
            .resultFormat("unified")
            .build();

    assertThat(config).isNotNull();
    assertThat(config.isUseCache()).isTrue();
    assertThat(config.isEnableQualityProcessing()).isTrue();
    assertThat(config.isForceOcr()).isFalse();
    assertThat(config.getOutputFormat()).isEqualTo("markdown");
    assertThat(config.getResultFormat()).isEqualTo("unified");
  }

  @Test
  @DisplayName("ExtractionConfig with nested configs")
  void testExtractionConfigWithNestedConfigs() {
    ExtractionConfig config =
        ExtractionConfig.builder()
            .ocr(OcrConfig.builder().backend("tesseract").build())
            .chunking(ChunkingConfig.builder().maxChars(512).build())
            .languageDetection(LanguageDetectionConfig.builder().enabled(false).build())
            .build();

    assertThat(config.getOcr()).isNotNull();
    assertThat(config.getChunking()).isNotNull();
    assertThat(config.getLanguageDetection()).isNotNull();
  }

  @Test
  @DisplayName("ExtractionConfig.toJson() works")
  void testExtractionConfigToJson() throws Exception {
    ExtractionConfig config =
        ExtractionConfig.builder().useCache(true).outputFormat("markdown").build();

    String json = config.toJson();
    assertThat(json).isNotNull().isNotEmpty();
    assertThat(json).contains("use_cache");
  }

  @Test
  @DisplayName("ExtractionConfig.fromJson() works")
  void testExtractionConfigFromJson() throws Exception {
    String json = "{\"use_cache\": true, \"output_format\": \"markdown\"}";
    ExtractionConfig config = ExtractionConfig.fromJson(json);

    assertThat(config).isNotNull();
    assertThat(config.isUseCache()).isTrue();
    assertThat(config.getOutputFormat()).isEqualTo("markdown");
  }

  @Test
  @DisplayName("ExtractionConfig.merge() works")
  void testExtractionConfigMerge() throws Exception {
    ExtractionConfig config1 = ExtractionConfig.builder().useCache(true).build();

    ExtractionConfig config2 = ExtractionConfig.builder().enableQualityProcessing(true).build();

    ExtractionConfig merged = config1.merge(config2);
    assertThat(merged).isNotNull();
  }
}
