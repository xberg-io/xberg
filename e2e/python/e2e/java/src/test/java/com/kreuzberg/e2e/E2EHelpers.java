package com.kreuzberg.e2e;

import static org.junit.jupiter.api.Assertions.assertNotNull;
import static org.junit.jupiter.api.Assertions.assertTrue;
import static org.junit.jupiter.api.Assertions.fail;

import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.ObjectMapper;
import dev.kreuzberg.ExtractionResult;
import dev.kreuzberg.Kreuzberg;
import dev.kreuzberg.MissingDependencyException;
import dev.kreuzberg.Table;
import dev.kreuzberg.config.ExtractionConfig;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.Paths;
import java.util.List;
import java.util.Map;
import org.junit.jupiter.api.Assumptions;

/** Helper utilities for E2E tests. */
public final class E2EHelpers {
  private static final Path WORKSPACE_ROOT = Paths.get("").toAbsolutePath().getParent().getParent();
  private static final Path TEST_DOCUMENTS = WORKSPACE_ROOT.resolve("test_documents");
  private static final ObjectMapper MAPPER = new ObjectMapper();

  private E2EHelpers() {}

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
      Exception error, String fixtureId, List<String> requirements, String notes) {
    String message = error.getMessage() != null ? error.getMessage() : "";
    String lowered = message.toLowerCase();
    boolean requirementHit =
        requirements.stream().anyMatch(req -> lowered.contains(req.toLowerCase()));
    boolean missingDependency =
        error instanceof MissingDependencyException || lowered.contains("missing dependency");
    boolean unsupportedFormat = lowered.contains("unsupported format");

    if (!missingDependency && !unsupportedFormat && !requirementHit) {
      return null;
    }

    String reason;
    if (missingDependency) {
      if (error instanceof MissingDependencyException) {
        // Extract dependency from exception message if available
        String msg = error.getMessage();
        reason =
            msg != null && !msg.isEmpty() ? "missing dependency: " + msg : "missing dependency";
      } else {
        reason = "missing dependency";
      }
    } else if (unsupportedFormat) {
      reason = "unsupported format";
    } else {
      reason = "requires " + String.join(", ", requirements);
    }

    String details =
        String.format(
            "Skipping %s: %s. %s: %s",
            fixtureId, reason, error.getClass().getSimpleName(), message);
    if (notes != null && !notes.isEmpty()) {
      details += " Notes: " + notes;
    }
    System.err.println(details);
    return details;
  }

  public static void skipIfPaddleOcrUnavailable() {
    String flag = System.getenv("KREUZBERG_PADDLE_OCR_AVAILABLE");
    Assumptions.assumeTrue(
        flag != null && !flag.isEmpty() && !"0".equals(flag) && !"false".equalsIgnoreCase(flag),
        "Skipping: PaddleOCR not available (set KREUZBERG_PADDLE_OCR_AVAILABLE=1)");
  }

  public static void runFixture(
      String fixtureId,
      String relativePath,
      JsonNode configNode,
      List<String> requirements,
      String notes,
      boolean skipIfMissing,
      TestCallback callback)
      throws Exception {
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

  /** Assertion utilities for E2E tests. */
  public static final class Assertions {
    private Assertions() {}

    public static void assertExpectedMime(ExtractionResult result, List<String> expected) {
      if (expected.isEmpty()) {
        return;
      }
      String mimeType = result.getMimeType();
      boolean matches =
          expected.stream().anyMatch(token -> mimeType != null && mimeType.contains(token));
      assertTrue(
          matches,
          String.format("Expected mime type to contain one of %s, got %s", expected, mimeType));
    }

    public static void assertMinContentLength(ExtractionResult result, int minimum) {
      String content = result.getContent();
      int length = content != null ? content.length() : 0;
      assertTrue(
          length >= minimum,
          String.format("Expected content length >= %d, got %d", minimum, length));
    }

    public static void assertMaxContentLength(ExtractionResult result, int maximum) {
      String content = result.getContent();
      int length = content != null ? content.length() : 0;
      assertTrue(
          length <= maximum,
          String.format("Expected content length <= %d, got %d", maximum, length));
    }

    public static void assertContentContainsAny(ExtractionResult result, List<String> snippets) {
      if (snippets.isEmpty()) {
        return;
      }
      String content = result.getContent();
      String lowered = content != null ? content.toLowerCase() : "";
      boolean matches =
          snippets.stream().anyMatch(snippet -> lowered.contains(snippet.toLowerCase()));
      assertTrue(matches, String.format("Expected content to contain any of %s", snippets));
    }

    public static void assertContentContainsAll(ExtractionResult result, List<String> snippets) {
      if (snippets.isEmpty()) {
        return;
      }
      String content = result.getContent();
      String lowered = content != null ? content.toLowerCase() : "";
      boolean allMatch =
          snippets.stream().allMatch(snippet -> lowered.contains(snippet.toLowerCase()));
      assertTrue(allMatch, String.format("Expected content to contain all of %s", snippets));
    }

    public static void assertTableCount(ExtractionResult result, Integer minimum, Integer maximum) {
      List<Table> tables = result.getTables();
      int count = tables != null ? tables.size() : 0;
      if (minimum != null) {
        assertTrue(
            count >= minimum, String.format("Expected table count >= %d, got %d", minimum, count));
      }
      if (maximum != null) {
        assertTrue(
            count <= maximum, String.format("Expected table count <= %d, got %d", maximum, count));
      }
    }

    public static void assertDetectedLanguages(
        ExtractionResult result, List<String> expected, Double minConfidence) {
      if (expected.isEmpty()) {
        return;
      }
      List<String> languages = result.getDetectedLanguages();
      assertNotNull(languages, "Expected detected languages to be present");
      boolean allFound = expected.stream().allMatch(lang -> languages.contains(lang));
      assertTrue(allFound, String.format("Expected languages %s to be in %s", expected, languages));

      if (minConfidence != null) {
        Map<String, Object> metadata = result.getMetadataMap();
        if (metadata != null && metadata.containsKey("confidence")) {
          Object confObj = metadata.get("confidence");
          double confidence = confObj instanceof Number ? ((Number) confObj).doubleValue() : 0.0;
          assertTrue(
              confidence >= minConfidence,
              String.format("Expected confidence >= %f, got %f", minConfidence, confidence));
        }
      }
    }

    public static void assertMetadataExpectation(
        ExtractionResult result, String path, Map<String, Object> expectation) {
      Map<String, Object> metadata = result.getMetadataMap();
      Object value = fetchMetadataValue(metadata, path);
      assertNotNull(value, String.format("Metadata path '%s' missing", path));

      if (expectation.containsKey("eq")) {
        Object expected = expectation.get("eq");
        assertTrue(
            valuesEqual(value, expected),
            String.format("Expected %s to equal %s", value, expected));
      }

      if (expectation.containsKey("gte")) {
        Object expected = expectation.get("gte");
        double actualNum = convertNumeric(value);
        double expectedNum = convertNumeric(expected);
        assertTrue(
            actualNum >= expectedNum, String.format("Expected %f >= %f", actualNum, expectedNum));
      }

      if (expectation.containsKey("lte")) {
        Object expected = expectation.get("lte");
        double actualNum = convertNumeric(value);
        double expectedNum = convertNumeric(expected);
        assertTrue(
            actualNum <= expectedNum, String.format("Expected %f <= %f", actualNum, expectedNum));
      }

      if (expectation.containsKey("contains")) {
        Object contains = expectation.get("contains");
        if (value instanceof String && contains instanceof String) {
          assertTrue(
              ((String) value).contains((String) contains),
              String.format("Expected '%s' to contain '%s'", value, contains));
        } else if (value instanceof List && contains instanceof String) {
          // List contains a string
          @SuppressWarnings("unchecked")
          List<Object> valueList = (List<Object>) value;
          boolean found =
              valueList.stream().anyMatch(item -> item.toString().contains((String) contains));
          assertTrue(found, String.format("Expected %s to contain '%s'", value, contains));
        } else if (value instanceof List && contains instanceof List) {
          @SuppressWarnings("unchecked")
          List<Object> valueList = (List<Object>) value;
          @SuppressWarnings("unchecked")
          List<Object> containsList = (List<Object>) contains;
          boolean allContained = containsList.stream().allMatch(valueList::contains);
          assertTrue(
              allContained, String.format("Expected %s to contain all of %s", value, contains));
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
        Boolean eachHasEmbedding) {
      var chunks = result.getChunks();
      int count = chunks != null ? chunks.size() : 0;
      if (minCount != null) {
        assertTrue(
            count >= minCount,
            String.format("Expected chunk count >= %d, got %d", minCount, count));
      }
      if (maxCount != null) {
        assertTrue(
            count <= maxCount,
            String.format("Expected chunk count <= %d, got %d", maxCount, count));
      }
      if (chunks != null && eachHasContent != null && eachHasContent) {
        for (var chunk : chunks) {
          String content = chunk.getContent();
          assertTrue(content != null && !content.isEmpty(), "Expected each chunk to have content");
        }
      }
      if (chunks != null && eachHasEmbedding != null && eachHasEmbedding) {
        for (var chunk : chunks) {
          assertNotNull(chunk.getEmbedding(), "Expected each chunk to have an embedding");
        }
      }
    }

    public static void assertImages(
        ExtractionResult result, Integer minCount, Integer maxCount, List<String> formatsInclude) {
      var images = result.getImages();
      int count = images != null ? images.size() : 0;
      if (minCount != null) {
        assertTrue(
            count >= minCount,
            String.format("Expected image count >= %d, got %d", minCount, count));
      }
      if (maxCount != null) {
        assertTrue(
            count <= maxCount,
            String.format("Expected image count <= %d, got %d", maxCount, count));
      }
      if (images != null && formatsInclude != null && !formatsInclude.isEmpty()) {
        var formats = images.stream().map(img -> img.getFormat()).filter(f -> f != null).toList();
        for (String expected : formatsInclude) {
          boolean found =
              formats.stream().anyMatch(f -> f.toLowerCase().contains(expected.toLowerCase()));
          assertTrue(
              found,
              String.format("Expected image formats to include '%s', got %s", expected, formats));
        }
      }
    }

    public static void assertPages(ExtractionResult result, Integer minCount, Integer exactCount) {
      var pages = result.getPages();
      int count = pages != null ? pages.size() : 0;
      if (minCount != null) {
        assertTrue(
            count >= minCount, String.format("Expected page count >= %d, got %d", minCount, count));
      }
      if (exactCount != null) {
        assertTrue(
            count == exactCount,
            String.format("Expected exactly %d pages, got %d", exactCount, count));
      }
      if (pages != null) {
        for (var page : pages) {
          // isBlank should be accessible (Optional<Boolean>)
          var isBlank = page.getIsBlank();
          assertTrue(isBlank != null, "getIsBlank() should return non-null Optional");
        }
      }
    }

    public static void assertElements(
        ExtractionResult result, Integer minCount, List<String> typesInclude) {
      var elements = result.getElements();
      int count = elements != null ? elements.size() : 0;
      if (minCount != null) {
        assertTrue(
            count >= minCount,
            String.format("Expected element count >= %d, got %d", minCount, count));
      }
      if (elements != null && typesInclude != null && !typesInclude.isEmpty()) {
        var types =
            elements.stream()
                .map(el -> el.getElementType().wireValue())
                .filter(t -> t != null)
                .toList();
        for (String expected : typesInclude) {
          boolean found =
              types.stream().anyMatch(t -> t.toLowerCase().contains(expected.toLowerCase()));
          assertTrue(
              found,
              String.format("Expected element types to include '%s', got %s", expected, types));
        }
      }
    }

    public static void assertOcrElements(
        ExtractionResult result,
        boolean hasElements,
        boolean hasGeometry,
        Boolean hasConfidence,
        Integer minCount) {
      var ocrElements = result.getOcrElements();
      if (hasElements) {
        assertTrue(!ocrElements.isEmpty(), "Expected OCR elements, but none found");
      }
      if (hasGeometry) {
        for (int i = 0; i < ocrElements.size(); i++) {
          assertNotNull(
              ocrElements.get(i).getGeometry(),
              String.format("OCR element %d expected to have geometry", i));
        }
      }
      if (hasConfidence != null && hasConfidence) {
        for (int i = 0; i < ocrElements.size(); i++) {
          assertNotNull(
              ocrElements.get(i).getConfidence(),
              String.format("OCR element %d expected to have confidence score", i));
        }
      }
      if (minCount != null) {
        assertTrue(
            ocrElements.size() >= minCount,
            String.format(
                "Expected at least %d OCR elements, found %d", minCount, ocrElements.size()));
      }
    }

    public static void assertDocument(
        ExtractionResult result,
        boolean hasDocument,
        Integer minNodeCount,
        List<String> nodeTypesInclude,
        Boolean hasGroups) {
      var document = result.getDocumentStructure().orElse(null);
      if (hasDocument) {
        assertNotNull(document, "Expected document but got null");
        var nodes = document.getNodes();
        assertNotNull(nodes, "Expected document nodes but got null");
        if (minNodeCount != null) {
          assertTrue(
              nodes.size() >= minNodeCount,
              String.format("Expected at least %d nodes, found %d", minNodeCount, nodes.size()));
        }
        if (nodeTypesInclude != null && !nodeTypesInclude.isEmpty()) {
          var types =
              nodes.stream().map(n -> n.getContent().getNodeType()).filter(t -> t != null).toList();
          for (String expected : nodeTypesInclude) {
            boolean found =
                types.stream().anyMatch(t -> t.toLowerCase().equals(expected.toLowerCase()));
            assertTrue(
                found, String.format("Expected node type '%s' not found in %s", expected, types));
          }
        }
        if (hasGroups != null) {
          boolean hasGroupNodes =
              nodes.stream().anyMatch(n -> "group".equals(n.getContent().getNodeType()));
          assertTrue(
              hasGroupNodes == hasGroups,
              String.format("Expected hasGroups=%b but got %b", hasGroups, hasGroupNodes));
        }
      } else {
        assertTrue(
            document == null, String.format("Expected document to be null but got %s", document));
      }
    }

    public static void assertKeywords(
        ExtractionResult result, Boolean hasKeywords, Integer minCount, Integer maxCount) {
      var keywordsOpt = result.getExtractedKeywords();
      var keywords = keywordsOpt.orElse(null);
      int count = keywords != null ? keywords.size() : 0;

      if (hasKeywords != null && hasKeywords) {
        assertTrue(keywords != null && !keywords.isEmpty(), "Expected keywords to be present");
      }
      if (hasKeywords != null && !hasKeywords) {
        assertTrue(
            keywords == null || keywords.isEmpty(),
            String.format("Expected no keywords but found %d", count));
      }

      if (minCount != null) {
        assertTrue(
            count >= minCount,
            String.format("Expected keyword count >= %d, got %d", minCount, count));
      }
      if (maxCount != null) {
        assertTrue(
            count <= maxCount,
            String.format("Expected keyword count <= %d, got %d", maxCount, count));
      }
    }

    public static void assertContentNotEmpty(ExtractionResult result) {
      String content = result.getContent();
      assertTrue(content != null && !content.isEmpty(), "Expected content to be non-empty");
    }

    public static void assertTableBoundingBoxes(ExtractionResult result) {
      var tables = result.getTables();
      if (tables != null) {
        for (int i = 0; i < tables.size(); i++) {
          assertNotNull(
              tables.get(i).boundingBox(),
              String.format("Table %d expected to have bounding box", i));
        }
      }
    }

    public static void assertTableContentContainsAny(
        ExtractionResult result, List<String> snippets) {
      if (snippets.isEmpty()) return;
      var tables = result.getTables();
      StringBuilder allContent = new StringBuilder();
      if (tables != null) {
        for (var table : tables) {
          allContent
              .append(table.markdown() != null ? table.markdown().toLowerCase() : "")
              .append(" ");
        }
      }
      String combined = allContent.toString();
      boolean found =
          snippets.stream().anyMatch(snippet -> combined.contains(snippet.toLowerCase()));
      assertTrue(found, String.format("Expected table content to contain any of %s", snippets));
    }

    public static void assertImageBoundingBoxes(ExtractionResult result) {
      var images = result.getImages();
      if (images != null) {
        for (int i = 0; i < images.size(); i++) {
          assertNotNull(
              images.get(i).getBoundingBox(),
              String.format("Image %d expected to have bounding box", i));
        }
      }
    }

    public static void assertQualityScore(
        ExtractionResult result, Boolean hasScore, Double minScore, Double maxScore) {
      if (hasScore != null && hasScore) {
        assertNotNull(result.getQualityScore(), "Expected quality score to be present");
      }
      Double score = result.getQualityScore().orElse(null);
      if (minScore != null && score != null) {
        assertTrue(
            score >= minScore,
            String.format("Expected quality score >= %f, got %f", minScore, score));
      }
      if (maxScore != null && score != null) {
        assertTrue(
            score <= maxScore,
            String.format("Expected quality score <= %f, got %f", maxScore, score));
      }
    }

    public static void assertProcessingWarnings(
        ExtractionResult result, Integer maxCount, Boolean isEmpty) {
      var warnings = result.getProcessingWarnings().orElse(null);
      int count = warnings != null ? warnings.size() : 0;
      if (isEmpty != null && isEmpty) {
        assertTrue(
            count == 0, String.format("Expected processing warnings to be empty, got %d", count));
      }
      if (maxCount != null) {
        assertTrue(
            count <= maxCount,
            String.format("Expected at most %d processing warnings, got %d", maxCount, count));
      }
    }

    public static void assertDjotContent(
        ExtractionResult result, Boolean hasContent, Integer minBlocks) {
      var djotContent = result.getDjotContent().orElse(null);
      if (hasContent != null && hasContent) {
        assertTrue(
            djotContent != null && !djotContent.getPlainText().isEmpty(),
            "Expected djot content to be present");
      }
      if (minBlocks != null && djotContent != null) {
        int blockCount = djotContent.getBlocks() != null ? djotContent.getBlocks().size() : 0;
        assertTrue(
            blockCount >= minBlocks,
            String.format("Expected at least %d djot blocks, got %d", minBlocks, blockCount));
      }
    }

    public static void assertAnnotations(
        ExtractionResult result, Boolean hasAnnotations, Integer minCount) {
      var annotations = result.getAnnotations().orElse(null);
      if (hasAnnotations != null && hasAnnotations) {
        assertTrue(
            annotations != null && !annotations.isEmpty(),
            "Expected annotations to be present and non-empty");
      }
      if (annotations != null && minCount != null) {
        assertTrue(
            annotations.size() >= minCount,
            String.format(
                "Expected at least %d annotations, got %d", minCount, annotations.size()));
      }
    }
  }
}
