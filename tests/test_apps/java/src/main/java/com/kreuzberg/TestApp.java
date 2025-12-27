package com.kreuzberg;

import dev.kreuzberg.BytesWithMime;
import dev.kreuzberg.Chunk;
import dev.kreuzberg.ErrorCode;
import dev.kreuzberg.ExtractionResult;
import dev.kreuzberg.ExtractedImage;
import dev.kreuzberg.Kreuzberg;
import dev.kreuzberg.KreuzbergException;
import dev.kreuzberg.OcrBackend;
import dev.kreuzberg.OcrException;
import dev.kreuzberg.PostProcessor;
import dev.kreuzberg.ProcessingStage;
import dev.kreuzberg.Table;
import dev.kreuzberg.Validator;
import dev.kreuzberg.ValidationException;
import dev.kreuzberg.config.ChunkingConfig;
import dev.kreuzberg.config.ExtractionConfig;
import dev.kreuzberg.config.ImageExtractionConfig;
import dev.kreuzberg.config.LanguageDetectionConfig;
import dev.kreuzberg.config.OcrConfig;
import dev.kreuzberg.config.PdfConfig;
import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.Paths;
import java.util.List;
import java.util.concurrent.CompletableFuture;
import java.util.concurrent.atomic.AtomicInteger;

/**
 * Comprehensive standalone test application for Kreuzberg Java FFM API.
 *
 * <p>This test app verifies all public API surface of the Kreuzberg library including:
 * - Configuration builders and options
 * - Synchronous and asynchronous file extraction
 * - Byte array extraction
 * - Batch extraction operations
 * - MIME type detection and validation
 * - Plugin system (validators, post-processors, OCR backends)
 * - Error handling and exception types
 * - Result object structure
 *
 * <p>Run with: mvn exec:java or mvn package && java -jar target/*.jar
 */
public final class TestApp {
    private static final Path TEST_DOCUMENTS = Paths.get("../../../../test_documents").toAbsolutePath().normalize();
    private static final AtomicInteger PASSED = new AtomicInteger(0);
    private static final AtomicInteger FAILED = new AtomicInteger(0);
    private static final AtomicInteger SKIPPED = new AtomicInteger(0);

    private TestApp() {
    }

    public static void main(String[] args) {
        System.out.println("========================================");
        System.out.println("Kreuzberg Java FFM API Comprehensive Test");
        System.out.println("========================================");
        System.out.println();

        try {
            verifyLibrarySetup();
            runAllTests();
            printResults();
        } catch (Exception e) {
            System.err.println("Fatal error during test execution:");
            e.printStackTrace();
            System.exit(1);
        }
    }

    private static void verifyLibrarySetup() throws KreuzbergException {
        test("Library Version", () -> {
            String version = Kreuzberg.getVersion();
            assertNotNull(version);
            assertFalse(version.isEmpty());
            System.out.println("  Version: " + version);
        });

        test("Test Documents Exist", () -> {
            assertTrue(Files.exists(TEST_DOCUMENTS), "Test documents directory must exist at: " + TEST_DOCUMENTS);
        });
    }

    private static void runAllTests() throws Exception {
        testTypeVerification();
        testConfigurationBuilders();
        testFileExtraction();
        testByteExtraction();
        testBatchExtraction();
        testMimeTypeDetection();
        testMimeTypeValidation();
        testEmbeddingPresets();
        testErrorHandling();
        testPluginSystem();
        testResultStructure();
        testConcurrentOperations();
    }

