package e2e

import (
	"context"
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"
	"strings"
	"testing"
	"unicode"

	kreuzberg "github.com/kreuzberg-dev/kreuzberg/packages/go/v4"
)

var (
	workspaceRoot = func() string {
		wd, err := os.Getwd()
		if err != nil {
			panic(fmt.Sprintf("failed to determine working directory: %v", err))
		}
		root := filepath.Clean(filepath.Join(wd, "..", ".."))
		abs, err := filepath.Abs(root)
		if err != nil {
			panic(fmt.Sprintf("failed to resolve workspace root: %v", err))
		}
		return abs
	}()
	testDocuments = filepath.Join(workspaceRoot, "test_documents")
)

func resolveDocument(relative string) string {
	return filepath.Join(testDocuments, filepath.FromSlash(relative))
}

func ensureDocument(t *testing.T, relative string, skipIfMissing bool) string {
	t.Helper()
	path := resolveDocument(relative)
	if _, err := os.Stat(path); err != nil {
		if skipIfMissing && os.IsNotExist(err) {
			t.Skipf("Skipping %s: missing document at %s", relative, path)
		}
		t.Fatalf("document %s unavailable: %v", path, err)
	}
	return path
}

func buildConfig(t *testing.T, raw []byte) *kreuzberg.ExtractionConfig {
	t.Helper()
	if len(raw) == 0 {
		return nil
	}
	var cfg kreuzberg.ExtractionConfig
	if err := json.Unmarshal(raw, &cfg); err != nil {
		t.Fatalf("failed to decode extraction config: %v", err)
	}
	return &cfg
}

func shouldSkipMissingDependency(err error) bool {
	if err == nil {
		return false
	}
	message := strings.Map(func(r rune) rune {
		if unicode.IsSpace(r) {
			return ' '
		}
		return r
	}, strings.ToLower(err.Error()))

	return strings.Contains(message, "missing dependency")
}

func runExtraction(t *testing.T, relativePath string, configJSON []byte) *kreuzberg.ExtractionResult {
	t.Helper()
	documentPath := ensureDocument(t, relativePath, true)
	config := buildConfig(t, configJSON)
	result, err := kreuzberg.ExtractFileSync(documentPath, config)
	if err != nil {
		if shouldSkipMissingDependency(err) {
			t.Skipf("Skipping %s: dependency unavailable (%v)", relativePath, err)
		}
		t.Fatalf("extractFileSync(%s) failed: %v", documentPath, err)
	}
	return result
}

func assertExpectedMime(t *testing.T, result *kreuzberg.ExtractionResult, expected []string) {
	t.Helper()
	if len(expected) == 0 {
		return
	}
	for _, token := range expected {
		if strings.Contains(strings.ToLower(result.MimeType), strings.ToLower(token)) {
			return
		}
	}
	t.Fatalf("expected MIME %q to include one of %v", result.MimeType, expected)
}

func assertMinContentLength(t *testing.T, result *kreuzberg.ExtractionResult, minimum int) {
	t.Helper()
	if len(result.Content) < minimum {
		t.Fatalf("expected content length >= %d, got %d", minimum, len(result.Content))
	}
}

func assertMaxContentLength(t *testing.T, result *kreuzberg.ExtractionResult, maximum int) {
	t.Helper()
	if len(result.Content) > maximum {
		t.Fatalf("expected content length <= %d, got %d", maximum, len(result.Content))
	}
}

func assertContentContainsAny(t *testing.T, result *kreuzberg.ExtractionResult, snippets []string) {
	t.Helper()
	if len(snippets) == 0 {
		return
	}
	lowered := strings.ToLower(result.Content)
	for _, snippet := range snippets {
		if strings.Contains(lowered, strings.ToLower(snippet)) {
			return
		}
	}
	t.Fatalf("expected content to contain any of %v", snippets)
}

func assertContentContainsAll(t *testing.T, result *kreuzberg.ExtractionResult, snippets []string) {
	t.Helper()
	if len(snippets) == 0 {
		return
	}
	lowered := strings.ToLower(result.Content)
	missing := make([]string, 0)
	for _, snippet := range snippets {
		if !strings.Contains(lowered, strings.ToLower(snippet)) {
			missing = append(missing, snippet)
		}
	}
	if len(missing) > 0 {
		t.Fatalf("expected content to contain all snippets %v, missing %v", snippets, missing)
	}
}

