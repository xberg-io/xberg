package com.kreuzberg.test_app;

import static org.assertj.core.api.Assertions.*;

import com.kreuzberg.e2e.E2EHelpers;
import dev.kreuzberg.*;
import dev.kreuzberg.config.*;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.List;
import java.util.Optional;
import java.util.concurrent.CompletableFuture;
import org.junit.jupiter.api.DisplayName;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.io.TempDir;

/**
 * Comprehensive API coverage tests for Kreuzberg Java bindings.
 *
 * <p>Tests async operations, embedding presets, configuration discovery, and all major
 * extraction/detection APIs.
 */
@DisplayName("Kreuzberg API Coverage Tests")
class ApiCoverageTest {

  /** Tests async file extraction with proper completion handling */
  @Test
  @DisplayName("extractFileAsync returns CompletableFuture that completes successfully")
  void testExtractFileAsync(@TempDir Path tempDir) throws Exception {
    Path pdfFile = E2EHelpers.resolveDocument("gmft/tiny.pdf");
    if (!Files.exists(pdfFile)) {
      return; // Skip if test document not available
    }

    CompletableFuture<ExtractionResult> future = Kreuzberg.extractFileAsync(pdfFile, null);

    assertThat(future).isNotNull();
    ExtractionResult result = future.get();
    assertThat(result).isNotNull();
    assertThat(result.getContent()).isNotEmpty();
  }

  /** Tests async bytes extraction */
  @Test
  @DisplayName("extractBytesAsync extracts from byte array")
  void testExtractBytesAsync() throws Exception {
    byte[] textBytes = "Hello World".getBytes();
    ExtractionConfig config = ExtractionConfig.builder().build();

    CompletableFuture<ExtractionResult> future =
        Kreuzberg.extractBytesAsync(textBytes, "text/plain", config);

    assertThat(future).isNotNull();
    ExtractionResult result = future.get();
    assertThat(result).isNotNull();
  }

  /** Tests async batch file extraction */
  @Test
  @DisplayName("batchExtractFilesAsync handles multiple files")
  void testBatchExtractFilesAsync() throws Exception {
    Path pdfFile = E2EHelpers.resolveDocument("gmft/tiny.pdf");
    if (!Files.exists(pdfFile)) {
      return;
    }

    List<String> paths = List.of(pdfFile.toString());
    CompletableFuture<List<ExtractionResult>> future =
        Kreuzberg.batchExtractFilesAsync(paths, null);

    assertThat(future).isNotNull();
    List<ExtractionResult> results = future.get();
    assertThat(results).isNotEmpty();
  }

  /** Tests async batch bytes extraction */
  @Test
  @DisplayName("batchExtractBytesAsync handles multiple byte arrays")
  void testBatchExtractBytesAsync() throws Exception {
    BytesWithMime item1 = new BytesWithMime("Hello World".getBytes(), "text/plain");
    List<BytesWithMime> items = List.of(item1);

    CompletableFuture<List<ExtractionResult>> future =
        Kreuzberg.batchExtractBytesAsync(items, null);

    assertThat(future).isNotNull();
    List<ExtractionResult> results = future.get();
    assertThat(results).isNotEmpty();
  }

  /** Tests MIME type detection from file path */
  @Test
  @DisplayName("detectMimeTypeFromPath identifies file type correctly")
  void testDetectMimeTypeFromPath(@TempDir Path tempDir) throws Exception {
    Path testFile = tempDir.resolve("test.txt");
    Files.writeString(testFile, "Hello World");

    String mimeType = Kreuzberg.detectMimeTypeFromPath(testFile.toString());

    assertThat(mimeType).isNotNull();
    assertThat(mimeType).isNotEmpty();
    assertThat(mimeType.toLowerCase()).contains("text");
  }

  /** Tests MIME type detection from bytes */
  @Test
  @DisplayName("detectMimeType from bytes identifies PDF correctly")
  void testDetectMimeTypeFromBytes() throws Exception {
    byte[] pdfHeader = "%PDF-1.4\n".getBytes();
    String mimeType = Kreuzberg.detectMimeType(pdfHeader);

    assertThat(mimeType).isNotNull();
    assertThat(mimeType.toLowerCase()).contains("pdf");
  }