    private static void testTypeVerification() {
        System.out.println("\n[Type Verification Tests]");

        test("ExtractionResult class accessible", () -> assertNotNull(ExtractionResult.class));
        test("ExtractionConfig class accessible", () -> assertNotNull(ExtractionConfig.class));
        test("OcrConfig class accessible", () -> assertNotNull(OcrConfig.class));
        test("ChunkingConfig class accessible", () -> assertNotNull(ChunkingConfig.class));
        test("LanguageDetectionConfig class accessible", () -> assertNotNull(LanguageDetectionConfig.class));
        test("PdfConfig class accessible", () -> assertNotNull(PdfConfig.class));
        test("ImageExtractionConfig class accessible", () -> assertNotNull(ImageExtractionConfig.class));
        test("Table class accessible", () -> assertNotNull(Table.class));
        test("Chunk class accessible", () -> assertNotNull(Chunk.class));
        test("ExtractedImage class accessible", () -> assertNotNull(ExtractedImage.class));
        test("ErrorCode enum accessible", () -> assertNotNull(ErrorCode.class));
        test("KreuzbergException class accessible", () -> assertNotNull(KreuzbergException.class));
        test("Kreuzberg API accessible", () -> assertNotNull(Kreuzberg.class));
    }

    private static void testConfigurationBuilders() {
        System.out.println("\n[Configuration Builder Tests]");

        test("Create default extraction config", () -> {
            ExtractionConfig config = ExtractionConfig.builder().build();
            assertNotNull(config);
        });

        test("Create config with cache disabled", () -> {
            ExtractionConfig config = ExtractionConfig.builder().useCache(false).build();
            assertNotNull(config);
        });

        test("Create config with quality processing enabled", () -> {
            ExtractionConfig config = ExtractionConfig.builder().enableQualityProcessing(true).build();
            assertNotNull(config);
        });

        test("Create config with OCR forced", () -> {
            ExtractionConfig config = ExtractionConfig.builder().forceOcr(true).build();
            assertNotNull(config);
        });

        test("Create config with chunking", () -> {
            ChunkingConfig chunking = ChunkingConfig.builder()
                .maxChars(1000)
                .maxOverlap(100)
                .build();
            ExtractionConfig config = ExtractionConfig.builder().chunking(chunking).build();
            assertNotNull(config.getChunking());
        });

        test("Create config with language detection", () -> {
            LanguageDetectionConfig ld = LanguageDetectionConfig.builder()
                .enabled(true)
                .build();
            ExtractionConfig config = ExtractionConfig.builder().languageDetection(ld).build();
            assertNotNull(config.getLanguageDetection());
        });

        test("Create config with PDF options", () -> {
            PdfConfig pdfConfig = PdfConfig.builder().extractImages(true).build();
            ExtractionConfig config = ExtractionConfig.builder().pdfOptions(pdfConfig).build();
            assertNotNull(config.getPdfOptions());
        });

        test("Create config with image extraction", () -> {
            ImageExtractionConfig imgConfig = ImageExtractionConfig.builder()
                .extractImages(true)
                .build();
            ExtractionConfig config = ExtractionConfig.builder().imageExtraction(imgConfig).build();
            assertNotNull(config.getImageExtraction());
        });

        test("Config toMap returns map", () -> {
            ExtractionConfig config = ExtractionConfig.builder()
                .useCache(false)
                .enableQualityProcessing(true)
                .build();
            assertNotNull(config.toMap());
        });
    }