func assertTableCount(t *testing.T, result *kreuzberg.ExtractionResult, min, max *int) {
	t.Helper()
	count := len(result.Tables)
	if min != nil && count < *min {
		t.Fatalf("expected at least %d tables, found %d", *min, count)
	}
	if max != nil && count > *max {
		t.Fatalf("expected at most %d tables, found %d", *max, count)
	}
}

func assertDetectedLanguages(t *testing.T, result *kreuzberg.ExtractionResult, expected []string, minConfidence *float64) {
	t.Helper()
	if len(expected) == 0 {
		return
	}
	langs := result.DetectedLanguages
	if len(langs) == 0 {
		t.Fatalf("expected detected languages %v but field is empty", expected)
	}
	missing := make([]string, 0)
	for _, lang := range expected {
		found := false
		for _, candidate := range langs {
			if strings.EqualFold(candidate, lang) {
				found = true
				break
			}
		}
		if !found {
			missing = append(missing, lang)
		}
	}
	if len(missing) > 0 {
		t.Fatalf("expected languages %v, missing %v", expected, missing)
	}

	if minConfidence != nil {
		metadata := metadataAsMap(t, result.Metadata)
		if value, ok := lookupMetadataValue(metadata, "confidence").(float64); ok {
			if value < *minConfidence {
				t.Fatalf("expected confidence >= %f, got %f", *minConfidence, value)
			}
		}
	}
}

func metadataAsMap(t *testing.T, metadata kreuzberg.Metadata) map[string]any {
	t.Helper()
	bytes, err := json.Marshal(metadata)
	if err != nil {
		t.Fatalf("failed to encode metadata: %v", err)
	}
	var out map[string]any
	if err := json.Unmarshal(bytes, &out); err != nil {
		t.Fatalf("failed to decode metadata map: %v", err)
	}
	return out
}

func lookupMetadataValue(metadata map[string]any, path string) any {
	if value := lookupMetadataPath(metadata, path); value != nil {
		return value
	}
	if format, ok := metadata["format"].(map[string]any); ok {
		return lookupMetadataPath(format, path)
	}
	return nil
}

func lookupMetadataPath(metadata map[string]any, path string) any {
	current := any(metadata)
	for _, segment := range strings.Split(path, ".") {
		asMap, ok := current.(map[string]any)
		if !ok {
			return nil
		}
		value, exists := asMap[segment]
		if !exists {
			return nil
		}
		current = value
	}
	return current
}

func intPtr(value int) *int {
	return &value
}

func floatPtr(value float64) *float64 {
	return &value
}

func boolPtr(value bool) *bool {
	return &value
}

func assertChunks(t *testing.T, result *kreuzberg.ExtractionResult, minCount, maxCount *int, eachHasContent, eachHasEmbedding *bool) {
	t.Helper()
	count := len(result.Chunks)
	if minCount != nil && count < *minCount {
		t.Fatalf("expected at least %d chunks, found %d", *minCount, count)
	}
	if maxCount != nil && count > *maxCount {
		t.Fatalf("expected at most %d chunks, found %d", *maxCount, count)
	}
	if eachHasContent != nil && *eachHasContent {
		for i, chunk := range result.Chunks {
			if len(chunk.Content) == 0 {
				t.Fatalf("chunk %d has empty content", i)
			}
		}
	}
	if eachHasEmbedding != nil && *eachHasEmbedding {
		for i, chunk := range result.Chunks {
			if len(chunk.Embedding) == 0 {
				t.Fatalf("chunk %d has no embedding", i)
			}
		}
	}
}

func assertImages(t *testing.T, result *kreuzberg.ExtractionResult, minCount, maxCount *int, formatsInclude []string) {
	t.Helper()
	count := len(result.Images)
	if minCount != nil && count < *minCount {
		t.Fatalf("expected at least %d images, found %d", *minCount, count)
	}
	if maxCount != nil && count > *maxCount {
		t.Fatalf("expected at most %d images, found %d", *maxCount, count)
	}
	if len(formatsInclude) > 0 {
		formats := make(map[string]bool)
		for _, img := range result.Images {
			formats[strings.ToLower(img.Format)] = true
		}
		for _, expected := range formatsInclude {
			if !formats[strings.ToLower(expected)] {
				t.Fatalf("expected image format %q not found in results", expected)
			}
		}
	}
}