  /** Tests MIME type validation */
  @Test
  @DisplayName("validateMimeType validates MIME type strings")
  void testValidateMimeType() throws Exception {
    String validated = Kreuzberg.validateMimeType("text/plain");

    assertThat(validated).isNotNull();
    assertThat(validated).isNotEmpty();
  }

  /** Tests file extensions lookup for MIME type */
  @Test
  @DisplayName("getExtensionsForMime returns file extensions")
  void testGetExtensionsForMime() throws Exception {
    List<String> extensions = Kreuzberg.getExtensionsForMime("text/plain");

    assertThat(extensions).isNotNull();
    assertThat(extensions).isNotEmpty();
    assertThat(extensions).contains("txt");
  }

  /** Tests listing embedding presets */
  @Test
  @DisplayName("listEmbeddingPresets returns available presets")
  void testListEmbeddingPresets() throws Exception {
    List<String> presets = Kreuzberg.listEmbeddingPresets();

    assertThat(presets).isNotNull();
    // Presets may be empty, that's ok
    assertThat(presets).isInstanceOf(List.class);
  }

  /** Tests getting specific embedding preset */
  @Test
  @DisplayName("getEmbeddingPreset returns Optional of preset")
  void testGetEmbeddingPreset() throws Exception {
    Optional<EmbeddingPreset> preset = Kreuzberg.getEmbeddingPreset("openai");

    assertThat(preset).isNotNull();
    // Preset may not exist, Optional handles that
  }

  /** Tests configuration discovery */
  @Test
  @DisplayName("discoverExtractionConfig returns Optional config")
  void testDiscoverExtractionConfig() throws Exception {
    Optional<ExtractionConfig> config = Kreuzberg.discoverExtractionConfig();

    assertThat(config).isNotNull();
    assertThat(config).isInstanceOf(Optional.class);
  }

  /** Tests loading configuration from file */
  @Test
  @DisplayName("loadExtractionConfigFromFile loads TOML config")
  void testLoadExtractionConfigFromFile(@TempDir Path tempDir) throws Exception {
    Path configFile = tempDir.resolve("config.toml");
    Files.writeString(configFile, "[chunking]\nmax_chars = 256\n");

    ExtractionConfig config = Kreuzberg.loadExtractionConfigFromFile(configFile);

    assertThat(config).isNotNull();
    assertThat(config.getChunking()).isNotNull();
    assertThat(config.getChunking().getMaxChars()).isEqualTo(256);
  }

  /** Tests version retrieval */
  @Test
  @DisplayName("getVersion returns version string")
  void testGetVersion() throws Exception {
    String version = Kreuzberg.getVersion();

    assertThat(version).isNotNull();
    assertThat(version).isNotEmpty();
    // Version should follow semantic versioning
    assertThat(version).matches("\\d+\\.\\d+\\.\\d+.*");
  }

  /** Tests listing post-processors when empty */
  @Test
  @DisplayName("listPostProcessors returns empty list initially")
  void testListPostProcessorsEmpty() throws Exception {
    Kreuzberg.clearPostProcessors();
    List<String> processors = Kreuzberg.listPostProcessors();

    assertThat(processors).isNotNull();
    assertThat(processors).isEmpty();
  }

  /** Tests listing validators when empty */
  @Test
  @DisplayName("listValidators returns empty list initially")
  void testListValidatorsEmpty() throws Exception {
    Kreuzberg.clearValidators();
    List<String> validators = Kreuzberg.listValidators();

    assertThat(validators).isNotNull();
    assertThat(validators).isEmpty();
  }

  /**
   * Tests listing OCR backends - verifies list contains expected backends
   *
   * <p>Note: This test has been modified to not clear all backends permanently, as that would break
   * subsequent OCR tests. Instead, it verifies that the list is not empty and contains the
   * Tesseract backend.
   */
  @Test
  @DisplayName("listOCRBackends returns non-empty list with Tesseract")
  void testListOCRBackendsEmpty() throws Exception {
    List<String> backends = Kreuzberg.listOCRBackends();

    assertThat(backends).isNotNull();
    // Should contain the built-in Tesseract backend
    assertThat(backends).contains("tesseract");
  }

