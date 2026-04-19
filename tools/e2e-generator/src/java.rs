use crate::fixtures::{
    Assertions, ExtractionMethod, Fixture, GenerationMode, InputType, PluginAssertions, PluginTestSpec,
    RenderAssertions,
};
use crate::parity::{self, ParityManifest, TypeDef};
use anyhow::{Context, Result};
use camino::Utf8Path;
use itertools::Itertools;
use serde_json::{Map, Value};
use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::fs;

const JAVA_HELPERS_TEMPLATE: &str = r#"package com.kreuzberg.e2e;

import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.ObjectMapper;
import dev.kreuzberg.ExtractionResult;
import dev.kreuzberg.Kreuzberg;
import dev.kreuzberg.MissingDependencyException;
import dev.kreuzberg.Table;
import dev.kreuzberg.config.ExtractionConfig;
import org.junit.jupiter.api.Assumptions;

import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.Paths;
import java.util.List;
import java.util.Map;

import static org.junit.jupiter.api.Assertions.assertNotNull;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertTrue;
import static org.junit.jupiter.api.Assertions.fail;

/**
 * Helper utilities for E2E tests.
 */
public final class E2EHelpers {
    private static final Path WORKSPACE_ROOT = findWorkspaceRoot();

    private static Path findWorkspaceRoot() {
        Path dir = Paths.get("").toAbsolutePath();
        while (dir != null) {
            if (Files.isDirectory(dir.resolve("test_documents"))) {
                return dir;
            }
            dir = dir.getParent();
        }
        throw new RuntimeException("Could not find workspace root (directory containing test_documents/)");
    }
    private static final Path TEST_DOCUMENTS = WORKSPACE_ROOT.resolve("test_documents");
    private static final ObjectMapper MAPPER = new ObjectMapper();

    private E2EHelpers() { }

    public static Path resolveDocument(String relativePath) {
        return TEST_DOCUMENTS.resolve(relativePath);
    }

    public static ExtractionConfig buildConfig(JsonNode configNode) throws Exception {
        if (configNode == null || configNode.isNull() || !configNode.isObject()) {
            return null;
        }
        try {
            String json = MAPPER.writeValueAsString(configNode);
            return ExtractionConfig.fromJson(json);
        } catch (Exception e) {
            throw new RuntimeException("Failed to parse config", e);
        }
    }

    public static String skipReasonFor(
            Exception error,
            String fixtureId,
            List<String> requirements,
            String notes
    ) {
        String message = error.getMessage() != null ? error.getMessage() : "";
        String lowered = message.toLowerCase();
        boolean requirementHit = requirements.stream()
                .anyMatch(req -> lowered.contains(req.toLowerCase()));
        boolean missingDependency = error instanceof MissingDependencyException
                || lowered.contains("missing dependency");
        boolean unsupportedFormat = lowered.contains("unsupported format");

        if (!missingDependency && !unsupportedFormat && !requirementHit) {
            return null;
        }

        String reason;
        if (missingDependency) {
            if (error instanceof MissingDependencyException) {
                // Extract dependency from exception message if available
                String msg = error.getMessage();
                reason = msg != null && !msg.isEmpty()
                        ? "missing dependency: " + msg
                        : "missing dependency";
            } else {
                reason = "missing dependency";
            }
        } else if (unsupportedFormat) {
            reason = "unsupported format";
        } else {
            reason = "requires " + String.join(", ", requirements);
        }

        String details = String.format(
                "Skipping %s: %s. %s: %s",
                fixtureId,
                reason,
                error.getClass().getSimpleName(),
                message
        );
        if (notes != null && !notes.isEmpty()) {
            details += " Notes: " + notes;
        }
        System.err.println(details);
        return details;
    }

    public static void skipIfFeatureUnavailable(String feature) {
        String envVar = "KREUZBERG_" + feature.replace("-", "_").toUpperCase() + "_DISABLED";
        String flag = System.getenv(envVar);
        Assumptions.assumeFalse(
                flag != null && ("1".equals(flag) || "true".equalsIgnoreCase(flag)),
                String.format("Skipping: feature '%s' disabled (via %s=1)", feature, envVar)
        );
    }

    public static void skipIfPaddleOcrUnavailable() {
        skipIfFeatureUnavailable("paddle-ocr");
    }

    public static void runFixture(
            String fixtureId,
            String relativePath,
            JsonNode configNode,
            List<String> requirements,
            String notes,
            boolean skipIfMissing,
            TestCallback callback
    ) throws Exception {
        Path documentPath = resolveDocument(relativePath);

        if (skipIfMissing && !Files.exists(documentPath)) {
            String msg = String.format("Skipping %s: missing document at %s", fixtureId, documentPath);
            System.err.println(msg);
            Assumptions.assumeTrue(false, msg);
            return;
        }

        ExtractionConfig config = buildConfig(configNode);
        ExtractionResult result;
        try {
            result = Kreuzberg.extractFile(documentPath, config);
        } catch (Exception e) {
            String skipReason = skipReasonFor(e, fixtureId, requirements, notes);
            if (skipReason != null) {
                Assumptions.assumeTrue(false, skipReason);
                return;
            }
            throw e;
        }

        callback.run(result);
    }

    @FunctionalInterface
    public interface TestCallback {
        void run(ExtractionResult result) throws Exception;
    }

    /**
     * Assertion utilities for E2E tests.
     */
    public static final class Assertions {
        public static void assertEmbedResult(float[][] results, int count, int dimensions, boolean noNan, boolean noInf, boolean nonZero, boolean normalized) {
            assertNotNull(results, "Embedding results should not be null");
            if (count >= 0) {
                org.junit.jupiter.api.Assertions.assertEquals(count, results.length, String.format("Expected %d vectors, got %d", count, results.length));
            }
            if (results.length > 0) {
                for (int i = 0; i < results.length; i++) {
                    float[] vector = results[i];
                    assertNotNull(vector, String.format("Vector %d should not be null", i));
                    if (dimensions > 0) {
                        org.junit.jupiter.api.Assertions.assertEquals(dimensions, vector.length, String.format("Vector %d expected length %d, got %d", i, dimensions, vector.length));
                    }

                    boolean hasNonZero = false;
                    double sqSum = 0.0;
                    for (int j = 0; j < vector.length; j++) {
                        float v = vector[j];
                        if (noNan) {
                            org.junit.jupiter.api.Assertions.assertFalse(Float.isNaN(v), String.format("Vector %d element %d is NaN", i, j));
                        }
                        if (noInf) {
                            org.junit.jupiter.api.Assertions.assertFalse(Float.isInfinite(v), String.format("Vector %d element %d is infinite", i, j));
                        }
                        if (v != 0.0f) {
                            hasNonZero = true;
                        }
                        sqSum += (double) v * (double) v;
                    }
                    if (nonZero) {
                        assertTrue(hasNonZero, String.format("Vector %d is all zeros", i));
                    }
                    if (normalized) {
                        double l2Norm = Math.sqrt(sqSum);
                        assertTrue(l2Norm > 0.999 && l2Norm < 1.001, String.format("Vector %d L2 norm is %f (expected ~1.0)", i, l2Norm));
                    }
                }
            }
        }

        private Assertions() { }

        public static void assertExpectedMime(ExtractionResult result, List<String> expected) {
            if (expected.isEmpty()) {
                return;
            }
            String mimeType = result.getMimeType();
            boolean matches = expected.stream()
                    .anyMatch(token -> mimeType != null && mimeType.contains(token));
            assertTrue(matches,
                    String.format("Expected mime type to contain one of %s, got %s", expected, mimeType));
        }

        public static void assertMinContentLength(ExtractionResult result, int minimum) {
            String content = result.getContent();
            int length = content != null ? content.length() : 0;
            assertTrue(length >= minimum,
                    String.format("Expected content length >= %d, got %d", minimum, length));
        }

        public static void assertMaxContentLength(ExtractionResult result, int maximum) {
            String content = result.getContent();
            int length = content != null ? content.length() : 0;
            assertTrue(length <= maximum,
                    String.format("Expected content length <= %d, got %d", maximum, length));
        }

        public static void assertContentContainsAny(ExtractionResult result, List<String> snippets) {
            if (snippets.isEmpty()) {
                return;
            }
            String content = result.getContent();
            String lowered = content != null ? content.toLowerCase() : "";
            boolean matches = snippets.stream()
                    .anyMatch(snippet -> lowered.contains(snippet.toLowerCase()));
            assertTrue(matches,
                    String.format("Expected content to contain any of %s", snippets));
        }

        public static void assertContentContainsAll(ExtractionResult result, List<String> snippets) {
            if (snippets.isEmpty()) {
                return;
            }
            String content = result.getContent();
            String lowered = content != null ? content.toLowerCase() : "";
            boolean allMatch = snippets.stream()
                    .allMatch(snippet -> lowered.contains(snippet.toLowerCase()));
            assertTrue(allMatch,
                    String.format("Expected content to contain all of %s", snippets));
        }

        public static void assertTableCount(
                ExtractionResult result,
                Integer minimum,
                Integer maximum
        ) {
            List<Table> tables = result.getTables();
            int count = tables != null ? tables.size() : 0;
            if (minimum != null) {
                assertTrue(count >= minimum,
                        String.format("Expected table count >= %d, got %d", minimum, count));
            }
            if (maximum != null) {
                assertTrue(count <= maximum,
                        String.format("Expected table count <= %d, got %d", maximum, count));
            }
        }

        public static void assertDetectedLanguages(
                ExtractionResult result,
                List<String> expected,
                Double minConfidence
        ) {
            if (expected.isEmpty()) {
                return;
            }
            List<String> languages = result.getDetectedLanguages();
            assertNotNull(languages, "Expected detected languages to be present");
            boolean allFound = expected.stream()
                    .allMatch(lang -> languages.contains(lang));
            assertTrue(allFound,
                    String.format("Expected languages %s to be in %s", expected, languages));

            if (minConfidence != null) {
                Map<String, Object> metadata = result.getMetadataMap();
                if (metadata != null && metadata.containsKey("confidence")) {
                    Object confObj = metadata.get("confidence");
                    double confidence = confObj instanceof Number
                            ? ((Number) confObj).doubleValue()
                            : 0.0;
                    assertTrue(confidence >= minConfidence,
                            String.format("Expected confidence >= %f, got %f", minConfidence, confidence));
                }
            }
        }

        public static void assertMetadataExpectation(
                ExtractionResult result,
                String path,
                Map<String, Object> expectation
        ) {
            Map<String, Object> metadata = result.getMetadataMap();
            Object value = fetchMetadataValue(metadata, path);
            assertNotNull(value, String.format("Metadata path '%s' missing", path));

            if (expectation.containsKey("eq")) {
                Object expected = expectation.get("eq");
                assertTrue(valuesEqual(value, expected),
                        String.format("Expected %s to equal %s", value, expected));
            }

            if (expectation.containsKey("gte")) {
                Object expected = expectation.get("gte");
                double actualNum = convertNumeric(value);
                double expectedNum = convertNumeric(expected);
                assertTrue(actualNum >= expectedNum,
                        String.format("Expected %f >= %f", actualNum, expectedNum));
            }

            if (expectation.containsKey("lte")) {
                Object expected = expectation.get("lte");
                double actualNum = convertNumeric(value);
                double expectedNum = convertNumeric(expected);
                assertTrue(actualNum <= expectedNum,
                        String.format("Expected %f <= %f", actualNum, expectedNum));
            }

            if (expectation.containsKey("contains")) {
                Object contains = expectation.get("contains");
                if (value instanceof String && contains instanceof String) {
                    assertTrue(((String) value).contains((String) contains),
                            String.format("Expected '%s' to contain '%s'", value, contains));
                } else if (value instanceof List && contains instanceof String) {
                    // List contains a string
                    @SuppressWarnings("unchecked")
                    List<Object> valueList = (List<Object>) value;
                    boolean found = valueList.stream()
                            .anyMatch(item -> item.toString().contains((String) contains));
                    assertTrue(found,
                            String.format("Expected %s to contain '%s'", value, contains));
                } else if (value instanceof List && contains instanceof List) {
                    @SuppressWarnings("unchecked")
                    List<Object> valueList = (List<Object>) value;
                    @SuppressWarnings("unchecked")
                    List<Object> containsList = (List<Object>) contains;
                    boolean allContained = containsList.stream()
                            .allMatch(valueList::contains);
                    assertTrue(allContained,
                            String.format("Expected %s to contain all of %s", value, contains));
                } else {
                    fail(String.format("Unsupported contains expectation for path '%s'", path));
                }
            }
        }

        private static Object fetchMetadataValue(Map<String, Object> metadata, String path) {
            if (metadata == null) {
                return null;
            }
            Object direct = lookupMetadataPath(metadata, path);
            if (direct != null) {
                return direct;
            }
            Object formatObj = metadata.get("format");
            if (formatObj instanceof Map<?, ?>) {
                @SuppressWarnings("unchecked")
                Map<String, Object> format = (Map<String, Object>) formatObj;
                return lookupMetadataPath(format, path);
            }
            return null;
        }

        private static Object lookupMetadataPath(Map<String, Object> metadata, String path) {
            Object current = metadata;
            for (String segment : path.split("\\.")) {
                if (!(current instanceof Map)) {
                    return null;
                }
                @SuppressWarnings("unchecked")
                Map<String, Object> map = (Map<String, Object>) current;
                current = map.get(segment);
            }
            return current;
        }

        private static boolean valuesEqual(Object lhs, Object rhs) {
            if (lhs == null && rhs == null) {
                return true;
            }
            if (lhs == null || rhs == null) {
                return false;
            }
            if (lhs instanceof String && rhs instanceof String) {
                return lhs.equals(rhs);
            }
            if (isNumericLike(lhs) && isNumericLike(rhs)) {
                return convertNumeric(lhs) == convertNumeric(rhs);
            }
            return lhs.equals(rhs);
        }