    private static void testFileExtraction() {
        System.out.println("\n[File Extraction Tests]");

        test("Extract PDF synchronously", () -> {
            Path path = TEST_DOCUMENTS.resolve("gmft/tiny.pdf");
            if (!Files.exists(path)) {
                skipTest("PDF test file not found");
                return;
            }
            ExtractionResult result = Kreuzberg.extractFile(path);
            assertNotNull(result);
            assertFalse(result.getContent().isEmpty());
        });

        test("Extract DOCX synchronously", () -> {
            Path path = TEST_DOCUMENTS.resolve("documents/lorem_ipsum.docx");
            if (!Files.exists(path)) {
                skipTest("DOCX test file not found");
                return;
            }
            ExtractionResult result = Kreuzberg.extractFile(path);
            assertNotNull(result);
            assertFalse(result.getContent().isEmpty());
        });

        test("Extract XLSX synchronously", () -> {
            Path path = TEST_DOCUMENTS.resolve("spreadsheets/test_01.xlsx");
            if (!Files.exists(path)) {
                skipTest("XLSX test file not found");
                return;
            }
            ExtractionResult result = Kreuzberg.extractFile(path);
            assertNotNull(result);
            assertFalse(result.getContent().isEmpty());
        });

        test("Extract from string path", () -> {
            Path path = TEST_DOCUMENTS.resolve("gmft/tiny.pdf");
            if (!Files.exists(path)) {
                skipTest("PDF test file not found");
                return;
            }
            ExtractionResult result = Kreuzberg.extractFile(path.toString());
            assertNotNull(result);
            assertFalse(result.getContent().isEmpty());
        });

        test("Extract with custom config", () -> {
            Path path = TEST_DOCUMENTS.resolve("gmft/tiny.pdf");
            if (!Files.exists(path)) {
                skipTest("PDF test file not found");
                return;
            }
            ExtractionConfig config = ExtractionConfig.builder().useCache(false).build();
            ExtractionResult result = Kreuzberg.extractFile(path, config);
            assertNotNull(result);
            assertFalse(result.getContent().isEmpty());
        });

        test("Extract async returns CompletableFuture", () -> {
            Path path = TEST_DOCUMENTS.resolve("gmft/tiny.pdf");
            if (!Files.exists(path)) {
                skipTest("PDF test file not found");
                return;
            }
            ExtractionConfig config = ExtractionConfig.builder().build();
            CompletableFuture<ExtractionResult> future = Kreuzberg.extractFileAsync(path, config);
            assertNotNull(future);
            ExtractionResult result = future.join();
            assertNotNull(result);
        });

        test("Multiple async extractions work concurrently", () -> {
            Path pdf = TEST_DOCUMENTS.resolve("gmft/tiny.pdf");
            Path docx = TEST_DOCUMENTS.resolve("documents/lorem_ipsum.docx");
            if (!Files.exists(pdf) || !Files.exists(docx)) {
                skipTest("Test files not found");
                return;
            }
            ExtractionConfig config = ExtractionConfig.builder().build();
            CompletableFuture<ExtractionResult> f1 = Kreuzberg.extractFileAsync(pdf, config);
            CompletableFuture<ExtractionResult> f2 = Kreuzberg.extractFileAsync(docx, config);
            CompletableFuture<Void> all = CompletableFuture.allOf(f1, f2);
            all.join();
            assertNotNull(f1.join());
            assertNotNull(f2.join());
        });

        test("Non-existent file throws IOException", () -> {
            assertThrows(IOException.class, () -> Kreuzberg.extractFile("/nonexistent/file.pdf"));
        });

        test("Null path throws NullPointerException", () -> {
            assertThrows(NullPointerException.class, () -> Kreuzberg.extractFile((Path) null));
        });
    }

    private static void testByteExtraction() {
        System.out.println("\n[Byte Extraction Tests]");

        test("Extract bytes synchronously", () -> {
            Path path = TEST_DOCUMENTS.resolve("gmft/tiny.pdf");
            if (!Files.exists(path)) {
                skipTest("PDF test file not found");
                return;
            }
            byte[] data = Files.readAllBytes(path);
            ExtractionResult result = Kreuzberg.extractBytes(data, "application/pdf", null);
            assertNotNull(result);
            assertFalse(result.getContent().isEmpty());
        });

        test("Extract bytes with config", () -> {
            Path path = TEST_DOCUMENTS.resolve("documents/lorem_ipsum.docx");
            if (!Files.exists(path)) {
                skipTest("DOCX test file not found");
                return;
            }
            byte[] data = Files.readAllBytes(path);
            ExtractionConfig config = ExtractionConfig.builder().enableQualityProcessing(true).build();
            ExtractionResult result = Kreuzberg.extractBytes(data, "application/vnd.openxmlformats-officedocument.wordprocessingml.document", config);
            assertNotNull(result);
            assertFalse(result.getContent().isEmpty());
        });

        test("Extract bytes async", () -> {
            Path path = TEST_DOCUMENTS.resolve("spreadsheets/test_01.xlsx");
            if (!Files.exists(path)) {
                skipTest("XLSX test file not found");
                return;
            }
            byte[] data = Files.readAllBytes(path);
            CompletableFuture<ExtractionResult> future = Kreuzberg.extractBytesAsync(
                data,
                "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
                null
            );
            assertNotNull(future.join());
        });

        test("Extract null bytes throws exception", () -> {
            assertThrows(NullPointerException.class, () -> Kreuzberg.extractBytes(null, "application/pdf", null));
        });

        test("Extract empty bytes throws exception", () -> {
            assertThrows(KreuzbergException.class, () -> Kreuzberg.extractBytes(new byte[0], "application/pdf", null));
        });

        test("Extract with null mime type throws exception", () -> {
            assertThrows(KreuzbergException.class, () -> {
                Path path = TEST_DOCUMENTS.resolve("gmft/tiny.pdf");
                if (!Files.exists(path)) throw new KreuzbergException("Test file not found");
                byte[] data = Files.readAllBytes(path);
                Kreuzberg.extractBytes(data, null, null);
            });
        });
    }