  /** Tests registering a custom post-processor */
  @Test
  @DisplayName("registerPostProcessor and unregister cycle works")
  void testPostProcessorRegistration() throws Exception {
    PostProcessor processor = result -> result;

    assertThatNoException()
        .isThrownBy(() -> Kreuzberg.registerPostProcessor("test-processor", processor));

    List<String> processors = Kreuzberg.listPostProcessors();
    assertThat(processors).contains("test-processor");

    assertThatNoException().isThrownBy(() -> Kreuzberg.unregisterPostProcessor("test-processor"));
  }

  /** Tests registering a custom validator */
  @Test
  @DisplayName("registerValidator and unregister cycle works")
  void testValidatorRegistration() throws Exception {
    Validator validator = result -> {};

    assertThatNoException()
        .isThrownBy(() -> Kreuzberg.registerValidator("test-validator", validator));

    List<String> validators = Kreuzberg.listValidators();
    assertThat(validators).contains("test-validator");

    assertThatNoException().isThrownBy(() -> Kreuzberg.unregisterValidator("test-validator"));
  }

  /** Tests registering a custom OCR backend */
  @Test
  @DisplayName("registerOcrBackend and unregister cycle works")
  void testOcrBackendRegistration() throws Exception {
    OcrBackend backend = (data, configJson) -> "OCR Result";

    assertThatNoException()
        .isThrownBy(() -> Kreuzberg.registerOcrBackend("test-ocr", backend, List.of("eng")));

    List<String> backends = Kreuzberg.listOCRBackends();
    assertThat(backends).contains("test-ocr");

    assertThatNoException().isThrownBy(() -> Kreuzberg.unregisterOCRBackend("test-ocr"));
  }

  /** Tests document extractor listing */
  @Test
  @DisplayName("listDocumentExtractors returns list of extractors")
  void testListDocumentExtractors() throws Exception {
    List<String> extractors = Kreuzberg.listDocumentExtractors();

    assertThat(extractors).isNotNull();
    // Should have at least some built-in extractors
    assertThat(extractors).isInstanceOf(List.class);
  }

  /** Tests unregistering document extractor */
  @Test
  @DisplayName("unregisterDocumentExtractor handles non-existent extractor")
  void testUnregisterDocumentExtractor() throws Exception {
    assertThatNoException()
        .isThrownBy(() -> Kreuzberg.unregisterDocumentExtractor("nonexistent-extractor"));
  }

  /** Tests clearing document extractors */
  @Test
  @DisplayName("clearDocumentExtractors empties the list")
  void testClearDocumentExtractors() throws Exception {
    Kreuzberg.clearDocumentExtractors();
    List<String> extractors = Kreuzberg.listDocumentExtractors();

    assertThat(extractors).isEmpty();
  }

  /** Tests extracting with explicit null config */
  @Test
  @DisplayName("extractFile with null config uses defaults")
  void testExtractWithNullConfig() throws Exception {
    Path pdfFile = E2EHelpers.resolveDocument("gmft/tiny.pdf");
    if (!Files.exists(pdfFile)) {
      return;
    }

    ExtractionResult result = Kreuzberg.extractFile(pdfFile, null);

    assertThat(result).isNotNull();
    assertThat(result.getContent()).isNotEmpty();
  }

  /** Tests batch extraction with empty list */
  @Test
  @DisplayName("batchExtractFiles handles empty list gracefully")
  void testBatchExtractFilesEmpty() throws Exception {
    List<ExtractionResult> results = Kreuzberg.batchExtractFiles(List.of(), null);

    assertThat(results).isEmpty();
  }

  /** Tests batch extraction with bytes and empty list */
  @Test
  @DisplayName("batchExtractBytes handles empty list gracefully")
  void testBatchExtractBytesEmpty() throws Exception {
    List<ExtractionResult> results = Kreuzberg.batchExtractBytes(List.of(), null);

    assertThat(results).isEmpty();
  }