        private static boolean isNumericLike(Object value) {
            return value instanceof Number;
        }

        private static double convertNumeric(Object value) {
            if (value instanceof Number) {
                return ((Number) value).doubleValue();
            }
            throw new IllegalArgumentException("Cannot convert to numeric: " + value);
        }

        public static void assertChunks(
                ExtractionResult result,
                Integer minCount,
                Integer maxCount,
                Boolean eachHasContent,
                Boolean eachHasEmbedding,
                Boolean eachHasHeadingContext,
                Boolean eachHasChunkType,
                Boolean contentStartsWithHeading
        ) {
            var chunks = result.getChunks();
            int count = chunks != null ? chunks.size() : 0;
            if (minCount != null) {
                assertTrue(count >= minCount,
                        String.format("Expected chunk count >= %d, got %d", minCount, count));
            }
            if (maxCount != null) {
                assertTrue(count <= maxCount,
                        String.format("Expected chunk count <= %d, got %d", maxCount, count));
            }
            if (chunks != null && eachHasContent != null && eachHasContent) {
                for (var chunk : chunks) {
                    String content = chunk.getContent();
                    assertTrue(content != null && !content.isEmpty(),
                            "Expected each chunk to have content");
                }
            }
            if (chunks != null && eachHasEmbedding != null && eachHasEmbedding) {
                for (var chunk : chunks) {
                    assertNotNull(chunk.getEmbedding(),
                            "Expected each chunk to have an embedding");
                }
            }
            if (chunks != null && eachHasHeadingContext != null && eachHasHeadingContext) {
                for (var chunk : chunks) {
                    assertNotNull(chunk.getMetadata().getHeadingContext(),
                            "Expected each chunk to have heading_context");
                }
            }
            if (chunks != null && eachHasHeadingContext != null && !eachHasHeadingContext) {
                for (var chunk : chunks) {
                    assertTrue(chunk.getMetadata().getHeadingContext().isEmpty(),
                            "Expected each chunk to have no heading_context");
                }
            }
            if (chunks != null && eachHasChunkType != null && eachHasChunkType) {
                for (var chunk : chunks) {
                    String type = chunk.getChunkType();
                    assertTrue(type != null && !"unknown".equals(type),
                            "Expected each chunk to have a specific chunk_type");
                }
            }
            if (chunks != null && contentStartsWithHeading != null && contentStartsWithHeading) {
                String headingPrefix = String.valueOf((char) 35);
                for (var chunk : chunks) {
                    if (chunk.getMetadata().getHeadingContext().isEmpty()) continue;
                    String content = chunk.getContent();
                    assertTrue(content != null && content.startsWith(headingPrefix),
                            "Expected each chunk content to start with a heading");
                }
            }
        }

        public static void assertImages(
                ExtractionResult result,
                Integer minCount,
                Integer maxCount,
                List<String> formatsInclude
        ) {
            var images = result.getImages();
            int count = images != null ? images.size() : 0;
            if (minCount != null) {
                assertTrue(count >= minCount,
                        String.format("Expected image count >= %d, got %d", minCount, count));
            }
            if (maxCount != null) {
                assertTrue(count <= maxCount,
                        String.format("Expected image count <= %d, got %d", maxCount, count));
            }
            if (images != null && formatsInclude != null && !formatsInclude.isEmpty()) {
                var formats = images.stream()
                        .map(img -> img.getFormat())
                        .filter(f -> f != null)
                        .toList();
                for (String expected : formatsInclude) {
                    boolean found = formats.stream()
                            .anyMatch(f -> f.toLowerCase().contains(expected.toLowerCase()));
                    assertTrue(found,
                            String.format("Expected image formats to include '%s', got %s", expected, formats));
                }
            }
        }

        public static void assertPages(
                ExtractionResult result,
                Integer minCount,
                Integer exactCount,
                Boolean hasLayoutRegions,
                List<String> layoutClassesInclude
        ) {
            var pages = result.getPages();
            int count = pages != null ? pages.size() : 0;
            if (minCount != null) {
                assertTrue(count >= minCount,
                        String.format("Expected page count >= %d, got %d", minCount, count));
            }
            if (exactCount != null) {
                assertTrue(count == exactCount,
                        String.format("Expected exactly %d pages, got %d", exactCount, count));
            }
            if (pages != null) {
                for (var page : pages) {
                    // isBlank should be accessible (Optional<Boolean>)
                    var isBlank = page.getIsBlank();
                    assertTrue(isBlank != null, "getIsBlank() should return non-null Optional");
                }
            }

            if (Boolean.TRUE.equals(hasLayoutRegions)) {
                boolean foundLayoutRegions = false;
                if (pages != null) {
                    for (var page : pages) {
                        var layoutRegions = page.getLayoutRegions();
                        if (layoutRegions != null && !layoutRegions.isEmpty()) {
                            foundLayoutRegions = true;
                            break;
                        }
                    }
                }
                assertTrue(foundLayoutRegions, "Expected at least one page to have layout_regions populated");
            }

            if (layoutClassesInclude != null && !layoutClassesInclude.isEmpty()) {
                var allClasses = new java.util.HashSet<String>();
                if (pages != null) {
                    for (var page : pages) {
                        var layoutRegions = page.getLayoutRegions();
                        if (layoutRegions != null) {
                            for (var region : layoutRegions) {
                                allClasses.add(region.getClassName());
                            }
                        }
                    }
                }
                for (var expectedClass : layoutClassesInclude) {
                    assertTrue(allClasses.contains(expectedClass),
                            String.format("Expected layout class '%s' not found in %s", expectedClass, allClasses));
                }
            }
        }

        public static void assertElements(
                ExtractionResult result,
                Integer minCount,
                List<String> typesInclude
        ) {
            var elements = result.getElements();
            int count = elements != null ? elements.size() : 0;
            if (minCount != null) {
                assertTrue(count >= minCount,
                        String.format("Expected element count >= %d, got %d", minCount, count));
            }
            if (elements != null && typesInclude != null && !typesInclude.isEmpty()) {
                var types = elements.stream()
                        .map(el -> el.getElementType().wireValue())
                        .filter(t -> t != null)
                        .toList();
                for (String expected : typesInclude) {
                    boolean found = types.stream()
                            .anyMatch(t -> t.toLowerCase().contains(expected.toLowerCase()));
                    assertTrue(found,
                            String.format("Expected element types to include '%s', got %s", expected, types));
                }
            }
        }

        public static void assertOcrElements(
                ExtractionResult result,
                boolean hasElements,
                Boolean hasGeometry,
                Boolean hasConfidence,
                Integer minCount
        ) {
            var ocrElements = result.getOcrElements();
            if (hasElements) {
                assertTrue(!ocrElements.isEmpty(), "Expected OCR elements, but none found");
            }
            if (hasGeometry != null && hasGeometry) {
                for (int i = 0; i < ocrElements.size(); i++) {
                    assertNotNull(ocrElements.get(i).getGeometry(),
                            String.format("OCR element %d expected to have geometry", i));
                }
            }
            if (hasConfidence != null && hasConfidence) {
                for (int i = 0; i < ocrElements.size(); i++) {
                    assertNotNull(ocrElements.get(i).getConfidence(),
                            String.format("OCR element %d expected to have confidence score", i));
                }
            }
            if (minCount != null) {
                assertTrue(ocrElements.size() >= minCount,
                        String.format("Expected at least %d OCR elements, found %d", minCount, ocrElements.size()));
            }
        }

        public static void assertDocument(
                ExtractionResult result,
                boolean hasDocument,
                Integer minNodeCount,
                List<String> nodeTypesInclude,
                Boolean hasGroups
        ) {
            var document = result.getDocumentStructure().orElse(null);
            if (hasDocument) {
                assertNotNull(document, "Expected document but got null");
                var nodes = document.getNodes();
                assertNotNull(nodes, "Expected document nodes but got null");
                if (minNodeCount != null) {
                    assertTrue(nodes.size() >= minNodeCount,
                            String.format("Expected at least %d nodes, found %d", minNodeCount, nodes.size()));
                }
                if (nodeTypesInclude != null && !nodeTypesInclude.isEmpty()) {
                    var types = nodes.stream()
                            .map(n -> n.getContent().getNodeType())
                            .filter(t -> t != null)
                            .toList();
                    for (String expected : nodeTypesInclude) {
                        boolean found = types.stream()
                                .anyMatch(t -> t.toLowerCase().equals(expected.toLowerCase()));
                        assertTrue(found,
                                String.format("Expected node type '%s' not found in %s", expected, types));
                    }
                }
                if (hasGroups != null) {
                    boolean hasGroupNodes = nodes.stream()
                            .anyMatch(n -> "group".equals(n.getContent().getNodeType()));
                    assertTrue(hasGroupNodes == hasGroups,
                            String.format("Expected hasGroups=%b but got %b", hasGroups, hasGroupNodes));
                }
            } else {
                assertTrue(document == null,
                        String.format("Expected document to be null but got %s", document));
            }
        }

        public static void assertKeywords(
                ExtractionResult result,
                Boolean hasKeywords,
                Integer minCount,
                Integer maxCount
        ) {
            var keywordsOpt = result.getExtractedKeywords();
            var keywords = keywordsOpt.orElse(null);
            int count = keywords != null ? keywords.size() : 0;

            if (hasKeywords != null && hasKeywords) {
                assertTrue(keywords != null && !keywords.isEmpty(), "Expected keywords to be present");
            }
            if (hasKeywords != null && !hasKeywords) {
                assertTrue(keywords == null || keywords.isEmpty(),
                        String.format("Expected no keywords but found %d", count));
            }

            if (minCount != null) {
                assertTrue(count >= minCount,
                        String.format("Expected keyword count >= %d, got %d", minCount, count));
            }
            if (maxCount != null) {
                assertTrue(count <= maxCount,
                        String.format("Expected keyword count <= %d, got %d", maxCount, count));
            }
        }

        public static void assertContentNotEmpty(ExtractionResult result) {
            String content = result.getContent();
            assertTrue(content != null && !content.isEmpty(),
                    "Expected content to be non-empty");
        }

        public static void assertTableBoundingBoxes(ExtractionResult result) {
            var tables = result.getTables();
            if (tables != null) {
                for (int i = 0; i < tables.size(); i++) {
                    assertNotNull(tables.get(i).boundingBox(),
                            String.format("Table %d expected to have bounding box", i));
                }
            }
        }

        public static void assertTableContentContainsAny(ExtractionResult result, List<String> snippets) {
            if (snippets.isEmpty()) return;
            var tables = result.getTables();
            StringBuilder allContent = new StringBuilder();
            if (tables != null) {
                for (var table : tables) {
                    allContent.append(table.markdown() != null ? table.markdown().toLowerCase() : "").append(" ");
                }
            }
            String combined = allContent.toString();
            boolean found = snippets.stream()
                    .anyMatch(snippet -> combined.contains(snippet.toLowerCase()));
            assertTrue(found,
                    String.format("Expected table content to contain any of %s", snippets));
        }

        public static void assertImageBoundingBoxes(ExtractionResult result) {
            var images = result.getImages();
            if (images != null) {
                for (int i = 0; i < images.size(); i++) {
                    assertNotNull(images.get(i).getBoundingBox(),
                            String.format("Image %d expected to have bounding box", i));
                }
            }
        }

        public static void assertQualityScore(ExtractionResult result, Boolean hasScore, Double minScore, Double maxScore) {
            if (hasScore != null && hasScore) {
                assertNotNull(result.getQualityScore(), "Expected quality score to be present");
            }
            Double score = result.getQualityScore().orElse(null);
            if (minScore != null && score != null) {
                assertTrue(score >= minScore,
                        String.format("Expected quality score >= %f, got %f", minScore, score));
            }
            if (maxScore != null && score != null) {
                assertTrue(score <= maxScore,
                        String.format("Expected quality score <= %f, got %f", maxScore, score));
            }
        }

        public static void assertProcessingWarnings(ExtractionResult result, Integer maxCount, Boolean isEmpty) {
            var warnings = result.getProcessingWarnings().orElse(null);
            int count = warnings != null ? warnings.size() : 0;
            if (isEmpty != null && isEmpty) {
                assertTrue(count == 0,
                        String.format("Expected processing warnings to be empty, got %d", count));
            }
            if (maxCount != null) {
                assertTrue(count <= maxCount,
                        String.format("Expected at most %d processing warnings, got %d", maxCount, count));
            }
        }

        public static void assertLlmUsage(ExtractionResult result, Integer maxCount, Boolean isEmpty) {
            var usage = result.getLlmUsage().orElse(null);
            int count = usage != null ? usage.size() : 0;
            if (isEmpty != null && isEmpty) {
                assertTrue(count == 0,
                        String.format("Expected llm usage to be empty, got %d", count));
            }
            if (maxCount != null) {
                assertTrue(count <= maxCount,
                        String.format("Expected at most %d llm usage entries, got %d", maxCount, count));
            }
        }

        public static void assertDjotContent(ExtractionResult result, Boolean hasContent, Integer minBlocks) {
            var djotContent = result.getDjotContent().orElse(null);
            if (hasContent != null && hasContent) {
                assertTrue(djotContent != null && !djotContent.getPlainText().isEmpty(),
                        "Expected djot content to be present");
            }
            if (minBlocks != null && djotContent != null) {
                int blockCount = djotContent.getBlocks() != null ? djotContent.getBlocks().size() : 0;
                assertTrue(blockCount >= minBlocks,
                        String.format("Expected at least %d djot blocks, got %d", minBlocks, blockCount));
            }
        }