    private static void testBatchExtraction() {
        System.out.println("\n[Batch Extraction Tests]");

        test("Batch extract files synchronously", () -> {
            Path pdf = TEST_DOCUMENTS.resolve("gmft/tiny.pdf");
            Path docx = TEST_DOCUMENTS.resolve("documents/lorem_ipsum.docx");
            if (!Files.exists(pdf) || !Files.exists(docx)) {
                skipTest("Test files not found");
                return;
            }
            List<String> paths = List.of(pdf.toString(), docx.toString());
            List<ExtractionResult> results = Kreuzberg.batchExtractFiles(paths, null);
            assertEquals(2, results.size());
            assertTrue(results.stream().allMatch(r -> !r.getContent().isEmpty()));
        });

        test("Batch extract with config", () -> {
            Path pdf = TEST_DOCUMENTS.resolve("gmft/tiny.pdf");
            if (!Files.exists(pdf)) {
                skipTest("Test file not found");
                return;
            }
            List<String> paths = List.of(pdf.toString());
            ExtractionConfig config = ExtractionConfig.builder().useCache(false).build();
            List<ExtractionResult> results = Kreuzberg.batchExtractFiles(paths, config);
            assertEquals(1, results.size());
        });

        test("Batch extract bytes synchronously", () -> {
            Path pdf = TEST_DOCUMENTS.resolve("gmft/tiny.pdf");
            Path docx = TEST_DOCUMENTS.resolve("documents/lorem_ipsum.docx");
            if (!Files.exists(pdf) || !Files.exists(docx)) {
                skipTest("Test files not found");
                return;
            }
            byte[] pdf_data = Files.readAllBytes(pdf);
            byte[] docx_data = Files.readAllBytes(docx);
            List<BytesWithMime> items = List.of(
                new BytesWithMime(pdf_data, "application/pdf"),
                new BytesWithMime(docx_data, "application/vnd.openxmlformats-officedocument.wordprocessingml.document")
            );
            List<ExtractionResult> results = Kreuzberg.batchExtractBytes(items, null);
            assertEquals(2, results.size());
        });

        test("Batch extract bytes async", () -> {
            Path pdf = TEST_DOCUMENTS.resolve("gmft/tiny.pdf");
            if (!Files.exists(pdf)) {
                skipTest("Test file not found");
                return;
            }
            byte[] data = Files.readAllBytes(pdf);
            List<BytesWithMime> items = List.of(new BytesWithMime(data, "application/pdf"));
            CompletableFuture<List<ExtractionResult>> future = Kreuzberg.batchExtractBytesAsync(items, null);
            List<ExtractionResult> results = future.join();
            assertEquals(1, results.size());
        });

        test("Batch extract empty list returns empty", () -> {
            List<ExtractionResult> results = Kreuzberg.batchExtractFiles(List.of(), null);
            assertEquals(0, results.size());
        });

        test("Batch extract files async", () -> {
            Path pdf = TEST_DOCUMENTS.resolve("gmft/tiny.pdf");
            if (!Files.exists(pdf)) {
                skipTest("Test file not found");
                return;
            }
            List<String> paths = List.of(pdf.toString());
            CompletableFuture<List<ExtractionResult>> future = Kreuzberg.batchExtractFilesAsync(paths, null);
            List<ExtractionResult> results = future.join();
            assertEquals(1, results.size());
        });

        test("Batch extract null list throws exception", () -> {
            assertThrows(NullPointerException.class, () -> Kreuzberg.batchExtractFiles(null, null));
        });

        test("Batch extract bytes null list throws exception", () -> {
            assertThrows(NullPointerException.class, () -> Kreuzberg.batchExtractBytes(null, null));
        });
    }