func assertPages(t *testing.T, result *kreuzberg.ExtractionResult, minCount, exactCount *int) {
	t.Helper()
	count := len(result.Pages)
	if minCount != nil && count < *minCount {
		t.Fatalf("expected at least %d pages, found %d", *minCount, count)
	}
	if exactCount != nil && count != *exactCount {
		t.Fatalf("expected exactly %d pages, found %d", *exactCount, count)
	}
	for i, page := range result.Pages {
		if page.IsBlank != nil {
			_ = *page.IsBlank // validate it's a valid bool pointer
		}
		_ = i
	}
}

func assertElements(t *testing.T, result *kreuzberg.ExtractionResult, minCount *int, typesInclude []string) {
	t.Helper()
	count := len(result.Elements)
	if minCount != nil && count < *minCount {
		t.Fatalf("expected at least %d elements, found %d", *minCount, count)
	}
	if len(typesInclude) > 0 {
		types := make(map[string]bool)
		for _, elem := range result.Elements {
			types[strings.ToLower(string(elem.ElementType))] = true
		}
		for _, expected := range typesInclude {
			if !types[strings.ToLower(expected)] {
				t.Fatalf("expected element type %q not found in results", expected)
			}
		}
	}
}

func assertOcrElements(t *testing.T, result *kreuzberg.ExtractionResult, hasElements, hasGeometry, hasConfidence *bool, minCount *int) {
	t.Helper()
	if hasElements != nil && *hasElements {
		if len(result.OcrElements) == 0 {
			t.Fatalf("expected OCR elements, but none found")
		}
	}
	if hasGeometry != nil && *hasGeometry {
		for i, elem := range result.OcrElements {
			if elem.Geometry == nil {
				t.Fatalf("OCR element %d expected to have geometry", i)
			}
		}
	}
	if hasConfidence != nil && *hasConfidence {
		for i, elem := range result.OcrElements {
			if elem.Confidence == nil {
				t.Fatalf("OCR element %d expected to have confidence score", i)
			}
		}
	}
	if minCount != nil && len(result.OcrElements) < *minCount {
		t.Fatalf("expected at least %d OCR elements, found %d", *minCount, len(result.OcrElements))
	}
}

func skipIfFeatureUnavailable(t *testing.T, feature string) {
	t.Helper()
	envVar := "KREUZBERG_" + strings.ToUpper(strings.ReplaceAll(feature, "-", "_")) + "_AVAILABLE"
	flag := os.Getenv(envVar)
	if flag == "" || flag == "0" || strings.EqualFold(flag, "false") {
		t.Skipf("Skipping: feature %q not available (set %s=1)", feature, envVar)
	}
}

func assertDocument(t *testing.T, result *kreuzberg.ExtractionResult, hasDocument bool, minNodeCount *int, nodeTypesInclude []string, hasGroups *bool) {
	t.Helper()
	doc := result.Document
	if hasDocument {
		if doc == nil {
			t.Fatal("Expected document but got nil")
		}
		nodes := doc.Nodes
		if nodes == nil {
			t.Fatal("Expected document nodes but got nil")
		}
		if minNodeCount != nil && len(nodes) < *minNodeCount {
			t.Fatalf("Expected at least %d nodes, found %d", *minNodeCount, len(nodes))
		}
		if len(nodeTypesInclude) > 0 {
			types := make(map[string]bool)
			for _, n := range nodes {
				if n.Content.NodeType != "" {
					types[strings.ToLower(n.Content.NodeType)] = true
				}
			}
			for _, expected := range nodeTypesInclude {
				if !types[strings.ToLower(expected)] {
					t.Fatalf("Expected node type %q not found in document", expected)
				}
			}
		}
		if hasGroups != nil {
			hasGroupNodes := false
			for _, n := range nodes {
				if n.Content.NodeType != "" && strings.EqualFold(n.Content.NodeType, "group") {
					hasGroupNodes = true
					break
				}
			}
			if hasGroupNodes != *hasGroups {
				t.Fatalf("Expected hasGroups=%v but got %v", *hasGroups, hasGroupNodes)
			}
		}
	} else if doc != nil {
		t.Fatal("Expected document to be nil but got a document")
	}
}