        public static void assertAnnotations(ExtractionResult result, Boolean hasAnnotations, Integer minCount) {
            var annotations = result.getAnnotations().orElse(null);
            if (hasAnnotations != null && hasAnnotations) {
                assertTrue(annotations != null && !annotations.isEmpty(),
                        "Expected annotations to be present and non-empty");
            }
            if (annotations != null && minCount != null) {
                assertTrue(annotations.size() >= minCount,
                        String.format("Expected at least %d annotations, got %d", minCount, annotations.size()));
            }
        }
    }

    public static void assertIsPng(byte[] data) {
        assertNotNull(data, "PNG data should not be null");
        assertTrue(data.length >= 4, String.format("Data too short for PNG: %d bytes", data.length));
        assertTrue(data[0] == (byte) 0x89 && data[1] == (byte) 0x50
                && data[2] == (byte) 0x4E && data[3] == (byte) 0x47,
                String.format("Missing PNG magic bytes, got: [%02x, %02x, %02x, %02x]",
                        data[0], data[1], data[2], data[3]));
    }

    public static void assertMinByteLength(byte[] data, int minLength) {
        assertNotNull(data, "Data should not be null");
        assertTrue(data.length >= minLength,
                String.format("Expected at least %d bytes, got %d", minLength, data.length));
    }

    public static void assertStructuredOutput(ExtractionResult result, Boolean hasOutput, Boolean validatesSchema, String[] fieldExists) {
        var output = result.getStructuredOutput().orElse(null);
        if (hasOutput != null && hasOutput) {
            assertNotNull(output, "Expected structured_output to be present");
        }
        if (hasOutput != null && !hasOutput) {
            assertNull(output, "Expected structured_output to be absent");
        }
        if (validatesSchema != null && validatesSchema) {
            assertNotNull(output, "structured_output required for validates_schema");
        }
        if (fieldExists != null) {
            assertNotNull(output, "structured_output required for field_exists");
            assertTrue(output instanceof java.util.Map, "structured_output must be a Map for field_exists");
            @SuppressWarnings("unchecked")
            var outputMap = (java.util.Map<String, Object>) output;
            for (String field : fieldExists) {
                assertTrue(outputMap.containsKey(field),
                        String.format("Expected structured_output to contain '%s'", field));
            }
        }
    }
}
"#;

fn build_pom_template(mode: &GenerationMode) -> String {
    let kreuzberg_dep = match mode {
        GenerationMode::Published { version: _ } => {
            "        <dependency>\n\
                 \x20           <groupId>dev.kreuzberg</groupId>\n\
                 \x20           <artifactId>kreuzberg</artifactId>\n\
                 \x20           <version>4.9.1</version>\n\
                 \x20       </dependency>".to_string()
        }
        GenerationMode::Local => {
            "        <dependency>\n\
             \x20           <groupId>dev.kreuzberg</groupId>\n\
             \x20           <artifactId>kreuzberg</artifactId>\n\
             \x20           <version>4.9.1</version>\n\
             \x20           <scope>system</scope>\n\
             \x20           <systemPath>${project.basedir}/../../packages/java/target/kreuzberg-4.9.1.jar</systemPath>\n\
             \x20       </dependency>"
                .to_string()
        }
    };

    let _kreuzberg_version = match mode {
        GenerationMode::Published { version } => version.as_str(),
        GenerationMode::Local => "4.9.1",
    };

    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<project xmlns="http://maven.apache.org/POM/4.0.0"
         xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"
         xsi:schemaLocation="http://maven.apache.org/POM/4.0.0 http://maven.apache.org/xsd/maven-4.0.0.xsd">
    <modelVersion>4.0.0</modelVersion>

    <groupId>com.kreuzberg</groupId>
    <artifactId>kreuzberg-e2e</artifactId>
    <version>1.0-SNAPSHOT</version>

    <properties>
        <maven.compiler.source>25</maven.compiler.source>
        <maven.compiler.target>25</maven.compiler.target>
        <maven.compiler.release>25</maven.compiler.release>
        <project.build.sourceEncoding>UTF-8</project.build.sourceEncoding>
        <junit.version>5.11.3</junit.version>
        <jackson.version>2.18.2</jackson.version>
        <kreuzberg.version>4.9.1</kreuzberg.version>
    </properties>

    <dependencies>
{kreuzberg_dep}

        <dependency>
            <groupId>org.junit.jupiter</groupId>
            <artifactId>junit-jupiter</artifactId>
            <version>${{junit.version}}</version>
            <scope>test</scope>
        </dependency>

        <dependency>
            <groupId>com.fasterxml.jackson.core</groupId>
            <artifactId>jackson-databind</artifactId>
            <version>${{jackson.version}}</version>
        </dependency>

        <dependency>
            <groupId>com.fasterxml.jackson.module</groupId>
            <artifactId>jackson-module-parameter-names</artifactId>
            <version>${{jackson.version}}</version>
        </dependency>
    </dependencies>

    <build>
        <plugins>
            <plugin>
                <groupId>org.apache.maven.plugins</groupId>
                <artifactId>maven-compiler-plugin</artifactId>
                <version>3.14.1</version>
                <configuration>
                    <release>25</release>
                    <compilerArgs>
                        <arg>--enable-preview</arg>
                    </compilerArgs>
                </configuration>
            </plugin>
            <plugin>
                <groupId>org.apache.maven.plugins</groupId>
                <artifactId>maven-surefire-plugin</artifactId>
                <version>3.5.4</version>
                <configuration>
                    <argLine>--enable-native-access=ALL-UNNAMED --enable-preview -Djava.library.path=${{project.basedir}}/../../target/release</argLine>
                    <forkedProcessTimeoutInSeconds>300</forkedProcessTimeoutInSeconds>
                </configuration>
            </plugin>
        </plugins>
    </build>
</project>
"#
    )
}

pub fn generate(fixtures: &[Fixture], output_root: &Utf8Path, mode: &GenerationMode) -> Result<()> {
    let java_root = output_root.join("java");
    let src_test = java_root.join("src/test/java/com/kreuzberg/e2e");

    fs::create_dir_all(&src_test).context("Failed to create Java test directory")?;

    write_helpers(&src_test)?;
    write_package_info(&src_test)?;
    write_pom(&java_root, mode)?;
    clean_test_files(&src_test)?;
    generate_embed_tests(fixtures, &src_test)?;

    let doc_fixtures: Vec<_> = fixtures.iter().filter(|f| f.is_document_extraction()).collect();

    let api_fixtures: Vec<_> = fixtures.iter().filter(|f| f.is_plugin_api()).collect();

    let mut grouped = doc_fixtures
        .into_iter()
        .into_group_map_by(|fixture| fixture.category().to_string())
        .into_iter()
        .collect::<Vec<_>>();
    grouped.sort_by(|a, b| a.0.cmp(&b.0));

    for (category, mut fixtures) in grouped {
        fixtures.sort_by(|a, b| a.id.cmp(&b.id));
        let class_name = to_java_class_name(&category) + "Test";
        let content = render_category(&category, &class_name, &fixtures)?;
        let path = src_test.join(format!("{}.java", class_name));
        fs::write(&path, content).with_context(|| format!("Writing {}", path))?;
    }

    if !api_fixtures.is_empty() {
        generate_plugin_api_tests(&api_fixtures, &src_test)?;
    }

    let render_fixtures: Vec<_> = fixtures.iter().filter(|f| f.is_render()).collect();
    if !render_fixtures.is_empty() {
        let mut sorted = render_fixtures;
        sorted.sort_by(|a, b| a.id.cmp(&b.id));
        let content = render_render_category(&sorted)?;
        let path = src_test.join("RenderTest.java");
        fs::write(&path, content).with_context(|| format!("Writing {}", path))?;
    }

    Ok(())
}

fn clean_test_files(src_test: &Utf8Path) -> Result<()> {
    if !src_test.exists() {
        return Ok(());
    }

    for entry in fs::read_dir(src_test.as_std_path())? {
        let entry = entry?;
        let path = entry.path();
        if path
            .file_name()
            .is_some_and(|name| name == "E2EHelpers.java" || name == "package-info.java")
        {
            continue;
        }
        if path.extension().is_some_and(|ext| ext == "java") {
            fs::remove_file(path)?;
        }
    }

    Ok(())
}

fn write_helpers(src_test: &Utf8Path) -> Result<()> {
    let helpers_path = src_test.join("E2EHelpers.java");
    fs::write(&helpers_path, JAVA_HELPERS_TEMPLATE).context("Failed to write Java helpers")
}

fn write_package_info(src_test: &Utf8Path) -> Result<()> {
    let package_info_path = src_test.join("package-info.java");
    let content = r#"/**
 * E2E test utilities and generated test classes for Kreuzberg.
 *
 * <p>This package contains auto-generated test classes organized by fixture category.
 * Tests use JUnit 5 and validate document extraction across multiple formats.
 *
 * @since 4.0.0
 */
package com.kreuzberg.e2e;
"#;
    fs::write(&package_info_path, content).context("Failed to write package-info.java")
}

fn write_pom(java_root: &Utf8Path, mode: &GenerationMode) -> Result<()> {
    let pom_path = java_root.join("pom.xml");
    let content = build_pom_template(mode);
    fs::write(&pom_path, content).context("Failed to write pom.xml")
}

fn render_category(category: &str, class_name: &str, fixtures: &[&Fixture]) -> Result<String> {
    let mut buffer = String::new();
    writeln!(buffer, "package com.kreuzberg.e2e;")?;
    writeln!(buffer)?;
    writeln!(buffer, "// CHECKSTYLE.OFF: UnusedImports - generated code")?;
    writeln!(buffer, "// CHECKSTYLE.OFF: LineLength - generated code")?;
    writeln!(buffer, "import com.fasterxml.jackson.databind.JsonNode;")?;
    writeln!(buffer, "import com.fasterxml.jackson.databind.ObjectMapper;")?;
    writeln!(buffer, "import dev.kreuzberg.BytesWithMime;")?;
    writeln!(buffer, "import dev.kreuzberg.ExtractionResult;")?;
    writeln!(buffer, "import dev.kreuzberg.Kreuzberg;")?;
    writeln!(buffer, "import dev.kreuzberg.config.ExtractionConfig;")?;
    writeln!(buffer, "import org.junit.jupiter.api.Test;")?;
    writeln!(buffer)?;
    writeln!(buffer, "import java.nio.file.Files;")?;
    writeln!(buffer, "import java.nio.file.Path;")?;
    writeln!(buffer, "import java.util.Arrays;")?;
    writeln!(buffer, "import java.util.Collections;")?;
    writeln!(buffer, "import java.util.List;")?;
    writeln!(buffer, "import java.util.Map;")?;
    writeln!(buffer)?;
    writeln!(buffer, "import static org.junit.jupiter.api.Assertions.assertTrue;")?;
    writeln!(buffer, "// CHECKSTYLE.ON: UnusedImports")?;
    writeln!(buffer, "// CHECKSTYLE.ON: LineLength")?;
    writeln!(buffer)?;
    writeln!(buffer, "// Code generated by kreuzberg-e2e-generator. DO NOT EDIT.")?;
    writeln!(
        buffer,
        "// To regenerate: cargo run -p kreuzberg-e2e-generator -- generate --lang java"
    )?;
    writeln!(buffer)?;
    writeln!(buffer, "/** Tests for {} fixtures. */", category)?;
    writeln!(buffer, "public class {} {{", class_name)?;
    writeln!(
        buffer,
        "    private static final ObjectMapper MAPPER = new ObjectMapper();"
    )?;
    writeln!(buffer)?;

    for fixture in fixtures {
        buffer.push_str(&render_test(fixture)?);
        writeln!(buffer)?;
    }

    writeln!(buffer, "}}")?;
    Ok(buffer)
}