    private static void testMimeTypeDetection() {
        System.out.println("\n[MIME Type Detection Tests]");

        test("Detect MIME type from PDF bytes", () -> {
            Path path = TEST_DOCUMENTS.resolve("gmft/tiny.pdf");
            if (!Files.exists(path)) {
                skipTest("PDF test file not found");
                return;
            }
            byte[] data = Files.readAllBytes(path);
            String mimeType = Kreuzberg.detectMimeType(data);
            assertNotNull(mimeType);
            assertFalse(mimeType.isEmpty());
        });

        test("Detect MIME type from file path", () -> {
            Path path = TEST_DOCUMENTS.resolve("gmft/tiny.pdf");
            if (!Files.exists(path)) {
                skipTest("PDF test file not found");
                return;
            }
            String mimeType = Kreuzberg.detectMimeType(path.toString());
            assertNotNull(mimeType);
            assertFalse(mimeType.isEmpty());
        });

        test("Detect MIME type from file path with checkExists", () -> {
            Path path = TEST_DOCUMENTS.resolve("gmft/tiny.pdf");
            if (!Files.exists(path)) {
                skipTest("PDF test file not found");
                return;
            }
            String mimeType = Kreuzberg.detectMimeType(path.toString(), true);
            assertNotNull(mimeType);
        });

        test("Detect MIME type from bytes null throws exception", () -> {
            assertThrows(NullPointerException.class, () -> Kreuzberg.detectMimeType((byte[]) null));
        });

        test("Detect MIME type from path null throws exception", () -> {
            assertThrows(NullPointerException.class, () -> Kreuzberg.detectMimeType((String) null));
        });

        test("Detect MIME type from empty bytes throws exception", () -> {
            assertThrows(KreuzbergException.class, () -> Kreuzberg.detectMimeType(new byte[0]));
        });
    }

    private static void testMimeTypeValidation() {
        System.out.println("\n[MIME Type Validation Tests]");

        test("Validate MIME type", () -> {
            String validated = Kreuzberg.validateMimeType("application/pdf");
            assertNotNull(validated);
            assertFalse(validated.isEmpty());
        });

        test("Validate null MIME type throws exception", () -> {
            assertThrows(KreuzbergException.class, () -> Kreuzberg.validateMimeType(null));
        });

        test("Get extensions for MIME type", () -> {
            List<String> extensions = Kreuzberg.getExtensionsForMime("application/pdf");
            assertNotNull(extensions);
        });

        test("Get extensions for null MIME type throws exception", () -> {
            assertThrows(KreuzbergException.class, () -> Kreuzberg.getExtensionsForMime(null));
        });

        test("Get extensions for blank MIME type throws exception", () -> {
            assertThrows(KreuzbergException.class, () -> Kreuzberg.getExtensionsForMime(""));
        });
    }

    private static void testEmbeddingPresets() {
        System.out.println("\n[Embedding Presets Tests]");

        test("List embedding presets", () -> {
            List<String> presets = Kreuzberg.listEmbeddingPresets();
            assertNotNull(presets);
        });

        test("Get embedding preset by name", () -> {
            List<String> presets = Kreuzberg.listEmbeddingPresets();
            if (!presets.isEmpty()) {
                var preset = Kreuzberg.getEmbeddingPreset(presets.get(0));
                assertNotNull(preset);
            }
        });

        test("Get non-existent embedding preset returns empty", () -> {
            var preset = Kreuzberg.getEmbeddingPreset("non-existent-preset-xyz");
            assertFalse(preset.isPresent());
        });

        test("Get embedding preset with null name throws exception", () -> {
            assertThrows(KreuzbergException.class, () -> Kreuzberg.getEmbeddingPreset(null));
        });

        test("Get embedding preset with blank name throws exception", () -> {
            assertThrows(KreuzbergException.class, () -> Kreuzberg.getEmbeddingPreset(""));
        });
    }

