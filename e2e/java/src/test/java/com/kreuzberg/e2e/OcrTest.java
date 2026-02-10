package com.kreuzberg.e2e;

// CHECKSTYLE.OFF: UnusedImports - generated code
// CHECKSTYLE.OFF: LineLength - generated code
import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.ObjectMapper;
import dev.kreuzberg.BytesWithMime;
import dev.kreuzberg.ExtractionResult;
import dev.kreuzberg.Kreuzberg;
import dev.kreuzberg.config.ExtractionConfig;
import org.junit.jupiter.api.Test;

import java.nio.file.Files;
import java.nio.file.Path;
import java.util.Arrays;
import java.util.Collections;
import java.util.List;
import java.util.Map;

import static org.junit.jupiter.api.Assertions.assertTrue;
// CHECKSTYLE.ON: UnusedImports
// CHECKSTYLE.ON: LineLength

/** Auto-generated tests for ocr fixtures. */
public class OcrTest {
    private static final ObjectMapper MAPPER = new ObjectMapper();

    @Test
    public void ocrImageHelloWorld() throws Exception {
        JsonNode config = MAPPER.readTree("{\"force_ocr\":true,\"ocr\":{\"backend\":\"tesseract\",\"language\":\"eng\"}}");
        E2EHelpers.runFixture(
            "ocr_image_hello_world",
            "images/test_hello_world.png",
            config,
            Arrays.asList("tesseract", "tesseract"),
            "Requires Tesseract OCR for image text extraction.",
            true,
            result -> {
                E2EHelpers.Assertions.assertExpectedMime(result, Arrays.asList("image/png"));
                E2EHelpers.Assertions.assertMinContentLength(result, 5);
                E2EHelpers.Assertions.assertContentContainsAny(result, Arrays.asList("hello", "world"));
            }
        );
    }

    @Test
    public void ocrImageNoText() throws Exception {
        JsonNode config = MAPPER.readTree("{\"force_ocr\":true,\"ocr\":{\"backend\":\"tesseract\",\"language\":\"eng\"}}");
        E2EHelpers.runFixture(
            "ocr_image_no_text",
            "images/flower_no_text.jpg",
            config,
            Arrays.asList("tesseract", "tesseract"),
            "Skip when Tesseract is unavailable.",
            true,
            result -> {
                E2EHelpers.Assertions.assertExpectedMime(result, Arrays.asList("image/jpeg"));
                E2EHelpers.Assertions.assertMaxContentLength(result, 200);
            }
        );
    }

    @Test
    public void ocrPaddleConfidenceFilter() throws Exception {
        JsonNode config = MAPPER.readTree("{\"force_ocr\":true,\"ocr\":{\"backend\":\"paddle-ocr\",\"language\":\"en\",\"paddle_ocr_config\":{\"min_confidence\":80.0}}}");
        E2EHelpers.runFixture(
            "ocr_paddle_confidence_filter",
            "images/ocr_image.jpg",
            config,
            Arrays.asList("paddle-ocr", "paddle-ocr", "onnxruntime"),
            "Tests confidence threshold filtering with PaddleOCR",
            true,
            result -> {
                E2EHelpers.Assertions.assertExpectedMime(result, Arrays.asList("image/jpeg"));
                E2EHelpers.Assertions.assertMinContentLength(result, 1);
            }
        );
    }

    @Test
    public void ocrPaddleImageChinese() throws Exception {
        JsonNode config = MAPPER.readTree("{\"force_ocr\":true,\"ocr\":{\"backend\":\"paddle-ocr\",\"language\":\"ch\"}}");
        E2EHelpers.runFixture(
            "ocr_paddle_image_chinese",
            "images/chi_sim_image.jpeg",
            config,
            Arrays.asList("paddle-ocr", "paddle-ocr", "onnxruntime"),
            "Requires PaddleOCR with Chinese models",
            true,
            result -> {
                E2EHelpers.Assertions.assertExpectedMime(result, Arrays.asList("image/jpeg"));
                E2EHelpers.Assertions.assertMinContentLength(result, 1);
            }
        );
    }

    @Test
    public void ocrPaddleImageEnglish() throws Exception {
        JsonNode config = MAPPER.readTree("{\"force_ocr\":true,\"ocr\":{\"backend\":\"paddle-ocr\",\"language\":\"en\"}}");
        E2EHelpers.runFixture(
            "ocr_paddle_image_english",
            "images/test_hello_world.png",
            config,
            Arrays.asList("paddle-ocr", "paddle-ocr", "onnxruntime"),
            "Requires PaddleOCR with ONNX Runtime",
            true,
            result -> {
                E2EHelpers.Assertions.assertExpectedMime(result, Arrays.asList("image/png"));
                E2EHelpers.Assertions.assertMinContentLength(result, 5);
                E2EHelpers.Assertions.assertContentContainsAny(result, Arrays.asList("hello", "Hello", "world", "World"));
            }
        );
    }

    @Test
    public void ocrPaddleMarkdown() throws Exception {
        JsonNode config = MAPPER.readTree("{\"force_ocr\":true,\"ocr\":{\"backend\":\"paddle-ocr\",\"language\":\"en\",\"paddle_ocr_config\":{\"output_format\":\"markdown\"}}}");
        E2EHelpers.runFixture(
            "ocr_paddle_markdown",
            "images/test_hello_world.png",
            config,
            Arrays.asList("paddle-ocr", "paddle-ocr", "onnxruntime"),
            "Tests markdown output format parity with Tesseract",
            true,
            result -> {
                E2EHelpers.Assertions.assertExpectedMime(result, Arrays.asList("image/png"));
                E2EHelpers.Assertions.assertMinContentLength(result, 5);
                E2EHelpers.Assertions.assertContentContainsAny(result, Arrays.asList("hello", "Hello", "world", "World"));
            }
        );
    }