fn render_test(fixture: &Fixture) -> Result<String> {
    let mut body = String::new();
    let extraction = fixture.extraction();
    let method = extraction.method;
    let input_type = extraction.input_type;

    writeln!(body, "    @Test")?;
    writeln!(
        body,
        "    public void {}() throws Exception {{",
        to_java_method_name(&fixture.id)
    )?;

    let config_expr = render_config_expression(&extraction.config)?;
    let config_var = match config_expr {
        Some(json) => {
            writeln!(
                body,
                "        JsonNode config = MAPPER.readTree({});",
                render_java_string(&json)
            )?;
            "config"
        }
        None => {
            writeln!(body, "        JsonNode config = null;")?;
            "config"
        }
    };

    let requirements = collect_requirements(fixture);
    let requirements_expr = render_string_list(&requirements);
    let notes_expr = render_optional_string(fixture.skip().notes.as_ref());
    let skip_flag = if fixture.skip().if_document_missing {
        "true"
    } else {
        "false"
    };

    // Skip if fixture requires features that may not be available
    let skip_directive = fixture.skip();
    let doc = fixture.document();
    let all_features: Vec<&str> = skip_directive
        .requires_feature
        .iter()
        .chain(doc.requires_external_tool.iter().filter(|t| *t == "paddle-ocr"))
        .map(|s| s.as_str())
        .collect();
    for feature in &all_features {
        writeln!(
            body,
            "        E2EHelpers.skipIfFeatureUnavailable({});",
            render_java_string(feature)
        )?;
    }

    let skip_platforms = &fixture.skip().skip_on_platform;
    if !skip_platforms.is_empty() {
        let conditions: Vec<String> = skip_platforms
            .iter()
            .filter_map(|triple| rust_triple_to_java_condition(triple))
            .collect();
        if !conditions.is_empty() {
            let combined = conditions.join(" || ");
            writeln!(body, "        if ({combined}) {{")?;
            writeln!(
                body,
                "            org.junit.jupiter.api.Assumptions.assumeTrue(false, \"Skipping {}: not supported on this platform\");",
                fixture.id
            )?;
            writeln!(body, "            return;")?;
            writeln!(body, "        }}")?;
        }
    }

    // Generate different code based on extraction method and input type
    match (method, input_type) {
        (ExtractionMethod::Sync, InputType::File) => {
            // Existing pattern: sync file extraction via runFixture helper
            writeln!(body, "        E2EHelpers.runFixture(")?;
            writeln!(body, "            {},", render_java_string(&fixture.id))?;
            writeln!(body, "            {},", render_java_string(&fixture.document().path))?;
            writeln!(body, "            {},", config_var)?;
            writeln!(body, "            {},", requirements_expr)?;
            writeln!(body, "            {},", notes_expr)?;
            writeln!(body, "            {},", skip_flag)?;
            writeln!(body, "            result -> {{")?;

            let assertions = render_assertions(&fixture.assertions());
            if !assertions.is_empty() {
                body.push_str(&assertions);
            }

            writeln!(body, "            }}")?;
            writeln!(body, "        );")?;
        }
        (ExtractionMethod::Sync, InputType::Bytes) => {
            // Sync bytes extraction: read file as bytes and call extractBytes
            // Note: extractBytes requires mimeType, so we detect it from the file bytes
            writeln!(
                body,
                "        Path documentPath = E2EHelpers.resolveDocument({});",
                render_java_string(&fixture.document().path)
            )?;
            writeln!(body)?;
            if skip_flag == "true" {
                writeln!(body, "        if (!Files.exists(documentPath)) {{")?;
                writeln!(
                    body,
                    "            String msg = String.format(\"Skipping {}: missing document at %s\", documentPath);",
                    fixture.id
                )?;
                writeln!(body, "            System.err.println(msg);")?;
                writeln!(
                    body,
                    "            org.junit.jupiter.api.Assumptions.assumeTrue(false, msg);"
                )?;
                writeln!(body, "            return;")?;
                writeln!(body, "        }}")?;
            }
            writeln!(body)?;
            writeln!(body, "        byte[] documentBytes = Files.readAllBytes(documentPath);")?;
            writeln!(
                body,
                "        String mimeType = Kreuzberg.detectMimeType(documentBytes);"
            )?;
            writeln!(
                body,
                "        ExtractionConfig extractionConfig = E2EHelpers.buildConfig({});",
                config_var
            )?;
            writeln!(body, "        ExtractionResult result;")?;
            writeln!(body, "        try {{")?;
            writeln!(
                body,
                "            result = Kreuzberg.extractBytes(documentBytes, mimeType, extractionConfig);"
            )?;
            writeln!(body, "        }} catch (Exception e) {{")?;
            writeln!(
                body,
                "            String skipReason = E2EHelpers.skipReasonFor(e, {}, {}, {});",
                render_java_string(&fixture.id),
                requirements_expr,
                notes_expr
            )?;
            writeln!(body, "            if (skipReason != null) {{")?;
            writeln!(
                body,
                "                org.junit.jupiter.api.Assumptions.assumeTrue(false, skipReason);"
            )?;
            writeln!(body, "                return;")?;
            writeln!(body, "            }}")?;
            writeln!(body, "            throw e;")?;
            writeln!(body, "        }}")?;
            writeln!(body)?;

            let assertions = render_assertions(&fixture.assertions());
            if !assertions.is_empty() {
                // Remove the leading indentation since we're not inside a callback
                for line in assertions.lines() {
                    let trimmed = line.trim_start();
                    if !trimmed.is_empty() {
                        writeln!(body, "        {}", trimmed)?;
                    }
                }
            }
        }
        (ExtractionMethod::Async, InputType::File) => {
            // Async file extraction using CompletableFuture
            writeln!(
                body,
                "        Path documentPath = E2EHelpers.resolveDocument({});",
                render_java_string(&fixture.document().path)
            )?;
            writeln!(body)?;
            if skip_flag == "true" {
                writeln!(body, "        if (!Files.exists(documentPath)) {{")?;
                writeln!(
                    body,
                    "            String msg = String.format(\"Skipping {}: missing document at %s\", documentPath);",
                    fixture.id
                )?;
                writeln!(body, "            System.err.println(msg);")?;
                writeln!(
                    body,
                    "            org.junit.jupiter.api.Assumptions.assumeTrue(false, msg);"
                )?;
                writeln!(body, "            return;")?;
                writeln!(body, "        }}")?;
            }
            writeln!(body)?;
            writeln!(
                body,
                "        ExtractionConfig extractionConfig = E2EHelpers.buildConfig({});",
                config_var
            )?;
            writeln!(
                body,
                "        java.util.concurrent.CompletableFuture<ExtractionResult> future = Kreuzberg.extractFileAsync(documentPath, extractionConfig);"
            )?;
            writeln!(body, "        ExtractionResult result;")?;
            writeln!(body, "        try {{")?;
            writeln!(body, "            result = future.get();")?;
            writeln!(body, "        }} catch (java.util.concurrent.ExecutionException e) {{")?;
            writeln!(
                body,
                "            Throwable cause = e.getCause() != null ? e.getCause() : e;"
            )?;
            writeln!(body, "            if (cause instanceof Exception) {{")?;
            writeln!(
                body,
                "                String skipReason = E2EHelpers.skipReasonFor((Exception) cause, {}, {}, {});",
                render_java_string(&fixture.id),
                requirements_expr,
                notes_expr
            )?;
            writeln!(body, "                if (skipReason != null) {{")?;
            writeln!(
                body,
                "                    org.junit.jupiter.api.Assumptions.assumeTrue(false, skipReason);"
            )?;
            writeln!(body, "                    return;")?;
            writeln!(body, "                }}")?;
            writeln!(body, "            }}")?;
            writeln!(body, "            throw e;")?;
            writeln!(body, "        }}")?;
            writeln!(body)?;

            let assertions = render_assertions(&fixture.assertions());
            if !assertions.is_empty() {
                for line in assertions.lines() {
                    let trimmed = line.trim_start();
                    if !trimmed.is_empty() {
                        writeln!(body, "        {}", trimmed)?;
                    }
                }
            }
        }
        (ExtractionMethod::Async, InputType::Bytes) => {
            // Async bytes extraction using CompletableFuture
            // Note: extractBytesAsync requires mimeType, so we detect it from the file first
            writeln!(
                body,
                "        Path documentPath = E2EHelpers.resolveDocument({});",
                render_java_string(&fixture.document().path)
            )?;
            writeln!(body)?;
            if skip_flag == "true" {
                writeln!(body, "        if (!Files.exists(documentPath)) {{")?;
                writeln!(
                    body,
                    "            String msg = String.format(\"Skipping {}: missing document at %s\", documentPath);",
                    fixture.id
                )?;
                writeln!(body, "            System.err.println(msg);")?;
                writeln!(
                    body,
                    "            org.junit.jupiter.api.Assumptions.assumeTrue(false, msg);"
                )?;
                writeln!(body, "            return;")?;
                writeln!(body, "        }}")?;
            }
            writeln!(body)?;
            writeln!(body, "        byte[] documentBytes = Files.readAllBytes(documentPath);")?;
            writeln!(
                body,
                "        String mimeType = Kreuzberg.detectMimeType(documentBytes);"
            )?;
            writeln!(
                body,
                "        ExtractionConfig extractionConfig = E2EHelpers.buildConfig({});",
                config_var
            )?;
            writeln!(
                body,
                "        java.util.concurrent.CompletableFuture<ExtractionResult> future = Kreuzberg.extractBytesAsync(documentBytes, mimeType, extractionConfig);"
            )?;
            writeln!(body, "        ExtractionResult result;")?;
            writeln!(body, "        try {{")?;
            writeln!(body, "            result = future.get();")?;
            writeln!(body, "        }} catch (java.util.concurrent.ExecutionException e) {{")?;
            writeln!(
                body,
                "            Throwable cause = e.getCause() != null ? e.getCause() : e;"
            )?;
            writeln!(body, "            if (cause instanceof Exception) {{")?;
            writeln!(
                body,
                "                String skipReason = E2EHelpers.skipReasonFor((Exception) cause, {}, {}, {});",
                render_java_string(&fixture.id),
                requirements_expr,
                notes_expr
            )?;
            writeln!(body, "                if (skipReason != null) {{")?;
            writeln!(
                body,
                "                    org.junit.jupiter.api.Assumptions.assumeTrue(false, skipReason);"
            )?;
            writeln!(body, "                    return;")?;
            writeln!(body, "                }}")?;
            writeln!(body, "            }}")?;
            writeln!(body, "            throw e;")?;
            writeln!(body, "        }}")?;
            writeln!(body)?;

            let assertions = render_assertions(&fixture.assertions());
            if !assertions.is_empty() {
                for line in assertions.lines() {
                    let trimmed = line.trim_start();
                    if !trimmed.is_empty() {
                        writeln!(body, "        {}", trimmed)?;
                    }
                }
            }
        }
        (ExtractionMethod::BatchSync, InputType::File) => {
            // Batch sync file extraction
            // Note: batchExtractFiles takes List<String> (paths as strings), not List<Path>
            writeln!(
                body,
                "        Path documentPath = E2EHelpers.resolveDocument({});",
                render_java_string(&fixture.document().path)
            )?;
            writeln!(body)?;
            if skip_flag == "true" {
                writeln!(body, "        if (!Files.exists(documentPath)) {{")?;
                writeln!(
                    body,
                    "            String msg = String.format(\"Skipping {}: missing document at %s\", documentPath);",
                    fixture.id
                )?;
                writeln!(body, "            System.err.println(msg);")?;
                writeln!(
                    body,
                    "            org.junit.jupiter.api.Assumptions.assumeTrue(false, msg);"
                )?;
                writeln!(body, "            return;")?;
                writeln!(body, "        }}")?;
            }
            writeln!(body)?;
            writeln!(
                body,
                "        ExtractionConfig extractionConfig = E2EHelpers.buildConfig({});",
                config_var
            )?;
            writeln!(
                body,
                "        List<String> paths = Arrays.asList(documentPath.toString());"
            )?;
            writeln!(body, "        List<ExtractionResult> results;")?;
            writeln!(body, "        try {{")?;
            writeln!(
                body,
                "            results = Kreuzberg.batchExtractFiles(paths, extractionConfig);"
            )?;
            writeln!(body, "        }} catch (Exception e) {{")?;
            writeln!(
                body,
                "            String skipReason = E2EHelpers.skipReasonFor(e, {}, {}, {});",
                render_java_string(&fixture.id),
                requirements_expr,
                notes_expr
            )?;
            writeln!(body, "            if (skipReason != null) {{")?;
            writeln!(
                body,
                "                org.junit.jupiter.api.Assumptions.assumeTrue(false, skipReason);"
            )?;
            writeln!(body, "                return;")?;
            writeln!(body, "            }}")?;
            writeln!(body, "            throw e;")?;
            writeln!(body, "        }}")?;
            writeln!(body)?;
            writeln!(
                body,
                "        assertTrue(results.size() == 1, \"Expected exactly 1 result from batch extraction\");"
            )?;
            writeln!(body, "        ExtractionResult result = results.get(0);")?;
            writeln!(body)?;

            let assertions = render_assertions(&fixture.assertions());
            if !assertions.is_empty() {
                for line in assertions.lines() {
                    let trimmed = line.trim_start();
                    if !trimmed.is_empty() {
                        writeln!(body, "        {}", trimmed)?;
                    }
                }
            }
        }
        (ExtractionMethod::BatchSync, InputType::Bytes) => {
            // Batch sync bytes extraction
            // Note: batchExtractBytes takes List<BytesWithMime>, so we detect mimeType
            writeln!(
                body,
                "        Path documentPath = E2EHelpers.resolveDocument({});",
                render_java_string(&fixture.document().path)
            )?;
            writeln!(body)?;
            if skip_flag == "true" {
                writeln!(body, "        if (!Files.exists(documentPath)) {{")?;
                writeln!(
                    body,
                    "            String msg = String.format(\"Skipping {}: missing document at %s\", documentPath);",
                    fixture.id
                )?;
                writeln!(body, "            System.err.println(msg);")?;
                writeln!(
                    body,
                    "            org.junit.jupiter.api.Assumptions.assumeTrue(false, msg);"
                )?;
                writeln!(body, "            return;")?;
                writeln!(body, "        }}")?;
            }
            writeln!(body)?;
            writeln!(body, "        byte[] documentBytes = Files.readAllBytes(documentPath);")?;
            writeln!(
                body,
                "        String mimeType = Kreuzberg.detectMimeType(documentBytes);"
            )?;
            writeln!(
                body,
                "        ExtractionConfig extractionConfig = E2EHelpers.buildConfig({});",
                config_var
            )?;
            writeln!(
                body,
                "        List<BytesWithMime> items = Arrays.asList(new BytesWithMime(documentBytes, mimeType));"
            )?;
            writeln!(body, "        List<ExtractionResult> results;")?;
            writeln!(body, "        try {{")?;
            writeln!(
                body,
                "            results = Kreuzberg.batchExtractBytes(items, extractionConfig);"
            )?;
            writeln!(body, "        }} catch (Exception e) {{")?;
            writeln!(
                body,
                "            String skipReason = E2EHelpers.skipReasonFor(e, {}, {}, {});",
                render_java_string(&fixture.id),
                requirements_expr,
                notes_expr
            )?;
            writeln!(body, "            if (skipReason != null) {{")?;
            writeln!(
                body,
                "                org.junit.jupiter.api.Assumptions.assumeTrue(false, skipReason);"
            )?;
            writeln!(body, "                return;")?;
            writeln!(body, "            }}")?;
            writeln!(body, "            throw e;")?;
            writeln!(body, "        }}")?;
            writeln!(body)?;
            writeln!(
                body,
                "        assertTrue(results.size() == 1, \"Expected exactly 1 result from batch extraction\");"
            )?;
            writeln!(body, "        ExtractionResult result = results.get(0);")?;
            writeln!(body)?;

            let assertions = render_assertions(&fixture.assertions());
            if !assertions.is_empty() {
                for line in assertions.lines() {
                    let trimmed = line.trim_start();
                    if !trimmed.is_empty() {
                        writeln!(body, "        {}", trimmed)?;
                    }
                }
            }
        }
        (ExtractionMethod::BatchAsync, InputType::File) => {
            // Batch async file extraction
            // Note: batchExtractFilesAsync takes List<String> (paths as strings), not List<Path>
            writeln!(
                body,
                "        Path documentPath = E2EHelpers.resolveDocument({});",
                render_java_string(&fixture.document().path)
            )?;
            writeln!(body)?;
            if skip_flag == "true" {
                writeln!(body, "        if (!Files.exists(documentPath)) {{")?;
                writeln!(
                    body,
                    "            String msg = String.format(\"Skipping {}: missing document at %s\", documentPath);",
                    fixture.id
                )?;
                writeln!(body, "            System.err.println(msg);")?;
                writeln!(
                    body,
                    "            org.junit.jupiter.api.Assumptions.assumeTrue(false, msg);"
                )?;
                writeln!(body, "            return;")?;
                writeln!(body, "        }}")?;
            }
            writeln!(body)?;
            writeln!(
                body,
                "        ExtractionConfig extractionConfig = E2EHelpers.buildConfig({});",
                config_var
            )?;
            writeln!(
                body,
                "        List<String> paths = Arrays.asList(documentPath.toString());"
            )?;
            writeln!(
                body,
                "        java.util.concurrent.CompletableFuture<List<ExtractionResult>> future = Kreuzberg.batchExtractFilesAsync(paths, extractionConfig);"
            )?;
            writeln!(body, "        List<ExtractionResult> results;")?;
            writeln!(body, "        try {{")?;
            writeln!(body, "            results = future.get();")?;
            writeln!(body, "        }} catch (java.util.concurrent.ExecutionException e) {{")?;
            writeln!(
                body,
                "            Throwable cause = e.getCause() != null ? e.getCause() : e;"
            )?;
            writeln!(body, "            if (cause instanceof Exception) {{")?;
            writeln!(
                body,
                "                String skipReason = E2EHelpers.skipReasonFor((Exception) cause, {}, {}, {});",
                render_java_string(&fixture.id),
                requirements_expr,
                notes_expr
            )?;
            writeln!(body, "                if (skipReason != null) {{")?;
            writeln!(
                body,
                "                    org.junit.jupiter.api.Assumptions.assumeTrue(false, skipReason);"
            )?;
            writeln!(body, "                    return;")?;
            writeln!(body, "                }}")?;
            writeln!(body, "            }}")?;
            writeln!(body, "            throw e;")?;
            writeln!(body, "        }}")?;
            writeln!(body)?;
            writeln!(
                body,
                "        assertTrue(results.size() == 1, \"Expected exactly 1 result from batch extraction\");"
            )?;
            writeln!(body, "        ExtractionResult result = results.get(0);")?;
            writeln!(body)?;

            let assertions = render_assertions(&fixture.assertions());
            if !assertions.is_empty() {
                for line in assertions.lines() {
                    let trimmed = line.trim_start();
                    if !trimmed.is_empty() {
                        writeln!(body, "        {}", trimmed)?;
                    }
                }
            }
        }
        (ExtractionMethod::BatchAsync, InputType::Bytes) => {
            // Batch async bytes extraction
            // Note: batchExtractBytesAsync takes List<BytesWithMime>, so we detect mimeType
            writeln!(
                body,
                "        Path documentPath = E2EHelpers.resolveDocument({});",
                render_java_string(&fixture.document().path)
            )?;
            writeln!(body)?;
            if skip_flag == "true" {
                writeln!(body, "        if (!Files.exists(documentPath)) {{")?;
                writeln!(
                    body,
                    "            String msg = String.format(\"Skipping {}: missing document at %s\", documentPath);",
                    fixture.id
                )?;
                writeln!(body, "            System.err.println(msg);")?;
                writeln!(
                    body,
                    "            org.junit.jupiter.api.Assumptions.assumeTrue(false, msg);"
                )?;
                writeln!(body, "            return;")?;
                writeln!(body, "        }}")?;
            }
            writeln!(body)?;
            writeln!(body, "        byte[] documentBytes = Files.readAllBytes(documentPath);")?;
            writeln!(
                body,
                "        String mimeType = Kreuzberg.detectMimeType(documentBytes);"
            )?;
            writeln!(
                body,
                "        ExtractionConfig extractionConfig = E2EHelpers.buildConfig({});",
                config_var
            )?;
            writeln!(
                body,
                "        List<BytesWithMime> items = Arrays.asList(new BytesWithMime(documentBytes, mimeType));"
            )?;
            writeln!(
                body,
                "        java.util.concurrent.CompletableFuture<List<ExtractionResult>> future = Kreuzberg.batchExtractBytesAsync(items, extractionConfig);"
            )?;
            writeln!(body, "        List<ExtractionResult> results;")?;
            writeln!(body, "        try {{")?;
            writeln!(body, "            results = future.get();")?;
            writeln!(body, "        }} catch (java.util.concurrent.ExecutionException e) {{")?;
            writeln!(
                body,
                "            Throwable cause = e.getCause() != null ? e.getCause() : e;"
            )?;
            writeln!(body, "            if (cause instanceof Exception) {{")?;
            writeln!(
                body,
                "                String skipReason = E2EHelpers.skipReasonFor((Exception) cause, {}, {}, {});",
                render_java_string(&fixture.id),
                requirements_expr,
                notes_expr
            )?;
            writeln!(body, "                if (skipReason != null) {{")?;
            writeln!(
                body,
                "                    org.junit.jupiter.api.Assumptions.assumeTrue(false, skipReason);"
            )?;
            writeln!(body, "                    return;")?;
            writeln!(body, "                }}")?;
            writeln!(body, "            }}")?;
            writeln!(body, "            throw e;")?;
            writeln!(body, "        }}")?;
            writeln!(body)?;
            writeln!(
                body,
                "        assertTrue(results.size() == 1, \"Expected exactly 1 result from batch extraction\");"
            )?;
            writeln!(body, "        ExtractionResult result = results.get(0);")?;
            writeln!(body)?;

            let assertions = render_assertions(&fixture.assertions());
            if !assertions.is_empty() {
                for line in assertions.lines() {
                    let trimmed = line.trim_start();
                    if !trimmed.is_empty() {
                        writeln!(body, "        {}", trimmed)?;
                    }
                }
            }
        }
    }

    writeln!(body, "    }}")?;

    Ok(body)
}