    private static void testErrorHandling() {
        System.out.println("\n[Error Handling Tests]");

        test("KreuzbergException has message", () -> {
            KreuzbergException e = new KreuzbergException("Test error");
            assertEquals("Test error", e.getMessage());
            assertNotNull(e.getErrorCode());
        });

        test("KreuzbergException with cause preserves cause", () -> {
            IOException cause = new IOException("Root cause");
            KreuzbergException e = new KreuzbergException("Test error", cause);
            assertEquals(cause, e.getCause());
        });

        test("Async extraction handles exceptions", () -> {
            ExtractionConfig config = ExtractionConfig.builder().build();
            CompletableFuture<ExtractionResult> future = Kreuzberg.extractFileAsync(Path.of("/nonexistent/file.pdf"), config);
            assertThrows(Exception.class, future::join);
        });
    }

    private static void testPluginSystem() {
        System.out.println("\n[Plugin System Tests]");

        test("List post processors", () -> {
            List<String> processors = Kreuzberg.listPostProcessors();
            assertNotNull(processors);
        });

        test("List validators", () -> {
            List<String> validators = Kreuzberg.listValidators();
            assertNotNull(validators);
        });

        test("List OCR backends", () -> {
            List<String> backends = Kreuzberg.listOCRBackends();
            assertNotNull(backends);
        });

        test("List document extractors", () -> {
            List<String> extractors = Kreuzberg.listDocumentExtractors();
            assertNotNull(extractors);
        });

        test("Register and unregister post processor", () -> {
            PostProcessor processor = result -> result;
            Kreuzberg.registerPostProcessor("test-processor", processor, 0, ProcessingStage.EARLY);
            List<String> registered = Kreuzberg.listPostProcessors();
            assertTrue(registered.contains("test-processor"));
            Kreuzberg.unregisterPostProcessor("test-processor");
            List<String> afterUnregister = Kreuzberg.listPostProcessors();
            assertFalse(afterUnregister.contains("test-processor"));
        });

        test("Register and unregister validator", () -> {
            Validator validator = result -> {
            };
            Kreuzberg.registerValidator("test-validator", validator, 0);
            List<String> registered = Kreuzberg.listValidators();
            assertTrue(registered.contains("test-validator"));
            Kreuzberg.unregisterValidator("test-validator");
            List<String> afterUnregister = Kreuzberg.listValidators();
            assertFalse(afterUnregister.contains("test-validator"));
        });

        test("Register and unregister OCR backend", () -> {
            OcrBackend backend = (data, config) -> "ocr result";
            Kreuzberg.registerOcrBackend("test-ocr", backend);
            List<String> registered = Kreuzberg.listOCRBackends();
            assertTrue(registered.contains("test-ocr"));
            Kreuzberg.unregisterOCRBackend("test-ocr");
            List<String> afterUnregister = Kreuzberg.listOCRBackends();
            assertFalse(afterUnregister.contains("test-ocr"));
        });

        test("Clear post processors", () -> {
            PostProcessor p = result -> result;
            Kreuzberg.registerPostProcessor("p1", p);
            Kreuzberg.registerPostProcessor("p2", p);
            Kreuzberg.clearPostProcessors();
            List<String> processors = Kreuzberg.listPostProcessors();
            assertTrue(processors.isEmpty());
        });

        test("Clear validators", () -> {
            Validator v = result -> {
            };
            Kreuzberg.registerValidator("v1", v);
            Kreuzberg.registerValidator("v2", v);
            Kreuzberg.clearValidators();
            List<String> validators = Kreuzberg.listValidators();
            assertTrue(validators.isEmpty());
        });

        test("Clear OCR backends", () -> {
            OcrBackend b = (data, config) -> "result";
            Kreuzberg.registerOcrBackend("b1", b);
            Kreuzberg.registerOcrBackend("b2", b);
            Kreuzberg.clearOCRBackends();
            List<String> backends = Kreuzberg.listOCRBackends();
            assertTrue(backends.isEmpty());
        });

        test("Unregister document extractor", () -> {
            List<String> extractors = Kreuzberg.listDocumentExtractors();
            if (!extractors.isEmpty()) {
                String first = extractors.get(0);
                Kreuzberg.unregisterDocumentExtractor(first);
            }
        });

        test("Clear document extractors", () -> {
            Kreuzberg.clearDocumentExtractors();
            List<String> extractors = Kreuzberg.listDocumentExtractors();
            assertTrue(extractors.isEmpty());
        });
    }