    @Test
    public void ocrPaddlePdfScanned() throws Exception {
        JsonNode config = MAPPER.readTree("{\"force_ocr\":true,\"ocr\":{\"backend\":\"paddle-ocr\",\"language\":\"en\"}}");
        E2EHelpers.runFixture(
            "ocr_paddle_pdf_scanned",
            "pdf/ocr_test.pdf",
            config,
            Arrays.asList("paddle-ocr", "paddle-ocr", "onnxruntime"),
            "Requires PaddleOCR with ONNX Runtime",
            true,
            result -> {
                E2EHelpers.Assertions.assertExpectedMime(result, Arrays.asList("application/pdf"));
                E2EHelpers.Assertions.assertMinContentLength(result, 20);
                E2EHelpers.Assertions.assertContentContainsAny(result, Arrays.asList("Docling", "Markdown", "JSON"));
            }
        );
    }

    @Test
    public void ocrPaddleStructured() throws Exception {
        JsonNode config = MAPPER.readTree("{\"force_ocr\":true,\"ocr\":{\"backend\":\"paddle-ocr\",\"element_config\":{\"include_elements\":true},\"language\":\"en\"}}");
        E2EHelpers.runFixture(
            "ocr_paddle_structured",
            "images/test_hello_world.png",
            config,
            Arrays.asList("paddle-ocr", "paddle-ocr", "onnxruntime"),
            "Tests structured output with bbox/confidence preservation",
            true,
            result -> {
                E2EHelpers.Assertions.assertExpectedMime(result, Arrays.asList("image/png"));
                E2EHelpers.Assertions.assertMinContentLength(result, 5);
                E2EHelpers.Assertions.assertOcrElements(result, true, true, true, null);
            }
        );
    }

    @Test
    public void ocrPaddleTableDetection() throws Exception {
        JsonNode config = MAPPER.readTree("{\"force_ocr\":true,\"ocr\":{\"backend\":\"paddle-ocr\",\"language\":\"en\",\"paddle_ocr_config\":{\"enable_table_detection\":true}}}");
        E2EHelpers.runFixture(
            "ocr_paddle_table_detection",
            "images/simple_table.png",
            config,
            Arrays.asList("paddle-ocr", "paddle-ocr", "onnxruntime"),
            "Tests table detection capability with PaddleOCR",
            true,
            result -> {
                E2EHelpers.Assertions.assertExpectedMime(result, Arrays.asList("image/png"));
                E2EHelpers.Assertions.assertMinContentLength(result, 10);
                E2EHelpers.Assertions.assertTableCount(result, 1, null);
            }
        );
    }

    @Test
    public void ocrPdfImageOnlyGerman() throws Exception {
        JsonNode config = MAPPER.readTree("{\"force_ocr\":true,\"ocr\":{\"backend\":\"tesseract\",\"language\":\"eng\"}}");
        E2EHelpers.runFixture(
            "ocr_pdf_image_only_german",
            "pdf/image_only_german_pdf.pdf",
            config,
            Arrays.asList("tesseract", "tesseract"),
            "Skip if OCR backend unavailable.",
            true,
            result -> {
                E2EHelpers.Assertions.assertExpectedMime(result, Arrays.asList("application/pdf"));
                E2EHelpers.Assertions.assertMinContentLength(result, 20);
                E2EHelpers.Assertions.assertMetadataExpectation(result, "format_type", Map.of("eq", "pdf"));
            }
        );
    }

    @Test
    public void ocrPdfRotated90() throws Exception {
        JsonNode config = MAPPER.readTree("{\"force_ocr\":true,\"ocr\":{\"backend\":\"tesseract\",\"language\":\"eng\"}}");
        E2EHelpers.runFixture(
            "ocr_pdf_rotated_90",
            "pdf/ocr_test_rotated_90.pdf",
            config,
            Arrays.asList("tesseract", "tesseract"),
            "Skip automatically when OCR backend is missing.",
            true,
            result -> {
                E2EHelpers.Assertions.assertExpectedMime(result, Arrays.asList("application/pdf"));
                E2EHelpers.Assertions.assertMinContentLength(result, 10);
            }
        );
    }

    @Test
    public void ocrPdfTesseract() throws Exception {
        JsonNode config = MAPPER.readTree("{\"force_ocr\":true,\"ocr\":{\"backend\":\"tesseract\",\"language\":\"eng\"}}");
        E2EHelpers.runFixture(
            "ocr_pdf_tesseract",
            "pdf/ocr_test.pdf",
            config,
            Arrays.asList("tesseract", "tesseract"),
            "Skip automatically if OCR backend is unavailable.",
            true,
            result -> {
                E2EHelpers.Assertions.assertExpectedMime(result, Arrays.asList("application/pdf"));
                E2EHelpers.Assertions.assertMinContentLength(result, 20);
                E2EHelpers.Assertions.assertContentContainsAny(result, Arrays.asList("Docling", "Markdown", "JSON"));
            }
        );
    }

}