  /** Tests configuration from JSON with complex structure */
  @Test
  @DisplayName("ExtractionConfig handles complex JSON structure")
  void testComplexConfigFromJson() throws Exception {
    String json =
        """
        {
        	"output_format": "markdown",
        	"use_cache": true,
        	"ocr": {
        		"backend": "tesseract",
        		"language": "eng"
        	},
        	"chunking": {
        		"max_chars": 1024,
        		"max_overlap": 100
        	}
        }
        """;

    ExtractionConfig config = ExtractionConfig.fromJson(json);

    assertThat(config).isNotNull();
    assertThat(config.getOutputFormat()).isEqualTo("markdown");
    assertThat(config.isUseCache()).isTrue();
    assertThat(config.getOcr()).isNotNull();
    assertThat(config.getChunking()).isNotNull();
  }

  /** Tests configuration merge with multiple levels */
  @Test
  @DisplayName("ExtractionConfig merge combines nested configs")
  void testConfigMerge() throws Exception {
    ExtractionConfig config1 =
        ExtractionConfig.builder()
            .useCache(true)
            .ocr(OcrConfig.builder().backend("tesseract").build())
            .build();

    ExtractionConfig config2 = ExtractionConfig.builder().enableQualityProcessing(true).build();

    ExtractionConfig merged = config1.merge(config2);

    assertThat(merged).isNotNull();
    assertThat(merged.isUseCache()).isTrue();
    assertThat(merged.isEnableQualityProcessing()).isTrue();
  }

  /** Tests BytesWithMime record */
  @Test
  @DisplayName("BytesWithMime record works correctly")
  void testBytesWithMime() {
    byte[] data = "test data".getBytes();
    BytesWithMime item = new BytesWithMime(data, "text/plain");

    assertThat(item.data()).isEqualTo(data);
    assertThat(item.mimeType()).isEqualTo("text/plain");
  }

  /** Tests extract with Path object */
  @Test
  @DisplayName("extractFile works with Path parameter")
  void testExtractFileWithPath() throws Exception {
    Path pdfFile = E2EHelpers.resolveDocument("gmft/tiny.pdf");
    if (!Files.exists(pdfFile)) {
      return;
    }

    ExtractionResult result = Kreuzberg.extractFile(pdfFile);

    assertThat(result).isNotNull();
    assertThat(result.getContent()).isNotEmpty();
  }

  /** Tests extract with String path */
  @Test
  @DisplayName("extractFile works with String path parameter")
  void testExtractFileWithStringPath() throws Exception {
    Path pdfFile = E2EHelpers.resolveDocument("gmft/tiny.pdf");
    if (!Files.exists(pdfFile)) {
      return;
    }

    ExtractionResult result = Kreuzberg.extractFile(pdfFile.toString());

    assertThat(result).isNotNull();
    assertThat(result.getContent()).isNotEmpty();
  }

  /** Tests exception handling for null inputs */
  @Test
  @DisplayName("API throws ValidationException for null data")
  void testNullDataValidation() {
    assertThatThrownBy(() -> Kreuzberg.extractBytes(null, "text/plain", null))
        .isInstanceOf(ValidationException.class);
  }

  /** Tests exception handling for empty data */
  @Test
  @DisplayName("API throws ValidationException for empty data")
  void testEmptyDataValidation() {
    assertThatThrownBy(() -> Kreuzberg.extractBytes(new byte[0], "text/plain", null))
        .isInstanceOf(ValidationException.class);
  }

  /** Tests exception handling for null MIME type */
  @Test
  @DisplayName("API throws ValidationException for null MIME type")
  void testNullMimeTypeValidation() {
    assertThatThrownBy(() -> Kreuzberg.extractBytes("test".getBytes(), null, null))
        .isInstanceOf(ValidationException.class);
  }

  /** Tests exception handling for blank MIME type */
  @Test
  @DisplayName("API throws ValidationException for blank MIME type")
  void testBlankMimeTypeValidation() {
    assertThatThrownBy(() -> Kreuzberg.extractBytes("test".getBytes(), "   ", null))
        .isInstanceOf(ValidationException.class);
  }
}