func runExtractionBytes(t *testing.T, relativePath string, configJSON []byte) *kreuzberg.ExtractionResult {
	t.Helper()
	documentPath := ensureDocument(t, relativePath, true)
	config := buildConfig(t, configJSON)
	data, err := os.ReadFile(documentPath)
	if err != nil {
		t.Fatalf("failed to read document %s: %v", documentPath, err)
	}
	// Detect MIME type from file path
	mimeType, err := kreuzberg.DetectMimeTypeFromPath(documentPath)
	if err != nil {
		t.Fatalf("failed to detect MIME type for %s: %v", documentPath, err)
	}
	result, err := kreuzberg.ExtractBytesSync(data, mimeType, config)
	if err != nil {
		if shouldSkipMissingDependency(err) {
			t.Skipf("Skipping %s: dependency unavailable (%v)", relativePath, err)
		}
		t.Fatalf("extractBytesSync(%s) failed: %v", documentPath, err)
	}
	return result
}

func runExtractionAsync(t *testing.T, relativePath string, configJSON []byte) *kreuzberg.ExtractionResult {
	t.Helper()
	documentPath := ensureDocument(t, relativePath, true)
	config := buildConfig(t, configJSON)
	// Note: Go SDK doesn't have true async - use sync version with context
	result, err := kreuzberg.ExtractFileWithContext(context.Background(), documentPath, config)
	if err != nil {
		if shouldSkipMissingDependency(err) {
			t.Skipf("Skipping %s: dependency unavailable (%v)", relativePath, err)
		}
		t.Fatalf("extractFileWithContext(%s) failed: %v", documentPath, err)
	}
	return result
}

func runExtractionBytesAsync(t *testing.T, relativePath string, configJSON []byte) *kreuzberg.ExtractionResult {
	t.Helper()
	documentPath := ensureDocument(t, relativePath, true)
	config := buildConfig(t, configJSON)
	data, err := os.ReadFile(documentPath)
	if err != nil {
		t.Fatalf("failed to read document %s: %v", documentPath, err)
	}
	// Detect MIME type from file path
	mimeType, err := kreuzberg.DetectMimeTypeFromPath(documentPath)
	if err != nil {
		t.Fatalf("failed to detect MIME type for %s: %v", documentPath, err)
	}
	// Note: Go SDK doesn't have true async - use sync version with context
	result, err := kreuzberg.ExtractBytesWithContext(context.Background(), data, mimeType, config)
	if err != nil {
		if shouldSkipMissingDependency(err) {
			t.Skipf("Skipping %s: dependency unavailable (%v)", relativePath, err)
		}
		t.Fatalf("extractBytesWithContext(%s) failed: %v", documentPath, err)
	}
	return result
}

func runBatchExtraction(t *testing.T, relativePaths []string, configJSON []byte) []*kreuzberg.ExtractionResult {
	t.Helper()
	var documentPaths []string
	for _, rel := range relativePaths {
		documentPaths = append(documentPaths, ensureDocument(t, rel, true))
	}
	config := buildConfig(t, configJSON)
	results, err := kreuzberg.BatchExtractFilesSync(documentPaths, config)
	if err != nil {
		if shouldSkipMissingDependency(err) {
			t.Skipf("Skipping batch: dependency unavailable (%v)", err)
		}
		t.Fatalf("batchExtractFilesSync failed: %v", err)
	}
	return results
}

func runBatchExtractionAsync(t *testing.T, relativePaths []string, configJSON []byte) []*kreuzberg.ExtractionResult {
	t.Helper()
	var documentPaths []string
	for _, rel := range relativePaths {
		documentPaths = append(documentPaths, ensureDocument(t, rel, true))
	}
	config := buildConfig(t, configJSON)
	// Note: Go SDK doesn't have true async - use sync version with context
	results, err := kreuzberg.BatchExtractFilesWithContext(context.Background(), documentPaths, config)
	if err != nil {
		if shouldSkipMissingDependency(err) {
			t.Skipf("Skipping batch: dependency unavailable (%v)", err)
		}
		t.Fatalf("batchExtractFilesWithContext failed: %v", err)
	}
	return results
}