fn render_assertions(assertions: &Assertions) -> String {
    let mut buffer = String::new();

    if !assertions.expected_mime.is_empty() {
        buffer.push_str(&format!(
            "                E2EHelpers.Assertions.assertExpectedMime(result, {});\n",
            render_string_list(&assertions.expected_mime)
        ));
    }

    if let Some(min) = assertions.min_content_length {
        buffer.push_str(&format!(
            "                E2EHelpers.Assertions.assertMinContentLength(result, {});\n",
            min
        ));
    }

    if let Some(max) = assertions.max_content_length {
        buffer.push_str(&format!(
            "                E2EHelpers.Assertions.assertMaxContentLength(result, {});\n",
            max
        ));
    }

    if !assertions.content_contains_any.is_empty() {
        buffer.push_str(&format!(
            "                E2EHelpers.Assertions.assertContentContainsAny(result, {});\n",
            render_string_list(&assertions.content_contains_any)
        ));
    }

    if !assertions.content_contains_all.is_empty() {
        buffer.push_str(&format!(
            "                E2EHelpers.Assertions.assertContentContainsAll(result, {});\n",
            render_string_list(&assertions.content_contains_all)
        ));
    }

    if let Some(tables) = assertions.tables.as_ref() {
        let min_literal = tables.min.map(|v| v.to_string()).unwrap_or_else(|| "null".to_string());
        let max_literal = tables.max.map(|v| v.to_string()).unwrap_or_else(|| "null".to_string());
        buffer.push_str(&format!(
            "                E2EHelpers.Assertions.assertTableCount(result, {}, {});\n",
            min_literal, max_literal
        ));
        if tables.has_bounding_boxes == Some(true) {
            buffer.push_str("                E2EHelpers.Assertions.assertTableBoundingBoxes(result);\n");
        }
        if let Some(snippets) = tables.content_contains_any.as_ref()
            && !snippets.is_empty()
        {
            buffer.push_str(&format!(
                "                E2EHelpers.Assertions.assertTableContentContainsAny(result, {});\n",
                render_string_list(snippets)
            ));
        }
    }

    if let Some(languages) = assertions.detected_languages.as_ref() {
        let expected = render_string_list(&languages.expects);
        let min_conf = languages
            .min_confidence
            .map(|v| format!("{:.2}", v))
            .unwrap_or_else(|| "null".to_string());
        buffer.push_str(&format!(
            "                E2EHelpers.Assertions.assertDetectedLanguages(result, {}, {});\n",
            expected, min_conf
        ));
    }

    if !assertions.metadata.is_empty() {
        for (path, expectation) in &assertions.metadata {
            buffer.push_str(&format!(
                "                E2EHelpers.Assertions.assertMetadataExpectation(result, {}, {});\n",
                render_java_string(path),
                render_java_map(expectation)
            ));
        }
    }

    if let Some(chunks) = assertions.chunks.as_ref() {
        let min_literal = chunks
            .min_count
            .map(|v| v.to_string())
            .unwrap_or_else(|| "null".to_string());
        let max_literal = chunks
            .max_count
            .map(|v| v.to_string())
            .unwrap_or_else(|| "null".to_string());
        let each_has_content = chunks
            .each_has_content
            .map(|v| v.to_string())
            .unwrap_or_else(|| "null".to_string());
        let each_has_embedding = chunks
            .each_has_embedding
            .map(|v| v.to_string())
            .unwrap_or_else(|| "null".to_string());
        let each_has_heading_context = chunks
            .each_has_heading_context
            .map(|v| v.to_string())
            .unwrap_or_else(|| "null".to_string());
        let each_has_chunk_type = chunks
            .each_has_chunk_type
            .map(|v| v.to_string())
            .unwrap_or_else(|| "null".into());
        let content_starts_with_heading = chunks
            .content_starts_with_heading
            .map(|v| v.to_string())
            .unwrap_or_else(|| "null".into());
        buffer.push_str(&format!(
            "                E2EHelpers.Assertions.assertChunks(result, {}, {}, {}, {}, {}, {}, {});\n",
            min_literal,
            max_literal,
            each_has_content,
            each_has_embedding,
            each_has_heading_context,
            each_has_chunk_type,
            content_starts_with_heading
        ));
    }

    if let Some(images) = assertions.images.as_ref() {
        let min_literal = images
            .min_count
            .map(|v| v.to_string())
            .unwrap_or_else(|| "null".to_string());
        let max_literal = images
            .max_count
            .map(|v| v.to_string())
            .unwrap_or_else(|| "null".to_string());
        let formats_expr = images
            .formats_include
            .as_ref()
            .map(|f| render_string_list(f))
            .unwrap_or_else(|| "null".to_string());
        buffer.push_str(&format!(
            "                E2EHelpers.Assertions.assertImages(result, {}, {}, {});\n",
            min_literal, max_literal, formats_expr
        ));
        if images.has_bounding_boxes == Some(true) {
            buffer.push_str("                E2EHelpers.Assertions.assertImageBoundingBoxes(result);\n");
        }
    }

    if let Some(pages) = assertions.pages.as_ref() {
        let min_literal = pages
            .min_count
            .map(|v| v.to_string())
            .unwrap_or_else(|| "null".to_string());
        let exact_literal = pages
            .exact_count
            .map(|v| v.to_string())
            .unwrap_or_else(|| "null".to_string());
        let has_layout_literal = pages
            .has_layout_regions
            .map(|v| v.to_string())
            .unwrap_or_else(|| "null".to_string());
        let layout_classes_expr = pages
            .layout_classes_include
            .as_ref()
            .map(|c| render_string_list(c))
            .unwrap_or_else(|| "null".to_string());
        buffer.push_str(&format!(
            "                E2EHelpers.Assertions.assertPages(result, {}, {}, {}, {});\n",
            min_literal, exact_literal, has_layout_literal, layout_classes_expr
        ));
    }

    if let Some(elements) = assertions.elements.as_ref() {
        let min_literal = elements
            .min_count
            .map(|v| v.to_string())
            .unwrap_or_else(|| "null".to_string());
        let types_expr = elements
            .types_include
            .as_ref()
            .map(|t| render_string_list(t))
            .unwrap_or_else(|| "null".to_string());
        buffer.push_str(&format!(
            "                E2EHelpers.Assertions.assertElements(result, {}, {});\n",
            min_literal, types_expr
        ));
    }

    if let Some(ocr) = assertions.ocr_elements.as_ref() {
        let has_elements = ocr
            .has_elements
            .map(|v| v.to_string())
            .unwrap_or_else(|| "null".to_string());
        let has_geometry = ocr
            .elements_have_geometry
            .map(|v| v.to_string())
            .unwrap_or_else(|| "null".to_string());
        let has_confidence = ocr
            .elements_have_confidence
            .map(|v| v.to_string())
            .unwrap_or_else(|| "null".to_string());
        let min_literal = ocr
            .min_count
            .map(|v| v.to_string())
            .unwrap_or_else(|| "null".to_string());
        buffer.push_str(&format!(
            "                E2EHelpers.Assertions.assertOcrElements(result, {}, {}, {}, {});\n",
            has_elements, has_geometry, has_confidence, min_literal
        ));
    }

    if let Some(document) = assertions.document.as_ref() {
        let has_document = document.has_document.to_string();
        let min_node_count = document
            .min_node_count
            .map(|v| v.to_string())
            .unwrap_or_else(|| "null".to_string());
        let node_types = if !document.node_types_include.is_empty() {
            render_string_list(&document.node_types_include)
        } else {
            "null".to_string()
        };
        let has_groups = document
            .has_groups
            .map(|v| v.to_string())
            .unwrap_or_else(|| "null".to_string());
        buffer.push_str(&format!(
            "                E2EHelpers.Assertions.assertDocument(result, {}, {}, {}, {});\n",
            has_document, min_node_count, node_types, has_groups
        ));
    }

    if let Some(keywords) = assertions.keywords.as_ref() {
        let has_keywords = keywords
            .has_keywords
            .map(|v| v.to_string())
            .unwrap_or_else(|| "null".to_string());
        let min_literal = keywords
            .min_count
            .map(|v| v.to_string())
            .unwrap_or_else(|| "null".to_string());
        let max_literal = keywords
            .max_count
            .map(|v| v.to_string())
            .unwrap_or_else(|| "null".to_string());
        buffer.push_str(&format!(
            "                E2EHelpers.Assertions.assertKeywords(result, {}, {}, {});\n",
            has_keywords, min_literal, max_literal
        ));
    }

    if assertions.content_not_empty == Some(true) {
        buffer.push_str("                E2EHelpers.Assertions.assertContentNotEmpty(result);\n");
    }

    if let Some(qs) = assertions.quality_score.as_ref() {
        let has_score = qs
            .has_score
            .map(|v| v.to_string())
            .unwrap_or_else(|| "null".to_string());
        let min_score = qs
            .min_score
            .map(|v| format!("{:.2}", v))
            .unwrap_or_else(|| "null".to_string());
        let max_score = qs
            .max_score
            .map(|v| format!("{:.2}", v))
            .unwrap_or_else(|| "null".to_string());
        buffer.push_str(&format!(
            "                E2EHelpers.Assertions.assertQualityScore(result, {}, {}, {});\n",
            has_score, min_score, max_score
        ));
    }
    if let Some(pw) = assertions.processing_warnings.as_ref() {
        let max_count = pw
            .max_count
            .map(|v| v.to_string())
            .unwrap_or_else(|| "null".to_string());
        let is_empty = pw.is_empty.map(|v| v.to_string()).unwrap_or_else(|| "null".to_string());
        buffer.push_str(&format!(
            "                E2EHelpers.Assertions.assertProcessingWarnings(result, {}, {});\n",
            max_count, is_empty
        ));
    }
    if let Some(lu) = assertions.llm_usage.as_ref() {
        let max_count = lu
            .max_count
            .map(|v| v.to_string())
            .unwrap_or_else(|| "null".to_string());
        let is_empty = lu.is_empty.map(|v| v.to_string()).unwrap_or_else(|| "null".to_string());
        buffer.push_str(&format!(
            "                E2EHelpers.Assertions.assertLlmUsage(result, {}, {});\n",
            max_count, is_empty
        ));
    }
    if let Some(dc) = assertions.djot_content.as_ref() {
        let has_content = dc
            .has_content
            .map(|v| v.to_string())
            .unwrap_or_else(|| "null".to_string());
        let min_blocks = dc
            .min_blocks
            .map(|v| v.to_string())
            .unwrap_or_else(|| "null".to_string());
        buffer.push_str(&format!(
            "                E2EHelpers.Assertions.assertDjotContent(result, {}, {});\n",
            has_content, min_blocks
        ));
    }

    if let Some(annotations) = assertions.annotations.as_ref() {
        let has_annotations = annotations.has_annotations.to_string();
        let min_count = annotations
            .min_count
            .map(|v| v.to_string())
            .unwrap_or_else(|| "null".to_string());
        buffer.push_str(&format!(
            "                E2EHelpers.Assertions.assertAnnotations(result, {}, {});\n",
            has_annotations, min_count
        ));
    }

    if let Some(structured) = assertions.structured_output.as_ref() {
        let has_output = structured
            .has_output
            .map(|v| v.to_string())
            .unwrap_or_else(|| "null".to_string());
        let validates_schema = structured
            .validates_schema
            .map(|v| v.to_string())
            .unwrap_or_else(|| "null".to_string());
        let field_exists = if let Some(ref fields) = structured.field_exists {
            let parts = fields
                .iter()
                .map(|f| format!("\"{}\"", f))
                .collect::<Vec<_>>()
                .join(", ");
            format!("new String[]{{{}}}", parts)
        } else {
            "null".to_string()
        };
        buffer.push_str(&format!(
            "                E2EHelpers.assertStructuredOutput(result, {}, {}, {});\n",
            has_output, validates_schema, field_exists
        ));
    }

    buffer
}