    private static void testResultStructure() {
        System.out.println("\n[Result Structure Tests]");

        test("Extraction result has content", () -> {
            Path path = TEST_DOCUMENTS.resolve("gmft/tiny.pdf");
            if (!Files.exists(path)) {
                skipTest("PDF test file not found");
                return;
            }
            ExtractionResult result = Kreuzberg.extractFile(path);
            assertFalse(result.getContent().isEmpty());
        });

        test("Extraction result has MIME type", () -> {
            Path path = TEST_DOCUMENTS.resolve("gmft/tiny.pdf");
            if (!Files.exists(path)) {
                skipTest("PDF test file not found");
                return;
            }
            ExtractionResult result = Kreuzberg.extractFile(path);
            assertFalse(result.getMimeType().isEmpty());
        });

        test("Extraction result has success flag", () -> {
            Path path = TEST_DOCUMENTS.resolve("gmft/tiny.pdf");
            if (!Files.exists(path)) {
                skipTest("PDF test file not found");
                return;
            }
            ExtractionResult result = Kreuzberg.extractFile(path);
            assertTrue(result.isSuccess());
        });

        test("Extraction result has metadata", () -> {
            Path path = TEST_DOCUMENTS.resolve("gmft/tiny.pdf");
            if (!Files.exists(path)) {
                skipTest("PDF test file not found");
                return;
            }
            ExtractionResult result = Kreuzberg.extractFile(path);
            assertNotNull(result.getMetadata());
        });

        test("Extraction result has tables list", () -> {
            Path path = TEST_DOCUMENTS.resolve("gmft/tiny.pdf");
            if (!Files.exists(path)) {
                skipTest("PDF test file not found");
                return;
            }
            ExtractionResult result = Kreuzberg.extractFile(path);
            assertNotNull(result.getTables());
        });

        test("Extraction result has chunks list", () -> {
            Path path = TEST_DOCUMENTS.resolve("gmft/tiny.pdf");
            if (!Files.exists(path)) {
                skipTest("PDF test file not found");
                return;
            }
            ExtractionResult result = Kreuzberg.extractFile(path);
            assertNotNull(result.getChunks());
        });

        test("Extraction result has images list", () -> {
            Path path = TEST_DOCUMENTS.resolve("gmft/tiny.pdf");
            if (!Files.exists(path)) {
                skipTest("PDF test file not found");
                return;
            }
            ExtractionResult result = Kreuzberg.extractFile(path);
            assertNotNull(result.getImages());
        });

        test("Extraction result has detected languages", () -> {
            Path path = TEST_DOCUMENTS.resolve("gmft/tiny.pdf");
            if (!Files.exists(path)) {
                skipTest("PDF test file not found");
                return;
            }
            ExtractionResult result = Kreuzberg.extractFile(path);
            assertNotNull(result.getDetectedLanguages());
        });

        test("Extraction result toString is not empty", () -> {
            Path path = TEST_DOCUMENTS.resolve("gmft/tiny.pdf");
            if (!Files.exists(path)) {
                skipTest("PDF test file not found");
                return;
            }
            ExtractionResult result = Kreuzberg.extractFile(path);
            assertFalse(result.toString().isEmpty());
        });
    }