func assertKeywords(t *testing.T, result *kreuzberg.ExtractionResult, hasKeywords *bool, minCount, maxCount *int) {
	t.Helper()
	if hasKeywords != nil {
		if *hasKeywords {
			if len(result.ExtractedKeywords) == 0 {
				t.Fatalf("expected keywords in result but ExtractedKeywords field is nil or empty")
			}
		} else {
			if len(result.ExtractedKeywords) > 0 {
				t.Fatalf("expected no keywords but found %d", len(result.ExtractedKeywords))
			}
		}
	}
	count := len(result.ExtractedKeywords)
	if minCount != nil && count < *minCount {
		t.Fatalf("expected at least %d keywords, found %d", *minCount, count)
	}
	if maxCount != nil && count > *maxCount {
		t.Fatalf("expected at most %d keywords, found %d", *maxCount, count)
	}
}

func assertContentNotEmpty(t *testing.T, result *kreuzberg.ExtractionResult) {
	t.Helper()
	if len(result.Content) == 0 {
		t.Fatalf("expected content to be non-empty, but it is empty")
	}
}

func assertTableBoundingBoxes(t *testing.T, result *kreuzberg.ExtractionResult) {
	t.Helper()
	for i, table := range result.Tables {
		if table.BoundingBox == nil {
			t.Fatalf("table %d expected to have bounding box", i)
		}
	}
}

//nolint:unused // referenced by generated tests when fixtures use table content assertions
func assertTableContentContainsAny(t *testing.T, result *kreuzberg.ExtractionResult, snippets []string) {
	t.Helper()
	if len(snippets) == 0 {
		return
	}
	var allContent string
	for _, table := range result.Tables {
		allContent += strings.ToLower(table.Markdown) + " "
	}
	for _, snippet := range snippets {
		if strings.Contains(allContent, strings.ToLower(snippet)) {
			return
		}
	}
	t.Fatalf("expected table content to contain any of %v", snippets)
}

//nolint:unused // referenced by generated tests when fixtures use image bounding box assertions
func assertImageBoundingBoxes(t *testing.T, result *kreuzberg.ExtractionResult) {
	t.Helper()
	for i, img := range result.Images {
		if img.BoundingBox == nil {
			t.Fatalf("image %d expected to have bounding box", i)
		}
	}
}

func assertQualityScore(t *testing.T, result *kreuzberg.ExtractionResult, hasScore *bool, minScore, maxScore *float64) {
	t.Helper()
	if hasScore != nil && *hasScore {
		if result.QualityScore == nil {
			t.Fatalf("expected quality score to be present")
		}
	}
	if minScore != nil && result.QualityScore != nil {
		if *result.QualityScore < *minScore {
			t.Fatalf("expected quality score >= %f, got %f", *minScore, *result.QualityScore)
		}
	}
	if maxScore != nil && result.QualityScore != nil {
		if *result.QualityScore > *maxScore {
			t.Fatalf("expected quality score <= %f, got %f", *maxScore, *result.QualityScore)
		}
	}
}

//nolint:unused // referenced by generated tests when fixtures use processing warning assertions
func assertProcessingWarnings(t *testing.T, result *kreuzberg.ExtractionResult, maxCount *int, isEmpty *bool) {
	t.Helper()
	warnings := result.ProcessingWarnings
	if isEmpty != nil && *isEmpty {
		if len(warnings) != 0 {
			t.Fatalf("expected processing warnings to be empty, got %d", len(warnings))
		}
	}
	if maxCount != nil && len(warnings) > *maxCount {
		t.Fatalf("expected at most %d processing warnings, got %d", *maxCount, len(warnings))
	}
}

//nolint:unused // referenced by generated tests when fixtures use djot content assertions
func assertDjotContent(t *testing.T, result *kreuzberg.ExtractionResult, hasContent *bool, minBlocks *int) {
	t.Helper()
	if hasContent != nil && *hasContent {
		if result.DjotContent == nil || result.DjotContent.PlainText == "" {
			t.Fatalf("expected djot content to be present")
		}
	}
	if minBlocks != nil && result.DjotContent != nil {
		blockCount := len(result.DjotContent.Blocks)
		if blockCount < *minBlocks {
			t.Fatalf("expected at least %d djot blocks, got %d", *minBlocks, blockCount)
		}
	}
}

func assertAnnotations(t *testing.T, result *kreuzberg.ExtractionResult, hasAnnotations bool, minCount *int) {
	t.Helper()
	if hasAnnotations {
		if len(result.Annotations) == 0 {
			t.Fatalf("expected annotations to be present and non-empty")
		}
	}
	if minCount != nil {
		if len(result.Annotations) < *minCount {
			t.Fatalf("expected at least %d annotations, got %d", *minCount, len(result.Annotations))
		}
	}
}