fn render_config_expression(config: &Map<String, Value>) -> Result<Option<String>> {
    if config.is_empty() {
        Ok(None)
    } else {
        let json_str = serde_json::to_string(config)?;
        Ok(Some(json_str))
    }
}

fn render_string_list(items: &[String]) -> String {
    if items.is_empty() {
        "Collections.emptyList()".to_string()
    } else {
        let content = items
            .iter()
            .map(|s| render_java_string(s))
            .collect::<Vec<_>>()
            .join(", ");
        format!("Arrays.asList({})", content)
    }
}

fn render_optional_string(value: Option<&String>) -> String {
    match value {
        Some(text) => render_java_string(text),
        None => "null".to_string(),
    }
}

fn render_java_string(text: &str) -> String {
    let escaped = text
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t");
    format!("\"{}\"", escaped)
}

fn render_java_map(value: &Value) -> String {
    match value {
        Value::Object(map) => {
            if map.is_empty() {
                return "Collections.emptyMap()".to_string();
            }
            let pairs = map
                .iter()
                .map(|(k, v)| format!("{}, {}", render_java_string(k), render_java_value(v)))
                .collect::<Vec<_>>()
                .join(", ");
            format!("Map.of({})", pairs)
        }
        _ => {
            let value_expr = render_java_value(value);
            format!("Map.of(\"eq\", {})", value_expr)
        }
    }
}

fn render_java_value(value: &Value) -> String {
    match value {
        Value::Null => "null".to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        Value::String(s) => render_java_string(s),
        Value::Array(arr) => {
            if arr.is_empty() {
                "Collections.emptyList()".to_string()
            } else {
                let items = arr.iter().map(render_java_value).collect::<Vec<_>>().join(", ");
                format!("Arrays.asList({})", items)
            }
        }
        Value::Object(map) => render_java_map(&Value::Object(map.clone())),
    }
}

fn to_java_class_name(input: &str) -> String {
    let mut output = String::new();
    let mut capitalize_next = true;

    for ch in input.chars() {
        if ch.is_ascii_alphanumeric() {
            if capitalize_next {
                output.push(ch.to_ascii_uppercase());
                capitalize_next = false;
            } else {
                output.push(ch);
            }
        } else {
            capitalize_next = true;
        }
    }

    if output.is_empty() {
        "Fixture".to_string()
    } else if output.chars().next().unwrap().is_ascii_digit() {
        format!("Test{}", output)
    } else {
        output
    }
}

fn to_java_method_name(input: &str) -> String {
    let mut output = String::new();
    let mut capitalize_next = false;

    for (idx, ch) in input.chars().enumerate() {
        if ch.is_ascii_alphanumeric() {
            if idx == 0 {
                output.push(ch.to_ascii_lowercase());
            } else if capitalize_next {
                output.push(ch.to_ascii_uppercase());
                capitalize_next = false;
            } else {
                output.push(ch);
            }
        } else {
            capitalize_next = true;
        }
    }

    if output.is_empty() {
        "testFixture".to_string()
    } else if output.chars().next().unwrap().is_ascii_digit() {
        format!("test{}", output)
    } else {
        output
    }
}

fn collect_requirements(fixture: &Fixture) -> Vec<String> {
    fixture
        .skip()
        .requires_feature
        .iter()
        .chain(fixture.document().requires_external_tool.iter())
        .filter(|value| !value.is_empty())
        .map(|value| value.to_string())
        .collect()
}

fn generate_plugin_api_tests(fixtures: &[&Fixture], output_dir: &Utf8Path) -> Result<()> {
    let test_file = output_dir.join("PluginAPIsTest.java");

    let mut content = String::new();

    writeln!(content, "// Auto-generated from fixtures/plugin_api/ - DO NOT EDIT")?;
    writeln!(content, "package com.kreuzberg.e2e;")?;
    writeln!(content)?;
    writeln!(content, "import static org.junit.jupiter.api.Assertions.*;")?;
    writeln!(content)?;
    writeln!(content, "import dev.kreuzberg.config.ExtractionConfig;")?;
    writeln!(content, "import dev.kreuzberg.Kreuzberg;")?;
    writeln!(content, "import dev.kreuzberg.KreuzbergException;")?;
    writeln!(content, "import java.io.IOException;")?;
    writeln!(content, "import java.nio.file.Files;")?;
    writeln!(content, "import java.nio.file.Path;")?;
    writeln!(content, "import java.util.List;")?;
    writeln!(content, "import org.junit.jupiter.api.DisplayName;")?;
    writeln!(content, "import org.junit.jupiter.api.Test;")?;
    writeln!(content, "import org.junit.jupiter.api.io.TempDir;")?;
    writeln!(content)?;

    writeln!(content, "/**")?;
    writeln!(content, " * E2E tests for plugin/config/utility APIs.")?;
    writeln!(content, " *")?;
    writeln!(content, " * <p>Generated from plugin API fixtures.")?;
    writeln!(
        content,
        " * To regenerate: cargo run -p kreuzberg-e2e-generator -- generate --lang java"
    )?;
    writeln!(content, " *")?;
    writeln!(content, " * @since 4.0.0")?;
    writeln!(content, " */")?;
    writeln!(content, "@DisplayName(\"Plugin API Tests\")")?;
    writeln!(content, "class PluginAPIsTest {{")?;
    writeln!(content)?;

    let grouped = group_by_category(fixtures)?;

    for (category, fixtures) in grouped {
        writeln!(content, "    // {} Tests", category_to_title(category))?;
        writeln!(content)?;

        for fixture in fixtures {
            generate_java_test_method(fixture, &mut content)?;
            writeln!(content)?;
        }
    }

    writeln!(content, "}}")?;

    fs::write(&test_file, content).with_context(|| format!("Failed to write {test_file}"))?;

    Ok(())
}

fn group_by_category<'a>(fixtures: &[&'a Fixture]) -> Result<BTreeMap<&'a str, Vec<&'a Fixture>>> {
    let mut grouped: BTreeMap<&str, Vec<&Fixture>> = BTreeMap::new();
    for fixture in fixtures {
        let category = fixture
            .api_category
            .as_ref()
            .with_context(|| format!("Fixture '{}' missing api_category", fixture.id))?
            .as_str();
        grouped.entry(category).or_default().push(fixture);
    }
    Ok(grouped)
}