    private static void testConcurrentOperations() {
        System.out.println("\n[Concurrent Operations Tests]");

        test("Multiple sync extractions work correctly", () -> {
            Path pdf = TEST_DOCUMENTS.resolve("gmft/tiny.pdf");
            if (!Files.exists(pdf)) {
                skipTest("Test file not found");
                return;
            }
            ExtractionResult r1 = Kreuzberg.extractFile(pdf);
            ExtractionResult r2 = Kreuzberg.extractFile(pdf);
            assertFalse(r1.getContent().isEmpty());
            assertFalse(r2.getContent().isEmpty());
        });

        test("Batch operations with multiple files complete successfully", () -> {
            Path pdf = TEST_DOCUMENTS.resolve("gmft/tiny.pdf");
            if (!Files.exists(pdf)) {
                skipTest("Test file not found");
                return;
            }
            List<String> paths = List.of(pdf.toString(), pdf.toString());
            List<ExtractionResult> results = Kreuzberg.batchExtractFiles(paths, null);
            assertEquals(2, results.size());
        });
    }


    private static void test(String name, TestFn fn) {
        try {
            fn.run();
            System.out.println("  PASS: " + name);
            PASSED.incrementAndGet();
        } catch (SkipException e) {
            System.out.println("  SKIP: " + name + " - " + e.getMessage());
            SKIPPED.incrementAndGet();
        } catch (AssertionError | Exception e) {
            System.out.println("  FAIL: " + name);
            System.out.println("    Error: " + e.getMessage());
            if (e.getCause() != null) {
                System.out.println("    Caused by: " + e.getCause().getMessage());
            }
            FAILED.incrementAndGet();
        }
    }

    private static void skipTest(String reason) {
        throw new SkipException(reason);
    }

    private static void assertTrue(boolean condition) {
        if (!condition) {
            throw new AssertionError("Expected true but was false");
        }
    }

    private static void assertTrue(boolean condition, String message) {
        if (!condition) {
            throw new AssertionError(message);
        }
    }

    private static void assertFalse(boolean condition) {
        if (condition) {
            throw new AssertionError("Expected false but was true");
        }
    }

    private static void assertFalse(boolean condition, String message) {
        if (condition) {
            throw new AssertionError(message);
        }
    }

    private static void assertNotNull(Object value) {
        if (value == null) {
            throw new AssertionError("Expected non-null value");
        }
    }

    private static void assertNull(Object value) {
        if (value != null) {
            throw new AssertionError("Expected null value but was: " + value);
        }
    }

    private static void assertEquals(Object expected, Object actual) {
        if (!expected.equals(actual)) {
            throw new AssertionError("Expected " + expected + " but was " + actual);
        }
    }

    private static void assertThrows(Class<? extends Exception> exceptionType, TestFnThrows fn) {
        try {
            fn.run();
            throw new AssertionError("Expected " + exceptionType.getSimpleName() + " to be thrown");
        } catch (Exception e) {
            if (!exceptionType.isInstance(e)) {
                throw new AssertionError("Expected " + exceptionType.getSimpleName() + " but got " + e.getClass().getSimpleName(), e);
            }
        }
    }

    private static void printResults() {
        System.out.println("\n========================================");
        System.out.println("Test Results");
        System.out.println("========================================");
        System.out.println("Passed:  " + PASSED.get());
        System.out.println("Failed:  " + FAILED.get());
        System.out.println("Skipped: " + SKIPPED.get());
        System.out.println("Total:   " + (PASSED.get() + FAILED.get() + SKIPPED.get()));
        System.out.println();

        if (FAILED.get() > 0) {
            System.err.println("Some tests failed!");
            System.exit(1);
        }

        System.out.println("All tests passed!");
        System.exit(0);
    }


    @FunctionalInterface
    private interface TestFn {
        void run() throws Exception;
    }

    @FunctionalInterface
    private interface TestFnThrows {
        void run() throws Exception;
    }

    private static final class SkipException extends RuntimeException {
        SkipException(String message) {
            super(message);
        }
    }
}