fn category_to_title(category: &str) -> String {
    category
        .split('_')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn generate_java_test_method(fixture: &Fixture, buf: &mut String) -> Result<()> {
    let test_spec = fixture
        .test_spec
        .as_ref()
        .with_context(|| format!("Fixture '{}' missing test_spec", fixture.id))?;
    let test_name = to_java_method_name(&fixture.id);

    if test_spec.pattern == "config_discover" {
        writeln!(
            buf,
            "    // SKIPPED: config_discover - System.setProperty(\"user.dir\") doesn't affect FFI working directory"
        )?;
        return Ok(());
    }

    writeln!(buf, "    @Test")?;

    writeln!(buf, "    @DisplayName(\"{}\")", fixture.description)?;

    match test_spec.pattern.as_str() {
        "config_from_file" | "mime_from_path" => {
            writeln!(
                buf,
                "    void {}(@TempDir Path tempDir) throws IOException, KreuzbergException {{",
                test_name
            )?;
        }
        _ => {
            writeln!(buf, "    void {}() throws KreuzbergException {{", test_name)?;
        }
    }

    match test_spec.pattern.as_str() {
        "simple_list" => generate_simple_list_test_java(test_spec, buf)?,
        "clear_registry" => generate_clear_registry_test_java(test_spec, buf)?,
        "graceful_unregister" => generate_graceful_unregister_test_java(test_spec, buf)?,
        "config_from_file" => generate_config_from_file_test_java(test_spec, buf)?,
        "mime_from_bytes" => generate_mime_from_bytes_test_java(test_spec, buf)?,
        "mime_from_path" => generate_mime_from_path_test_java(test_spec, buf)?,
        "mime_extension_lookup" => generate_mime_extension_lookup_test_java(test_spec, buf)?,
        _ => anyhow::bail!("Unknown test pattern: {}", test_spec.pattern),
    }

    writeln!(buf, "    }}")?;

    Ok(())
}

fn generate_simple_list_test_java(test_spec: &PluginTestSpec, buf: &mut String) -> Result<()> {
    let func_name = snake_to_camel(&test_spec.function_call.name);
    let assertions = &test_spec.assertions;

    writeln!(buf, "        List<String> result = Kreuzberg.{}();", func_name)?;

    writeln!(buf, "        assertNotNull(result);")?;

    if let Some(item_type) = &assertions.list_item_type
        && item_type == "string"
    {
        writeln!(
            buf,
            "        assertTrue(result.stream().allMatch(item -> item instanceof String));"
        )?;
    }

    if let Some(contains) = &assertions.list_contains {
        writeln!(buf, "        assertTrue(result.contains(\"{}\"));", contains)?;
    }

    Ok(())
}

fn generate_clear_registry_test_java(test_spec: &PluginTestSpec, buf: &mut String) -> Result<()> {
    let clear_func = snake_to_camel(&test_spec.function_call.name);

    writeln!(buf, "        Kreuzberg.{}();", clear_func)?;

    if test_spec.assertions.verify_cleanup {
        let list_func = clear_func.replace("clear", "list");
        writeln!(buf, "        List<String> result = Kreuzberg.{}();", list_func)?;
        writeln!(buf, "        assertEquals(0, result.size());")?;
    }

    Ok(())
}

fn generate_graceful_unregister_test_java(test_spec: &PluginTestSpec, buf: &mut String) -> Result<()> {
    let func_name = snake_to_camel(&test_spec.function_call.name);
    let arg = test_spec
        .function_call
        .args
        .first()
        .with_context(|| format!("Function '{}' missing argument", test_spec.function_call.name))?;
    let arg_str = arg
        .as_str()
        .with_context(|| format!("Function '{}' argument is not a string", test_spec.function_call.name))?;

    writeln!(
        buf,
        "        assertDoesNotThrow(() -> Kreuzberg.{}(\"{}\"));",
        func_name, arg_str
    )?;

    Ok(())
}

fn generate_config_from_file_test_java(test_spec: &PluginTestSpec, buf: &mut String) -> Result<()> {
    let setup = test_spec
        .setup
        .as_ref()
        .with_context(|| "Test spec missing setup for config_from_file")?;
    let file_content = setup
        .temp_file_content
        .as_ref()
        .with_context(|| "Setup missing temp_file_content")?;
    let file_name = setup
        .temp_file_name
        .as_ref()
        .with_context(|| "Setup missing temp_file_name")?;

    writeln!(buf, "        Path configPath = tempDir.resolve(\"{}\");", file_name)?;
    writeln!(buf, "        Files.writeString(configPath, \"\"\"")?;
    writeln!(buf, "{}\"\"\");", file_content)?;
    writeln!(buf)?;

    writeln!(
        buf,
        "        ExtractionConfig config = ExtractionConfig.fromFile(configPath.toString());"
    )?;

    generate_object_property_assertions_java(&test_spec.assertions, buf)?;

    Ok(())
}

fn generate_mime_from_bytes_test_java(test_spec: &PluginTestSpec, buf: &mut String) -> Result<()> {
    let setup = test_spec
        .setup
        .as_ref()
        .with_context(|| "Test spec missing setup for mime_from_bytes")?;
    let test_data = setup.test_data.as_ref().with_context(|| "Setup missing test_data")?;
    let func_name = snake_to_camel(&test_spec.function_call.name);

    let bytes_literal = test_data.clone();
    writeln!(buf, "        byte[] testBytes = \"{}\".getBytes();", bytes_literal)?;
    writeln!(buf, "        String result = Kreuzberg.{}(testBytes);", func_name)?;

    if let Some(contains) = &test_spec.assertions.string_contains {
        writeln!(
            buf,
            "        assertTrue(result.toLowerCase().contains(\"{}\"));",
            contains
        )?;
    }

    Ok(())
}

fn generate_mime_from_path_test_java(test_spec: &PluginTestSpec, buf: &mut String) -> Result<()> {
    let setup = test_spec
        .setup
        .as_ref()
        .with_context(|| "Test spec missing setup for mime_from_path")?;
    let file_name = setup
        .temp_file_name
        .as_ref()
        .with_context(|| "Setup missing temp_file_name")?;
    let file_content = setup
        .temp_file_content
        .as_ref()
        .with_context(|| "Setup missing temp_file_content")?;
    let func_name = snake_to_camel(&test_spec.function_call.name);

    writeln!(buf, "        Path testFile = tempDir.resolve(\"{}\");", file_name)?;
    writeln!(buf, "        Files.writeString(testFile, \"{}\");", file_content)?;
    writeln!(buf)?;

    writeln!(
        buf,
        "        String result = Kreuzberg.{}(testFile.toString());",
        func_name
    )?;

    if let Some(contains) = &test_spec.assertions.string_contains {
        writeln!(
            buf,
            "        assertTrue(result.toLowerCase().contains(\"{}\"));",
            contains
        )?;
    }

    Ok(())
}

fn generate_mime_extension_lookup_test_java(test_spec: &PluginTestSpec, buf: &mut String) -> Result<()> {
    let func_name = snake_to_camel(&test_spec.function_call.name);
    let arg = test_spec
        .function_call
        .args
        .first()
        .with_context(|| format!("Function '{}' missing argument", test_spec.function_call.name))?;
    let mime_type = arg
        .as_str()
        .with_context(|| format!("Function '{}' argument is not a string", test_spec.function_call.name))?;

    writeln!(
        buf,
        "        List<String> result = Kreuzberg.{}(\"{}\");",
        func_name, mime_type
    )?;
    writeln!(buf, "        assertNotNull(result);")?;

    if let Some(contains) = &test_spec.assertions.list_contains {
        writeln!(buf, "        assertTrue(result.contains(\"{}\"));", contains)?;
    }

    Ok(())
}

fn generate_object_property_assertions_java(assertions: &PluginAssertions, buf: &mut String) -> Result<()> {
    for prop in &assertions.object_properties {
        let parts: Vec<&str> = prop.path.split('.').collect();

        let is_bool_property = prop.value.as_ref().map(|v| v.is_boolean()).unwrap_or(false);

        if let Some(exists) = prop.exists
            && exists
        {
            let mut path = "config".to_string();
            for (i, part) in parts.iter().enumerate() {
                let is_last = i == parts.len() - 1;
                let is_bool = is_last && is_bool_property;
                let getter = if is_bool {
                    property_to_is_getter(part)
                } else {
                    property_to_getter(part)
                };
                writeln!(buf, "        assertNotNull({}.{}());", path, getter)?;
                path = format!("{}.{}()", path, getter);
            }
        }

        if let Some(value) = &prop.value {
            let mut getter_path = String::from("config");
            for (i, part) in parts.iter().enumerate() {
                let is_last = i == parts.len() - 1;
                let is_bool = is_last && is_bool_property;
                let getter = if is_bool {
                    property_to_is_getter(part)
                } else {
                    property_to_getter(part)
                };
                getter_path = format!("{}.{}()", getter_path, getter);
            }

            match value {
                Value::Number(n) => {
                    writeln!(buf, "        assertEquals({}, {});", n, getter_path)?;
                }
                Value::Bool(b) => {
                    if *b {
                        writeln!(buf, "        assertTrue({});", getter_path)?;
                    } else {
                        writeln!(buf, "        assertFalse({});", getter_path)?;
                    }
                }
                Value::String(s) => {
                    writeln!(buf, "        assertEquals(\"{}\", {});", s, getter_path)?;
                }
                _ => {}
            }
        }
    }

    Ok(())
}

fn snake_to_camel(input: &str) -> String {
    if input == "ocr_backends" {
        return "oCRBackends".to_string();
    }
    if input.starts_with("unregister_ocr_backend") {
        return "unregisterOCRBackend".to_string();
    }
    if input.starts_with("clear_ocr_backends") {
        return "clearOCRBackends".to_string();
    }
    if input.starts_with("list_ocr_backends") {
        return "listOCRBackends".to_string();
    }

    let mut result = String::new();
    let mut capitalize_next = false;

    for (i, ch) in input.chars().enumerate() {
        if ch == '_' {
            capitalize_next = true;
        } else if i == 0 {
            result.push(ch.to_ascii_lowercase());
        } else if capitalize_next {
            result.push(ch.to_ascii_uppercase());
            capitalize_next = false;
        } else {
            result.push(ch);
        }
    }

    result
}

fn property_to_getter(property: &str) -> String {
    let mut result = String::new();
    let mut capitalize_next = true;

    for ch in property.chars() {
        if ch == '_' {
            capitalize_next = true;
        } else if capitalize_next {
            result.push(ch.to_ascii_uppercase());
            capitalize_next = false;
        } else {
            result.push(ch);
        }
    }

    format!("get{}", result)
}

fn property_to_is_getter(property: &str) -> String {
    let mut result = String::new();
    let mut capitalize_next = true;

    for ch in property.chars() {
        if ch == '_' {
            capitalize_next = true;
        } else if capitalize_next {
            result.push(ch.to_ascii_uppercase());
            capitalize_next = false;
        } else {
            result.push(ch);
        }
    }

    format!("is{}", result)
}

fn render_render_category(fixtures: &[&Fixture]) -> Result<String> {
    let mut buffer = String::new();
    writeln!(buffer, "package com.kreuzberg.e2e;")?;
    writeln!(buffer)?;
    writeln!(buffer, "// Code generated by kreuzberg-e2e-generator. DO NOT EDIT.")?;
    writeln!(
        buffer,
        "// To regenerate: cargo run -p kreuzberg-e2e-generator -- generate --lang java"
    )?;
    writeln!(buffer)?;
    writeln!(buffer, "import dev.kreuzberg.Kreuzberg;")?;
    writeln!(buffer, "import dev.kreuzberg.Kreuzberg.PdfPageIterator;")?;
    writeln!(buffer, "import dev.kreuzberg.Kreuzberg.PageResult;")?;
    writeln!(buffer, "import org.junit.jupiter.api.Test;")?;
    writeln!(buffer)?;
    writeln!(buffer, "import java.nio.file.Files;")?;
    writeln!(buffer, "import java.nio.file.Path;")?;
    writeln!(buffer)?;
    writeln!(buffer, "import static org.junit.jupiter.api.Assertions.assertTrue;")?;
    writeln!(buffer, "import static org.junit.jupiter.api.Assumptions.assumeTrue;")?;
    writeln!(buffer)?;
    writeln!(buffer, "/** Tests for render fixtures. */")?;
    writeln!(buffer, "public class RenderTest {{")?;
    writeln!(buffer)?;

    for fixture in fixtures {
        buffer.push_str(&render_render_test_java(fixture)?);
        writeln!(buffer)?;
    }

    writeln!(buffer, "}}")?;
    Ok(buffer)
}

fn render_render_test_java(fixture: &Fixture) -> Result<String> {
    let mut body = String::new();
    let render = fixture.render.as_ref().expect("render spec required");
    let assertions = fixture.assertions().render.unwrap_or_default();
    let method_name = to_java_method_name(&fixture.id);
    let doc_path = render_java_string(&fixture.document().path);

    writeln!(body, "    @Test")?;
    writeln!(body, "    public void {method_name}() throws Exception {{")?;
    writeln!(
        body,
        "        Path documentPath = E2EHelpers.resolveDocument({doc_path});"
    )?;
    writeln!(body, "        assumeTrue(Files.exists(documentPath),")?;
    writeln!(body, "                \"Skipping {}: missing document\");", fixture.id)?;

    let dpi_arg = render.dpi.map(|d| format!(", {d}")).unwrap_or_default();

    match render.mode.as_str() {
        "single_page" => {
            let page_index = render.page_index.unwrap_or(0);
            writeln!(
                body,
                "        byte[] pngData = Kreuzberg.renderPdfPage(documentPath, {page_index}{dpi_arg});"
            )?;
            render_render_assertions_java(&assertions, "pngData", &mut body, "        ")?;
        }
        "iterator" => {
            writeln!(body, "        int pageCount = 0;")?;
            writeln!(
                body,
                "        try (PdfPageIterator iter = PdfPageIterator.open(documentPath{dpi_arg})) {{"
            )?;
            writeln!(body, "            while (iter.hasNext()) {{")?;
            writeln!(body, "                PageResult page = iter.next();")?;
            writeln!(body, "                E2EHelpers.assertIsPng(page.data());")?;
            writeln!(body, "                pageCount++;")?;
            writeln!(body, "            }}")?;
            writeln!(body, "        }}")?;
            if let Some(page_count_gte) = assertions.page_count_gte {
                writeln!(body, "        assertTrue(pageCount >= {page_count_gte},")?;
                writeln!(
                    body,
                    "                String.format(\"Expected at least {page_count_gte} pages, got %d\", pageCount));"
                )?;
            }
        }
        _ => anyhow::bail!("Unknown render mode: {}", render.mode),
    }

    writeln!(body, "    }}")?;
    Ok(body)
}

fn render_render_assertions_java(
    assertions: &RenderAssertions,
    var: &str,
    code: &mut String,
    indent: &str,
) -> Result<()> {
    if assertions.is_png == Some(true) {
        writeln!(code, "{indent}E2EHelpers.assertIsPng({var});")?;
    }
    if let Some(min_len) = assertions.min_byte_length {
        writeln!(code, "{indent}E2EHelpers.assertMinByteLength({var}, {min_len});")?;
    }
    Ok(())
}

/// Java types implemented as records (use field-name accessors, not getters).
///
/// Records use `fieldName()` style accessors instead of `getFieldName()`.
/// NOTE: Only include types that are actual Java records. Types implemented as
/// regular classes with getX()/isX() methods must NOT be listed here.
const JAVA_RECORD_TYPES: &[&str] = &[];

/// Special field name mappings for Java (manifest field name -> Java accessor name).
fn java_field_override(type_name: &str, field_name: &str) -> Option<String> {
    match (type_name, field_name) {
        ("ExtractionResult", "document") => Some("getDocumentStructure".to_string()),
        _ => None,
    }
}

/// Return the correct Java accessor for a field, respecting records vs classes
/// and any per-field overrides.
///
/// When `json_type` is `"boolean"`, non-record types use the `isFieldName()`
/// convention instead of `getFieldName()`.
fn to_java_accessor_with_type(type_name: &str, field_name: &str, json_type: Option<&str>) -> String {
    if let Some(override_name) = java_field_override(type_name, field_name) {
        return override_name;
    }
    if JAVA_RECORD_TYPES.contains(&type_name) {
        parity::to_camel_case(field_name)
    } else if json_type == Some("boolean") {
        format!("is{}", parity::to_pascal_case(field_name))
    } else {
        format!("get{}", parity::to_pascal_case(field_name))
    }
}

/// Generate parity tests for the Java binding.
///
/// Produces `e2e/java/src/test/java/com/kreuzberg/e2e/ParityTest.java` that
/// verifies all manifest struct types expose the expected getter methods via
/// reflection.
pub fn generate_parity(manifest: &ParityManifest, output_root: &Utf8Path, _mode: &GenerationMode) -> Result<()> {
    let src_test = output_root.join("java/src/test/java/com/kreuzberg/e2e");
    fs::create_dir_all(&src_test).context("Failed to create Java test directory for parity")?;

    let lang = "java";
    let profile_name = parity::profile_for_language(lang);
    let enabled_features = manifest.feature_profiles.get(profile_name).cloned().unwrap_or_default();

    let mut buffer = String::new();
    writeln!(buffer, "// Code generated by kreuzberg-e2e-generator. DO NOT EDIT.")?;
    writeln!(
        buffer,
        "// To regenerate: cargo run -p kreuzberg-e2e-generator -- generate --lang java"
    )?;
    writeln!(buffer, "package com.kreuzberg.e2e;")?;
    writeln!(buffer)?;
    writeln!(buffer, "import static org.junit.jupiter.api.Assertions.*;")?;
    writeln!(buffer)?;
    writeln!(buffer, "import dev.kreuzberg.ExtractionResult;")?;
    writeln!(buffer, "import dev.kreuzberg.config.ExtractionConfig;")?;

    // Collect additional struct type names for imports
    let mut extra_imports: Vec<String> = Vec::new();
    for (type_name, type_def) in &manifest.types {
        if type_name == "ExtractionResult" || type_name == "ExtractionConfig" {
            continue;
        }
        let has_fields = match type_def {
            TypeDef::Struct { fields } => {
                let filtered = parity::filter_fields_for_profile(fields, &enabled_features);
                !filtered.is_empty()
            }
            _ => false,
        };
        if has_fields {
            extra_imports.push(type_name.clone());
        }
    }
    for type_name in &extra_imports {
        writeln!(buffer, "import dev.kreuzberg.{type_name};")?;
    }

    writeln!(buffer, "import org.junit.jupiter.api.Test;")?;
    writeln!(buffer)?;
    writeln!(buffer, "class ParityTest {{")?;

    // ExtractionResult parity
    if let Some(fields) = parity::fields_for_type_and_lang(manifest, "ExtractionResult", lang) {
        let required_fields: Vec<(&String, String)> = fields
            .iter()
            .filter(|(_, f)| f.required)
            .map(|(name, f)| {
                (
                    name,
                    to_java_accessor_with_type("ExtractionResult", name, Some(&f.json_type)),
                )
            })
            .collect();
        let all_fields: Vec<(&String, String)> = fields
            .iter()
            .map(|(name, f)| {
                (
                    name,
                    to_java_accessor_with_type("ExtractionResult", name, Some(&f.json_type)),
                )
            })
            .collect();

        writeln!(buffer)?;
        writeln!(buffer, "    @Test")?;
        writeln!(
            buffer,
            "    void testExtractionResultRequiredGetters() throws Exception {{"
        )?;
        writeln!(
            buffer,
            "        String[] requiredGetters = {{{}}};",
            required_fields
                .iter()
                .map(|(_, getter)| format!("\"{getter}\""))
                .join(", ")
        )?;
        writeln!(buffer, "        for (String getter : requiredGetters) {{")?;
        writeln!(buffer, "            assertDoesNotThrow(")?;
        writeln!(
            buffer,
            "                () -> ExtractionResult.class.getMethod(getter),"
        )?;
        writeln!(
            buffer,
            "                \"ExtractionResult missing required getter: \" + getter"
        )?;
        writeln!(buffer, "            );")?;
        writeln!(buffer, "        }}")?;
        writeln!(buffer, "    }}")?;

        writeln!(buffer)?;
        writeln!(buffer, "    @Test")?;
        writeln!(buffer, "    void testExtractionResultAllGetters() throws Exception {{")?;
        writeln!(
            buffer,
            "        String[] allGetters = {{{}}};",
            all_fields.iter().map(|(_, getter)| format!("\"{getter}\"")).join(", ")
        )?;
        writeln!(buffer, "        for (String getter : allGetters) {{")?;
        writeln!(buffer, "            assertDoesNotThrow(")?;
        writeln!(
            buffer,
            "                () -> ExtractionResult.class.getMethod(getter),"
        )?;
        writeln!(buffer, "                \"ExtractionResult missing getter: \" + getter")?;
        writeln!(buffer, "            );")?;
        writeln!(buffer, "        }}")?;
        writeln!(buffer, "    }}")?;
    }

    // ExtractionConfig parity
    if let Some(fields) = parity::fields_for_type_and_lang(manifest, "ExtractionConfig", lang) {
        let all_fields: Vec<(&String, String)> = fields
            .iter()
            .map(|(name, f)| {
                (
                    name,
                    to_java_accessor_with_type("ExtractionConfig", name, Some(&f.json_type)),
                )
            })
            .collect();

        writeln!(buffer)?;
        writeln!(buffer, "    @Test")?;
        writeln!(buffer, "    void testExtractionConfigAllGetters() throws Exception {{")?;
        writeln!(
            buffer,
            "        String[] allGetters = {{{}}};",
            all_fields.iter().map(|(_, getter)| format!("\"{getter}\"")).join(", ")
        )?;
        writeln!(buffer, "        for (String getter : allGetters) {{")?;
        writeln!(buffer, "            assertDoesNotThrow(")?;
        writeln!(
            buffer,
            "                () -> ExtractionConfig.class.getMethod(getter),"
        )?;
        writeln!(buffer, "                \"ExtractionConfig missing getter: \" + getter")?;
        writeln!(buffer, "            );")?;
        writeln!(buffer, "        }}")?;
        writeln!(buffer, "    }}")?;
    }

    // Additional struct types
    for (type_name, type_def) in &manifest.types {
        if type_name == "ExtractionResult" || type_name == "ExtractionConfig" {
            continue;
        }
        let fields = match type_def {
            TypeDef::Struct { fields } => parity::filter_fields_for_profile(fields, &enabled_features),
            _ => continue,
        };
        if fields.is_empty() {
            continue;
        }

        let all_fields: Vec<(&String, String)> = fields
            .iter()
            .map(|(name, f)| (name, to_java_accessor_with_type(type_name, name, Some(&f.json_type))))
            .collect();

        let method_name = format!("test{}AllGetters", type_name);

        writeln!(buffer)?;
        writeln!(buffer, "    @Test")?;
        writeln!(buffer, "    void {method_name}() throws Exception {{")?;
        writeln!(
            buffer,
            "        String[] allGetters = {{{}}};",
            all_fields.iter().map(|(_, getter)| format!("\"{getter}\"")).join(", ")
        )?;
        writeln!(buffer, "        for (String getter : allGetters) {{")?;
        writeln!(buffer, "            assertDoesNotThrow(")?;
        writeln!(buffer, "                () -> {type_name}.class.getMethod(getter),")?;
        writeln!(buffer, "                \"{type_name} missing getter: \" + getter")?;
        writeln!(buffer, "            );")?;
        writeln!(buffer, "        }}")?;
        writeln!(buffer, "    }}")?;
    }

    writeln!(buffer, "}}")?;

    fs::write(src_test.join("ParityTest.java"), &buffer).context("Failed to write Java parity test")?;

    Ok(())
}

fn rust_triple_to_java_condition(triple: &str) -> Option<String> {
    let (arch, os_name) = match triple {
        "aarch64-unknown-linux-gnu" | "aarch64-unknown-linux-musl" => ("aarch64", "Linux"),
        "x86_64-unknown-linux-gnu" | "x86_64-unknown-linux-musl" => ("amd64", "Linux"),
        "aarch64-apple-darwin" => ("aarch64", "Mac OS X"),
        "x86_64-apple-darwin" => ("x86_64", "Mac OS X"),
        "x86_64-pc-windows-msvc" => ("amd64", "Windows"),
        _ => return None,
    };
    Some(format!(
        "(System.getProperty(\"os.arch\").equals(\"{arch}\") && System.getProperty(\"os.name\").startsWith(\"{os_name}\"))"
    ))
}

fn generate_embed_tests(fixtures: &[Fixture], src_test: &Utf8Path) -> Result<()> {
    let embed_fixtures: Vec<_> = fixtures.iter().filter(|f| f.is_embed()).collect();
    if embed_fixtures.is_empty() {
        return Ok(());
    }

    let content = render_embed_category(&embed_fixtures)?;
    fs::write(src_test.join("EmbedStandaloneTest.java"), content)
        .context("Failed to write EmbedStandaloneTest.java")?;
    Ok(())
}

fn render_embed_category(fixtures: &[&Fixture]) -> Result<String> {
    let mut buffer = String::new();
    writeln!(buffer, "package com.kreuzberg.e2e;")?;
    writeln!(buffer)?;
    writeln!(buffer, "// Code generated by kreuzberg-e2e-generator. DO NOT EDIT.")?;
    writeln!(
        buffer,
        "// To regenerate: cargo run -p kreuzberg-e2e-generator -- generate --lang java"
    )?;
    writeln!(buffer)?;
    writeln!(buffer, "import dev.kreuzberg.Kreuzberg;")?;
    writeln!(buffer, "import dev.kreuzberg.config.EmbeddingConfig;")?;
    writeln!(buffer, "import java.util.Arrays;")?;
    writeln!(buffer, "import java.util.Collections;")?;
    writeln!(buffer, "import java.util.List;")?;
    writeln!(buffer, "import org.junit.jupiter.api.Test;")?;
    writeln!(buffer)?;
    writeln!(buffer, "import java.util.Arrays;")?;
    writeln!(buffer, "import java.util.List;")?;
    writeln!(buffer)?;
    writeln!(buffer, "import static org.junit.jupiter.api.Assertions.assertTrue;")?;
    writeln!(buffer, "import static org.junit.jupiter.api.Assumptions.assumeTrue;")?;
    writeln!(buffer)?;
    writeln!(buffer, "/** Tests for standalone embedding fixtures. */")?;
    writeln!(buffer, "public class EmbedStandaloneTest {{")?;
    writeln!(buffer)?;

    for fixture in fixtures {
        buffer.push_str(&render_embed_test_java(fixture)?);
        writeln!(buffer)?;
    }

    writeln!(buffer, "}}")?;
    Ok(buffer)
}

fn render_embed_test_java(fixture: &Fixture) -> Result<String> {
    let mut body = String::new();
    let embed = fixture.embed.as_ref().expect("embed spec required");
    let fixture_assertions = fixture.assertions();
    let assertions = fixture_assertions.embed.as_ref().expect("embed assertions required");
    let method_name = to_java_method_name(&fixture.id);

    writeln!(body, "    @Test")?;
    writeln!(body, "    public void {method_name}() throws Exception {{")?;

    if !fixture.id.contains("disabled") {
        writeln!(
            body,
            "        if ((System.getProperty(\"os.arch\").equals(\"amd64\") && System.getProperty(\"os.name\").startsWith(\"Windows\"))) {{"
        )?;
        writeln!(
            body,
            "            assumeTrue(false, \"Skipping {}: embeddings not supported on Windows x64 via ONNX yet\");",
            fixture.id
        )?;
        writeln!(body, "            return;")?;
        writeln!(body, "        }}")?;
    }

    let texts_expr = render_string_list(&embed.texts);
    let config_expr = if !embed.config.is_empty() {
        render_embed_config_java(&embed.config)?
    } else {
        "null".to_string()
    };

    writeln!(body, "        float[][] results;")?;
    writeln!(body, "        try {{")?;
    writeln!(
        body,
        "            results = Kreuzberg.embed({texts_expr}, {config_expr});"
    )?;
    writeln!(body, "        }} catch (Exception e) {{")?;
    writeln!(
        body,
        "            String skipReason = E2EHelpers.skipReasonFor(e, {}, Arrays.asList(\"embeddings\"), null);",
        render_java_string(&fixture.id)
    )?;
    writeln!(body, "            if (skipReason != null) {{")?;
    writeln!(body, "                assumeTrue(false, skipReason);")?;
    writeln!(body, "                return;")?;
    writeln!(body, "            }}")?;
    writeln!(body, "            throw e;")?;
    writeln!(body, "        }}")?;

    let count = assertions.count.map(|c| c as i32).unwrap_or(-1);
    let dimensions = assertions.dimensions.map(|d| d as i32).unwrap_or(-1);

    writeln!(
        body,
        "        E2EHelpers.Assertions.assertEmbedResult(results, {}, {}, {}, {}, {}, {});",
        count, dimensions, assertions.no_nan, assertions.no_inf, assertions.non_zero, assertions.normalized
    )?;

    writeln!(body, "    }}")?;
    Ok(body)
}

fn render_embed_config_java(config: &Map<String, Value>) -> Result<String> {
    use std::fmt::Write as _;
    let mut expr = "EmbeddingConfig.builder()".to_string();

    if let Some(model) = config.get("model")
        && let Some(name) = model.get("name").and_then(|v| v.as_str())
    {
        write!(expr, ".preset(\"{name}\")").unwrap();
    }

    if let Some(normalize) = config.get("normalize").and_then(|v| v.as_bool()) {
        write!(expr, ".normalize({normalize})").unwrap();
    }

    if let Some(batch_size) = config.get("batch_size").and_then(|v| v.as_i64()) {
        write!(expr, ".batchSize({batch_size})").unwrap();
    }

    write!(expr, ".build()").unwrap();
    Ok(expr)
}
