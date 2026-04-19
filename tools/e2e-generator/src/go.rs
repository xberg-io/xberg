use crate::fixtures::{Assertions, ExtractionMethod, Fixture, GenerationMode, InputType, RenderAssertions};
use crate::parity::{self, ParityManifest, TypeDef};
use anyhow::{Context, Result};
use camino::Utf8Path;
use itertools::Itertools;
use serde_json::{Map, Value};
use std::fmt::Write as _;
use std::fs;

const GO_HELPERS_TEMPLATE: &str = r##"package e2e

import (
	"context"
	"encoding/json"
	"fmt"
	"math"
	"os"
	"path/filepath"
	"strings"
	"testing"
	"unicode"

	"github.com/kreuzberg-dev/kreuzberg/packages/go/v4"
)

var (
	workspaceRoot = func() string {
		wd, err := os.Getwd()
		if err != nil {
			panic(fmt.Sprintf("failed to determine working directory: %v", err))
		}
		dir := wd
		for {
			if info, err := os.Stat(filepath.Join(dir, "test_documents")); err == nil && info.IsDir() {
				return dir
			}
			parent := filepath.Dir(dir)
			if parent == dir {
				panic("could not find workspace root (directory containing test_documents/)")
			}
			dir = parent
		}
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

func assertContentContainsNone(t *testing.T, result *kreuzberg.ExtractionResult, snippets []string) {
	t.Helper()
	if len(snippets) == 0 {
		return
	}
	lowered := strings.ToLower(result.Content)
	found := make([]string, 0)
	for _, snippet := range snippets {
		if strings.Contains(lowered, strings.ToLower(snippet)) {
			found = append(found, snippet)
		}
	}
	if len(found) > 0 {
		t.Fatalf("expected content to contain none of %v, but found %v", snippets, found)
	}
}

func assertTableCount(t *testing.T, result *kreuzberg.ExtractionResult, minVal, maxVal *int) {
	t.Helper()
	count := len(result.Tables)
	if minVal != nil && count < *minVal {
		t.Fatalf("expected at least %d tables, found %d", *minVal, count)
	}
	if maxVal != nil && count > *maxVal {
		t.Fatalf("expected at most %d tables, found %d", *maxVal, count)
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

func assertChunks(t *testing.T, result *kreuzberg.ExtractionResult, minCount, maxCount *int, eachHasContent, eachHasEmbedding, eachHasHeadingContext, eachHasChunkType, contentStartsWithHeading *bool) {
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
	if eachHasHeadingContext != nil && *eachHasHeadingContext {
		for i, chunk := range result.Chunks {
			if chunk.Metadata.HeadingContext == nil {
				t.Fatalf("chunk %d has no heading_context", i)
			}
		}
	}
	if eachHasHeadingContext != nil && !*eachHasHeadingContext {
		for i, chunk := range result.Chunks {
			if chunk.Metadata.HeadingContext != nil {
				t.Fatalf("chunk %d should have no heading_context", i)
			}
		}
	}
	if eachHasChunkType != nil && *eachHasChunkType {
		for i, chunk := range result.Chunks {
			if chunk.ChunkType == "" || chunk.ChunkType == "unknown" {
				t.Fatalf("chunk %d has no specific chunk_type, got %q", i, chunk.ChunkType)
			}
		}
	}
	if contentStartsWithHeading != nil && *contentStartsWithHeading {
		for i, chunk := range result.Chunks {
			if chunk.Metadata.HeadingContext == nil {
				continue
			}
			if !strings.HasPrefix(chunk.Content, "#") {
				t.Fatalf("chunk %d content does not start with '#'", i)
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

func assertPages(t *testing.T, result *kreuzberg.ExtractionResult, minCount, exactCount *int, hasLayoutRegions *bool, layoutClassesInclude []string) {
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

	if hasLayoutRegions != nil && *hasLayoutRegions {
		foundLayoutRegions := false
		for _, page := range result.Pages {
			if len(page.LayoutRegions) > 0 {
				foundLayoutRegions = true
				break
			}
		}
		if !foundLayoutRegions {
			t.Fatal("expected at least one page to have layout_regions populated")
		}
	}

	if len(layoutClassesInclude) > 0 {
		allClasses := make(map[string]bool)
		for _, page := range result.Pages {
			if page.LayoutRegions != nil {
				for _, region := range page.LayoutRegions {
					allClasses[region.Class] = true
				}
			}
		}
		for _, expectedClass := range layoutClassesInclude {
			if !allClasses[expectedClass] {
				t.Fatalf("expected layout class %q not found in collected classes", expectedClass)
			}
		}
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
	envVar := "KREUZBERG_" + strings.ToUpper(strings.ReplaceAll(feature, "-", "_")) + "_DISABLED"
	flag := os.Getenv(envVar)
	if flag == "1" || strings.EqualFold(flag, "true") {
		t.Skipf("Skipping: feature %q disabled (via %s=1)", feature, envVar)
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

//nolint:unused // referenced by generated tests when fixtures use llm usage assertions
func assertLlmUsage(t *testing.T, result *kreuzberg.ExtractionResult, maxCount *int, isEmpty *bool) {
	t.Helper()
	usage := result.LlmUsage
	if usage == nil {
		usage = []kreuzberg.LlmUsage{}
	}
	if isEmpty != nil && *isEmpty {
		if len(usage) != 0 {
			t.Fatalf("expected llm usage to be empty, got %d", len(usage))
		}
	}
	if maxCount != nil && len(usage) > *maxCount {
		t.Fatalf("expected at most %d llm usage entries, got %d", *maxCount, len(usage))
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

func assertIsPng(t *testing.T, data []byte) {
	t.Helper()
	if len(data) < 4 {
		t.Fatalf("data too short for PNG: %d bytes", len(data))
	}
	if data[0] != 0x89 || data[1] != 0x50 || data[2] != 0x4E || data[3] != 0x47 {
		t.Fatalf("missing PNG magic bytes, got: %v", data[:4])
	}
}

func assertMinByteLength(t *testing.T, data []byte, minLen int) {
	t.Helper()
	if len(data) < minLen {
		t.Fatalf("expected at least %d bytes, got %d", minLen, len(data))
	}
}

func assertEmbedResult(t *testing.T, result [][]float32, count *int, dims *int, noNan, noInf, nonZero, normalized bool) {
	t.Helper()
	if count != nil && len(result) != *count {
		t.Fatalf("expected %d embeddings, got %d", *count, len(result))
	}
	for i, vec := range result {
		if dims != nil && len(vec) != *dims {
			t.Fatalf("embedding %d: expected %d dims, got %d", i, *dims, len(vec))
		}
		if noNan {
			for _, v := range vec {
				if v != v { // NaN check
					t.Fatalf("embedding %d contains NaN", i)
				}
			}
		}
		if noInf {
			for _, v := range vec {
				if math.IsInf(float64(v), 0) {
					t.Fatalf("embedding %d contains Inf", i)
				}
			}
		}
		if nonZero {
			hasNonZero := false
			for _, v := range vec {
				if v != 0 {
					hasNonZero = true
					break
				}
			}
			if !hasNonZero {
				t.Fatalf("embedding %d is all zeros", i)
			}
		}
		if normalized {
			var sqSum float64
			for _, v := range vec {
				sqSum += float64(v * v)
			}
			// Approximate sqrt by checking bounds of sqSum
			if sqSum < 0.999 || sqSum > 1.001 {
				t.Fatalf("embedding %d squared sum is %f (not normalized)", i, sqSum)
			}
		}
	}
}

func assertStructuredOutput(t *testing.T, result *kreuzberg.ExtractionResult, hasOutput *bool, validatesSchema *bool, fieldExists []string) {
	t.Helper()
	output := result.StructuredOutput
	if hasOutput != nil && *hasOutput {
		if output == nil {
			t.Fatalf("expected structured_output to be present")
		}
	}
	if hasOutput != nil && !*hasOutput {
		if output != nil {
			t.Fatalf("expected structured_output to be absent")
		}
	}
	if validatesSchema != nil && *validatesSchema {
		if output == nil {
			t.Fatalf("structured_output required for validates_schema")
		}
	}
	if fieldExists != nil {
		if output == nil {
			t.Fatalf("structured_output required for field_exists")
		}
		outputMap, ok := output.(map[string]interface{})
		if !ok {
			t.Fatalf("structured_output must be a map for field_exists")
		}
		for _, field := range fieldExists {
			if _, exists := outputMap[field]; !exists {
				t.Fatalf("expected structured_output to contain '%s'", field)
			}
		}
	}
}
"##;

pub fn generate(fixtures: &[Fixture], output_root: &Utf8Path, mode: &GenerationMode) -> Result<()> {
    let go_root = output_root.join("go");
    fs::create_dir_all(&go_root).context("failed to create go e2e directory")?;

    write_go_mod(&go_root, mode)?;
    clean_tests(&go_root)?;
    write_helpers(&go_root)?;

    let doc_fixtures: Vec<_> = fixtures.iter().filter(|f| f.is_document_extraction()).collect();

    let plugin_fixtures: Vec<_> = fixtures.iter().filter(|f| f.is_plugin_api()).collect();

    let mut grouped = doc_fixtures
        .into_iter()
        .into_group_map_by(|fixture| fixture.category().to_string())
        .into_iter()
        .collect::<Vec<_>>();
    grouped.sort_by(|a, b| a.0.cmp(&b.0));

    for (category, mut fixtures) in grouped {
        fixtures.sort_by(|a, b| a.id.cmp(&b.id));
        let filename = format!("{}_test.go", category.to_lowercase());
        let content = render_category(&category, &fixtures)?;
        fs::write(go_root.join(&filename), content)
            .with_context(|| format!("failed to write Go test file {filename}"))?;
    }

    if !plugin_fixtures.is_empty() {
        generate_plugin_api_tests(&go_root, &plugin_fixtures)?;
    }

    let render_fixtures: Vec<_> = fixtures.iter().filter(|f| f.is_render()).collect();
    if !render_fixtures.is_empty() {
        let mut sorted = render_fixtures;
        sorted.sort_by(|a, b| a.id.cmp(&b.id));
        let content = render_render_category(&sorted)?;
        fs::write(go_root.join("render_test.go"), content).context("Failed to write Go render test file")?;
    }

    write_scripts(&go_root, mode)?;

    let embed_fixtures: Vec<_> = fixtures.iter().filter(|f| f.is_embed()).collect();
    if !embed_fixtures.is_empty() {
        let mut sorted = embed_fixtures;
        sorted.sort_by(|a, b| a.id.cmp(&b.id));
        let content = generate_embed_tests_go(&sorted)?;
        fs::write(go_root.join("embed_test.go"), content).context("Failed to write Go embed test file")?;
    }

    Ok(())
}

fn write_scripts(go_root: &Utf8Path, mode: &GenerationMode) -> Result<()> {
    if !mode.is_published() {
        return Ok(());
    }
    let setup = go_root.join("setup.sh");
    fs::write(
        &setup,
        r#"#!/usr/bin/env bash
set -euo pipefail
echo "Setting up Go test app..."

# Download pre-built FFI binaries from GitHub releases.
# The install command places libraries under ~/.kreuzberg/lib/<platform>/
# and a header under ~/.kreuzberg/include/.
GOWORK=off go run github.com/kreuzberg-dev/kreuzberg/packages/go/v4/cmd/install@latest

# Remove the cgo_flags.go generated in CWD — it has package kreuzberg
# which conflicts with our package e2e. We use CGO env vars instead.
rm -f cgo_flags.go

GOWORK=off go mod tidy
echo "Setup complete."
"#,
    )
    .context("Failed to write setup.sh")?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&setup, fs::Permissions::from_mode(0o755))?;
    }

    let run = go_root.join("run_tests.sh");
    fs::write(
        &run,
        r#"#!/usr/bin/env bash
set -euo pipefail

KREUZBERG_HOME="${HOME}/.kreuzberg"
ARCH="$(uname -m)"
OS="$(uname -s)"

case "${OS}" in
  Darwin)
    case "${ARCH}" in
      arm64) PLATFORM="darwin_arm64" ;;
      x86_64) PLATFORM="darwin_amd64" ;;
    esac
    LDFLAGS="${KREUZBERG_HOME}/lib/${PLATFORM}/libkreuzberg_ffi.a -framework CoreFoundation -framework CoreServices -framework SystemConfiguration -framework Security -framework Foundation -lc++"
    ;;
  Linux)
    case "${ARCH}" in
      aarch64) PLATFORM="linux_arm64" ;;
      x86_64) PLATFORM="linux_amd64" ;;
    esac
    LDFLAGS="-L${KREUZBERG_HOME}/lib/${PLATFORM} -Wl,-Bstatic -lkreuzberg_ffi -Wl,-Bdynamic -lpthread -ldl -lm -lstdc++"
    ;;
  *)
    echo "Unsupported OS: ${OS}" >&2
    exit 1
    ;;
esac

if [ ! -d "${KREUZBERG_HOME}/lib/${PLATFORM}" ]; then
  echo "FFI library not found. Run setup.sh first." >&2
  exit 1
fi

export CGO_CFLAGS="-I${KREUZBERG_HOME}/include"
export CGO_LDFLAGS="${LDFLAGS}"

echo "Running Go tests..."
GOWORK=off go test -v -count=1 -timeout 10m ./...
"#,
    )
    .context("Failed to write run_tests.sh")?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&run, fs::Permissions::from_mode(0o755))?;
    }
    Ok(())
}

fn write_go_mod(go_root: &Utf8Path, mode: &GenerationMode) -> Result<()> {
    let go_mod = go_root.join("go.mod");
    let template = match mode {
        GenerationMode::Published { version } => {
            format!(
                "module github.com/kreuzberg-dev/kreuzberg/e2e/go\n\
                 \n\
                 go 1.26\n\
                 \n\
                 require github.com/kreuzberg-dev/kreuzberg/packages/go/v4 v{version}\n"
            )
        }
        GenerationMode::Local => "module github.com/kreuzberg-dev/kreuzberg/e2e/go\n\
             \n\
             go 1.26\n\
             \n\
             require github.com/kreuzberg-dev/kreuzberg/packages/go/v4 v4.0.0\n\
             \n\
             replace github.com/kreuzberg-dev/kreuzberg/packages/go/v4 => ../../packages/go/v4\n"
            .to_string(),
    };
    fs::write(go_mod.as_std_path(), template).context("failed to write go.mod")?;
    Ok(())
}

fn clean_tests(go_root: &Utf8Path) -> Result<()> {
    if !go_root.exists() {
        return Ok(());
    }
    for entry in fs::read_dir(go_root.as_std_path())? {
        let entry = entry?;
        if entry.path().extension().is_some_and(|ext| ext == "go") {
            let name = entry.file_name().to_string_lossy().to_string();
            if name == "helpers_test.go" || name.ends_with("_test.go") {
                fs::remove_file(entry.path())?;
            }
        }
    }
    Ok(())
}

fn write_helpers(go_root: &Utf8Path) -> Result<()> {
    let helpers_path = go_root.join("helpers_test.go");
    fs::write(helpers_path.as_std_path(), GO_HELPERS_TEMPLATE).context("failed to write helpers_test.go")?;
    Ok(())
}

fn render_category(category: &str, fixtures: &[&Fixture]) -> Result<String> {
    let mut buffer = String::new();
    writeln!(buffer, "// Code generated by kreuzberg-e2e-generator. DO NOT EDIT.")?;
    writeln!(buffer, "// Category: {category}")?;
    writeln!(buffer)?;
    writeln!(buffer, "package e2e")?;
    writeln!(buffer)?;
    writeln!(buffer, "import \"testing\"")?;
    writeln!(buffer)?;

    for fixture in fixtures {
        buffer.push_str(&render_test(fixture)?);
        buffer.push('\n');
    }

    Ok(indent_with_tabs(&buffer))
}

fn render_test(fixture: &Fixture) -> Result<String> {
    let mut code = String::new();
    let test_name = format!(
        "Test{}{}",
        to_go_pascal_case(fixture.category()),
        to_go_pascal_case(&fixture.id)
    );
    writeln!(code, "func {test_name}(t *testing.T) {{")?;

    let extraction = fixture.extraction();
    let doc = fixture.document();
    let method = extraction.method;
    let input_type = extraction.input_type;
    let doc_path = go_string_literal(&doc.path);
    let config_literal = render_config_literal(&extraction.config)?;

    // Skip if fixture requires features that may not be available
    let skip_directive = fixture.skip();
    let all_features: Vec<&str> = skip_directive
        .requires_feature
        .iter()
        .chain(doc.requires_external_tool.iter().filter(|t| *t == "paddle-ocr"))
        .map(|s| s.as_str())
        .collect();
    for feature in &all_features {
        writeln!(code, "    skipIfFeatureUnavailable(t, {})", go_string_literal(feature))?;
    }

    match (method, input_type) {
        (ExtractionMethod::Sync, InputType::File) => {
            writeln!(code, "    result := runExtraction(t, {}, {})", doc_path, config_literal)?;
        }
        (ExtractionMethod::Sync, InputType::Bytes) => {
            writeln!(
                code,
                "    result := runExtractionBytes(t, {}, {})",
                doc_path, config_literal
            )?;
        }
        (ExtractionMethod::Async, InputType::File) => {
            writeln!(
                code,
                "    result := runExtractionAsync(t, {}, {})",
                doc_path, config_literal
            )?;
        }
        (ExtractionMethod::Async, InputType::Bytes) => {
            writeln!(
                code,
                "    result := runExtractionBytesAsync(t, {}, {})",
                doc_path, config_literal
            )?;
        }
        (ExtractionMethod::BatchSync, InputType::File) => {
            writeln!(
                code,
                "    results := runBatchExtraction(t, []string{{{}}}, {})",
                doc_path, config_literal
            )?;
            writeln!(code, "    if len(results) == 0 {{")?;
            writeln!(
                code,
                "        t.Fatal(\"expected at least one result from batch extraction\")"
            )?;
            writeln!(code, "    }}")?;
            writeln!(code, "    result := results[0]")?;
        }
        (ExtractionMethod::BatchSync, InputType::Bytes) => {
            // For batch bytes, we use the same file-based batch for simplicity
            // as batch bytes extraction would require reading all files first
            writeln!(
                code,
                "    results := runBatchExtraction(t, []string{{{}}}, {})",
                doc_path, config_literal
            )?;
            writeln!(code, "    if len(results) == 0 {{")?;
            writeln!(
                code,
                "        t.Fatal(\"expected at least one result from batch extraction\")"
            )?;
            writeln!(code, "    }}")?;
            writeln!(code, "    result := results[0]")?;
        }
        (ExtractionMethod::BatchAsync, InputType::File) => {
            writeln!(
                code,
                "    results := runBatchExtractionAsync(t, []string{{{}}}, {})",
                doc_path, config_literal
            )?;
            writeln!(code, "    if len(results) == 0 {{")?;
            writeln!(
                code,
                "        t.Fatal(\"expected at least one result from batch extraction\")"
            )?;
            writeln!(code, "    }}")?;
            writeln!(code, "    result := results[0]")?;
        }
        (ExtractionMethod::BatchAsync, InputType::Bytes) => {
            writeln!(
                code,
                "    results := runBatchExtractionAsync(t, []string{{{}}}, {})",
                doc_path, config_literal
            )?;
            writeln!(code, "    if len(results) == 0 {{")?;
            writeln!(
                code,
                "        t.Fatal(\"expected at least one result from batch extraction\")"
            )?;
            writeln!(code, "    }}")?;
            writeln!(code, "    result := results[0]")?;
        }
    }

    code.push_str(&render_assertions(&fixture.assertions()));
    writeln!(code, "}}")?;
    Ok(code)
}

fn render_assertions(assertions: &Assertions) -> String {
    let mut buffer = String::new();

    if !assertions.expected_mime.is_empty() {
        writeln!(
            buffer,
            "    assertExpectedMime(t, result, {})",
            render_string_slice(&assertions.expected_mime)
        )
        .unwrap();
    }
    if let Some(min) = assertions.min_content_length {
        writeln!(buffer, "    assertMinContentLength(t, result, {min})").unwrap();
    }
    if let Some(max) = assertions.max_content_length {
        writeln!(buffer, "    assertMaxContentLength(t, result, {max})").unwrap();
    }
    if !assertions.content_contains_any.is_empty() {
        writeln!(
            buffer,
            "    assertContentContainsAny(t, result, {})",
            render_string_slice(&assertions.content_contains_any)
        )
        .unwrap();
    }
    if !assertions.content_contains_all.is_empty() {
        writeln!(
            buffer,
            "    assertContentContainsAll(t, result, {})",
            render_string_slice(&assertions.content_contains_all)
        )
        .unwrap();
    }
    if !assertions.content_contains_none.is_empty() {
        writeln!(
            buffer,
            "    assertContentContainsNone(t, result, {})",
            render_string_slice(&assertions.content_contains_none)
        )
        .unwrap();
    }
    if let Some(tables) = assertions.tables.as_ref() {
        let min_literal = tables
            .min
            .map(|v| format!("intPtr({v})"))
            .unwrap_or_else(|| "nil".to_string());
        let max_literal = tables
            .max
            .map(|v| format!("intPtr({v})"))
            .unwrap_or_else(|| "nil".to_string());
        writeln!(buffer, "    assertTableCount(t, result, {min_literal}, {max_literal})").unwrap();
        if tables.has_bounding_boxes == Some(true) {
            writeln!(buffer, "    assertTableBoundingBoxes(t, result)").unwrap();
        }
        if let Some(snippets) = tables.content_contains_any.as_ref()
            && !snippets.is_empty()
        {
            writeln!(
                buffer,
                "    assertTableContentContainsAny(t, result, {})",
                render_string_slice(snippets)
            )
            .unwrap();
        }
    }
    if let Some(lang) = assertions.detected_languages.as_ref() {
        let expected = render_string_slice(&lang.expects);
        let min_conf = lang
            .min_confidence
            .map(|v| format!("floatPtr({v})"))
            .unwrap_or_else(|| "nil".to_string());
        writeln!(buffer, "    assertDetectedLanguages(t, result, {expected}, {min_conf})").unwrap();
    }
    if let Some(chunks) = assertions.chunks.as_ref() {
        let min_count = chunks
            .min_count
            .map(|v| format!("intPtr({v})"))
            .unwrap_or_else(|| "nil".to_string());
        let max_count = chunks
            .max_count
            .map(|v| format!("intPtr({v})"))
            .unwrap_or_else(|| "nil".to_string());
        let each_has_content = chunks
            .each_has_content
            .map(|v| format!("boolPtr({v})"))
            .unwrap_or_else(|| "nil".to_string());
        let each_has_embedding = chunks
            .each_has_embedding
            .map(|v| format!("boolPtr({v})"))
            .unwrap_or_else(|| "nil".to_string());
        let each_has_heading_context = chunks
            .each_has_heading_context
            .map(|v| format!("boolPtr({v})"))
            .unwrap_or_else(|| "nil".to_string());
        let each_has_chunk_type = chunks
            .each_has_chunk_type
            .map(|v| format!("boolPtr({v})"))
            .unwrap_or_else(|| "nil".into());
        let content_starts_with_heading = chunks
            .content_starts_with_heading
            .map(|v| format!("boolPtr({v})"))
            .unwrap_or_else(|| "nil".into());
        buffer.push_str(&format!(
            "    assertChunks(t, result, {min_count}, {max_count}, {each_has_content}, {each_has_embedding}, {each_has_heading_context}, {each_has_chunk_type}, {content_starts_with_heading})\n"
        ));
    }
    if let Some(images) = assertions.images.as_ref() {
        let min_count = images
            .min_count
            .map(|v| format!("intPtr({v})"))
            .unwrap_or_else(|| "nil".to_string());
        let max_count = images
            .max_count
            .map(|v| format!("intPtr({v})"))
            .unwrap_or_else(|| "nil".to_string());
        let formats_include = images
            .formats_include
            .as_ref()
            .map(|v| render_string_slice(v))
            .unwrap_or_else(|| "nil".to_string());
        writeln!(
            buffer,
            "    assertImages(t, result, {}, {}, {})",
            min_count, max_count, formats_include
        )
        .unwrap();
        if images.has_bounding_boxes == Some(true) {
            writeln!(buffer, "    assertImageBoundingBoxes(t, result)").unwrap();
        }
    }
    if let Some(pages) = assertions.pages.as_ref() {
        let min_count = pages
            .min_count
            .map(|v| format!("intPtr({v})"))
            .unwrap_or_else(|| "nil".to_string());
        let exact_count = pages
            .exact_count
            .map(|v| format!("intPtr({v})"))
            .unwrap_or_else(|| "nil".to_string());
        let has_layout = pages
            .has_layout_regions
            .map(|v| format!("boolPtr({v})"))
            .unwrap_or_else(|| "nil".to_string());
        let layout_classes = pages
            .layout_classes_include
            .as_ref()
            .map(|v| render_string_slice(v))
            .unwrap_or_else(|| "nil".to_string());
        writeln!(
            buffer,
            "    assertPages(t, result, {}, {}, {}, {})",
            min_count, exact_count, has_layout, layout_classes
        )
        .unwrap();
    }
    if let Some(elements) = assertions.elements.as_ref() {
        let min_count = elements
            .min_count
            .map(|v| format!("intPtr({v})"))
            .unwrap_or_else(|| "nil".to_string());
        let types_include = elements
            .types_include
            .as_ref()
            .map(|v| render_string_slice(v))
            .unwrap_or_else(|| "nil".to_string());
        writeln!(
            buffer,
            "    assertElements(t, result, {}, {})",
            min_count, types_include
        )
        .unwrap();
    }
    if let Some(ocr) = assertions.ocr_elements.as_ref() {
        let has_elements = ocr
            .has_elements
            .map(|v| format!("boolPtr({v})"))
            .unwrap_or_else(|| "nil".to_string());
        let has_geometry = ocr
            .elements_have_geometry
            .map(|v| format!("boolPtr({v})"))
            .unwrap_or_else(|| "nil".to_string());
        let has_confidence = ocr
            .elements_have_confidence
            .map(|v| format!("boolPtr({v})"))
            .unwrap_or_else(|| "nil".to_string());
        let min_count = ocr
            .min_count
            .map(|v| format!("intPtr({v})"))
            .unwrap_or_else(|| "nil".to_string());
        writeln!(
            buffer,
            "    assertOcrElements(t, result, {}, {}, {}, {})",
            has_elements, has_geometry, has_confidence, min_count
        )
        .unwrap();
    }
    if let Some(document) = assertions.document.as_ref() {
        let has_document = format!("{}", document.has_document);
        let min_node_count = document
            .min_node_count
            .map(|v| format!("intPtr({v})"))
            .unwrap_or_else(|| "nil".to_string());
        let node_types = if !document.node_types_include.is_empty() {
            render_string_slice(&document.node_types_include)
        } else {
            "nil".to_string()
        };
        let has_groups = document
            .has_groups
            .map(|v| format!("boolPtr({v})"))
            .unwrap_or_else(|| "nil".to_string());
        writeln!(
            buffer,
            "    assertDocument(t, result, {}, {}, {}, {})",
            has_document, min_node_count, node_types, has_groups
        )
        .unwrap();
    }
    if let Some(keywords) = assertions.keywords.as_ref() {
        let has_keywords = keywords
            .has_keywords
            .map(|v| format!("boolPtr({v})"))
            .unwrap_or_else(|| "nil".to_string());
        let min_count = keywords
            .min_count
            .map(|v| format!("intPtr({v})"))
            .unwrap_or_else(|| "nil".to_string());
        let max_count = keywords
            .max_count
            .map(|v| format!("intPtr({v})"))
            .unwrap_or_else(|| "nil".to_string());
        writeln!(
            buffer,
            "    assertKeywords(t, result, {}, {}, {})",
            has_keywords, min_count, max_count
        )
        .unwrap();
    }
    if assertions.content_not_empty == Some(true) {
        writeln!(buffer, "    assertContentNotEmpty(t, result)").unwrap();
    }
    if let Some(qs) = assertions.quality_score.as_ref() {
        let has_score = qs
            .has_score
            .map(|v| format!("boolPtr({v})"))
            .unwrap_or_else(|| "nil".to_string());
        let min_score = qs
            .min_score
            .map(|v| format!("floatPtr({v})"))
            .unwrap_or_else(|| "nil".to_string());
        let max_score = qs
            .max_score
            .map(|v| format!("floatPtr({v})"))
            .unwrap_or_else(|| "nil".to_string());
        writeln!(
            buffer,
            "    assertQualityScore(t, result, {}, {}, {})",
            has_score, min_score, max_score
        )
        .unwrap();
    }
    if let Some(pw) = assertions.processing_warnings.as_ref() {
        let max_count = pw
            .max_count
            .map(|v| format!("intPtr({v})"))
            .unwrap_or_else(|| "nil".to_string());
        let is_empty = pw
            .is_empty
            .map(|v| format!("boolPtr({v})"))
            .unwrap_or_else(|| "nil".to_string());
        writeln!(
            buffer,
            "    assertProcessingWarnings(t, result, {}, {})",
            max_count, is_empty
        )
        .unwrap();
    }
    if let Some(lu) = assertions.llm_usage.as_ref() {
        let max_count = lu
            .max_count
            .map(|v| format!("intPtr({v})"))
            .unwrap_or_else(|| "nil".to_string());
        let is_empty = lu
            .is_empty
            .map(|v| format!("boolPtr({v})"))
            .unwrap_or_else(|| "nil".to_string());
        writeln!(buffer, "    assertLlmUsage(t, result, {}, {})", max_count, is_empty).unwrap();
    }
    if let Some(dc) = assertions.djot_content.as_ref() {
        let has_content = dc
            .has_content
            .map(|v| format!("boolPtr({v})"))
            .unwrap_or_else(|| "nil".to_string());
        let min_blocks = dc
            .min_blocks
            .map(|v| format!("intPtr({v})"))
            .unwrap_or_else(|| "nil".to_string());
        writeln!(
            buffer,
            "    assertDjotContent(t, result, {}, {})",
            has_content, min_blocks
        )
        .unwrap();
    }
    if let Some(annotations) = assertions.annotations.as_ref() {
        let has_annotations = annotations.has_annotations.to_string();
        let min_count = annotations
            .min_count
            .map(|v| format!("intPtr({v})"))
            .unwrap_or_else(|| "nil".to_string());
        writeln!(
            buffer,
            "    assertAnnotations(t, result, {}, {})",
            has_annotations, min_count
        )
        .unwrap();
    }

    if let Some(structured) = assertions.structured_output.as_ref() {
        let has_output = structured
            .has_output
            .map(|v| format!("boolPtr({v})"))
            .unwrap_or_else(|| "nil".to_string());
        let validates_schema = structured
            .validates_schema
            .map(|v| format!("boolPtr({v})"))
            .unwrap_or_else(|| "nil".to_string());
        let field_exists = if let Some(ref fields) = structured.field_exists {
            let parts = fields
                .iter()
                .map(|f| format!("\"{}\"", f))
                .collect::<Vec<_>>()
                .join(", ");
            format!("[]string{{{}}}", parts)
        } else {
            "nil".to_string()
        };
        writeln!(
            buffer,
            "    assertStructuredOutput(t, result, {}, {}, {})",
            has_output, validates_schema, field_exists
        )
        .unwrap();
    }

    buffer
}

fn render_config_literal(config: &Map<String, Value>) -> Result<String> {
    if config.is_empty() {
        Ok("nil".to_string())
    } else {
        let json = Value::Object(config.clone());
        let literal = serde_json::to_string_pretty(&json)?;
        Ok(format!("[]byte(`{}`)", literal))
    }
}

fn render_string_slice(values: &[String]) -> String {
    if values.is_empty() {
        "nil".to_string()
    } else {
        let mut literal = String::from("[]string{");
        literal.push_str(
            &values
                .iter()
                .map(|value| go_string_literal(value))
                .collect::<Vec<_>>()
                .join(", "),
        );
        literal.push('}');
        literal
    }
}

fn go_string_literal(value: &str) -> String {
    format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\""))
}

/// Convert a snake_case or UPPER_CASE identifier to PascalCase for Go test names
fn to_go_pascal_case(value: &str) -> String {
    value
        .split('_')
        .filter(|s| !s.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first.to_uppercase().collect::<String>() + &chars.as_str().to_lowercase(),
            }
        })
        .collect()
}

/// Convert space-based indentation (4 spaces per level) to tab-based indentation.
/// Go's gofmt expects tabs, not spaces.
fn indent_with_tabs(text: &str) -> String {
    text.lines()
        .map(|line| {
            let trimmed = line.trim_start();
            if trimmed.is_empty() {
                String::new()
            } else {
                let indent_count = (line.len() - trimmed.len()) / 4;
                format!("{}{}", "\t".repeat(indent_count), trimmed)
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Generate plugin API tests in Go
fn generate_plugin_api_tests(go_root: &Utf8Path, fixtures: &[&Fixture]) -> Result<()> {
    let mut buffer = String::new();

    writeln!(buffer, "// Auto-generated from fixtures/plugin_api/ - DO NOT EDIT")?;
    writeln!(buffer, "//")?;
    writeln!(buffer, "// E2E tests for plugin/config/utility APIs.")?;
    writeln!(buffer, "//")?;
    writeln!(buffer, "// Generated from plugin API fixtures.")?;
    writeln!(
        buffer,
        "// To regenerate: cargo run -p kreuzberg-e2e-generator -- generate --lang go"
    )?;
    writeln!(buffer)?;
    writeln!(buffer, "package e2e")?;
    writeln!(buffer)?;
    writeln!(buffer, "import (")?;
    writeln!(buffer, "    \"os\"")?;
    writeln!(buffer, "    \"path/filepath\"")?;
    writeln!(buffer, "    \"strings\"")?;
    writeln!(buffer, "    \"testing\"")?;
    writeln!(buffer)?;
    writeln!(
        buffer,
        "    kreuzberg \"github.com/kreuzberg-dev/kreuzberg/packages/go/v4\""
    )?;
    writeln!(buffer, ")")?;
    writeln!(buffer)?;

    let mut grouped: Vec<(String, Vec<&Fixture>)> = Vec::new();
    for fixture in fixtures.iter() {
        let category = fixture
            .api_category
            .as_ref()
            .with_context(|| format!("Fixture '{}' missing api_category", fixture.id))?
            .as_str()
            .to_string();

        if let Some(entry_pos) = grouped.iter().position(|(cat, _)| cat == &category) {
            grouped[entry_pos].1.push(fixture);
        } else {
            grouped.push((category, vec![fixture]));
        }
    }
    grouped.sort_by(|a, b| a.0.cmp(&b.0));

    for (category, mut category_fixtures) in grouped {
        category_fixtures.sort_by(|a, b| a.id.cmp(&b.id));

        writeln!(buffer, "// {} Tests", to_title_case(&category))?;
        writeln!(buffer)?;

        for fixture in category_fixtures {
            buffer.push_str(&render_plugin_test(fixture)?);
            buffer.push('\n');
        }
    }

    let output_path = go_root.join("plugin_apis_test.go");
    let formatted_buffer = indent_with_tabs(&buffer);
    fs::write(output_path.as_std_path(), formatted_buffer).context("failed to write plugin_apis_test.go")?;

    Ok(())
}

/// Render a single plugin API test function
fn render_plugin_test(fixture: &Fixture) -> Result<String> {
    let test_spec = fixture
        .test_spec
        .as_ref()
        .with_context(|| format!("Fixture '{}' missing test_spec", fixture.id))?;

    let mut code = String::new();

    let test_name = format!("Test{}", to_pascal_case(&test_spec.function_call.name));
    writeln!(code, "func {test_name}(t *testing.T) {{")?;

    match test_spec.pattern.as_str() {
        "simple_list" => render_simple_list(fixture, test_spec, &mut code)?,
        "clear_registry" => render_clear_registry(fixture, test_spec, &mut code)?,
        "graceful_unregister" => render_graceful_unregister(fixture, test_spec, &mut code)?,
        "config_from_file" => render_config_from_file(fixture, test_spec, &mut code)?,
        "config_discover" => render_config_discover(fixture, test_spec, &mut code)?,
        "mime_from_bytes" => render_mime_from_bytes(fixture, test_spec, &mut code)?,
        "mime_from_path" => render_mime_from_path(fixture, test_spec, &mut code)?,
        "mime_extension_lookup" => render_mime_extension_lookup(fixture, test_spec, &mut code)?,
        _ => anyhow::bail!("Unknown test pattern: {}", test_spec.pattern),
    }

    writeln!(code, "}}")?;
    Ok(code)
}

/// Convert snake_case to PascalCase, handling acronyms like OCR
fn to_pascal_case(s: &str) -> String {
    s.split('_')
        .map(|word| match word.to_uppercase().as_str() {
            "OCR" | "TTL" | "XML" | "HTML" | "URL" | "API" | "ID" => word.to_uppercase(),
            _ => {
                let mut chars = word.chars();
                match chars.next() {
                    None => String::new(),
                    Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                }
            }
        })
        .collect()
}

/// Convert snake_case to Title Case (with spaces)
fn to_title_case(s: &str) -> String {
    s.split('_')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Render a simple list test
fn render_simple_list(
    _fixture: &Fixture,
    test_spec: &crate::fixtures::PluginTestSpec,
    code: &mut String,
) -> Result<()> {
    let func_name = to_pascal_case(&test_spec.function_call.name);

    if let Some(setup) = &test_spec.setup
        && let Some(lazy_init) = &setup.lazy_init_required
        && lazy_init.languages.contains(&"go".to_string())
    {
        writeln!(code, "    tmpDir := t.TempDir()")?;
        writeln!(code, "    testFile := filepath.Join(tmpDir, \"test.pdf\")")?;
        writeln!(code, "    pdfContent := []byte(\"%PDF-1.4\\\\n%EOF\\\\n\")")?;
        writeln!(
            code,
            "    if err := os.WriteFile(testFile, pdfContent, 0644); err != nil {{"
        )?;
        writeln!(code, "        t.Fatalf(\"Failed to write test PDF file: %v\", err)")?;
        writeln!(code, "    }}")?;
        writeln!(code)?;
        writeln!(code, "    // This will initialize the PDF extractor")?;
        writeln!(code, "    _, _ = kreuzberg.ExtractFileSync(testFile, nil)")?;
        writeln!(code)?;
    }

    writeln!(code, "    result, err := kreuzberg.{}()", func_name)?;
    writeln!(code, "    if err != nil {{")?;
    writeln!(code, "        t.Fatalf(\"{} failed: %v\", err)", func_name)?;
    writeln!(code, "    }}")?;
    writeln!(code, "    if result == nil {{")?;
    writeln!(code, "        t.Fatal(\"Result should not be nil\")")?;
    writeln!(code, "    }}")?;

    Ok(())
}

/// Render a clear registry test
fn render_clear_registry(
    _fixture: &Fixture,
    test_spec: &crate::fixtures::PluginTestSpec,
    code: &mut String,
) -> Result<()> {
    let clear_func = to_pascal_case(&test_spec.function_call.name);

    writeln!(code, "    err := kreuzberg.{}()", clear_func)?;
    writeln!(code, "    if err != nil {{")?;
    writeln!(code, "        t.Fatalf(\"{} failed: %v\", err)", clear_func)?;
    writeln!(code, "    }}")?;

    if test_spec.assertions.verify_cleanup {
        let list_func = clear_func.replace("Clear", "List");
        writeln!(code)?;
        writeln!(code, "    result, err := kreuzberg.{}()", list_func)?;
        writeln!(code, "    if err != nil {{")?;
        writeln!(code, "        t.Fatalf(\"{} failed: %v\", err)", list_func)?;
        writeln!(code, "    }}")?;
        writeln!(code, "    if len(result) != 0 {{")?;
        writeln!(
            code,
            "        t.Errorf(\"Expected empty list after clear, got %d items\", len(result))"
        )?;
        writeln!(code, "    }}")?;
    }

    Ok(())
}

/// Render a graceful unregister test
fn render_graceful_unregister(
    _fixture: &Fixture,
    test_spec: &crate::fixtures::PluginTestSpec,
    code: &mut String,
) -> Result<()> {
    let func_name = to_pascal_case(&test_spec.function_call.name);
    let arg = test_spec
        .function_call
        .args
        .first()
        .and_then(|v| v.as_str())
        .unwrap_or("nonexistent-backend-xyz");

    writeln!(code, "    err := kreuzberg.{}(\"{}\")", func_name, arg)?;
    writeln!(code, "    if err != nil {{")?;
    writeln!(
        code,
        "        t.Errorf(\"{} should not error for nonexistent item: %v\", err)",
        func_name
    )?;
    writeln!(code, "    }}")?;

    Ok(())
}

/// Render a config_from_file test
fn render_config_from_file(
    fixture: &Fixture,
    test_spec: &crate::fixtures::PluginTestSpec,
    code: &mut String,
) -> Result<()> {
    let setup = test_spec
        .setup
        .as_ref()
        .with_context(|| format!("Fixture '{}' missing setup for config_from_file", fixture.id))?;

    let file_content = setup
        .temp_file_content
        .as_ref()
        .with_context(|| format!("Fixture '{}' missing temp_file_content", fixture.id))?;

    let file_name = setup
        .temp_file_name
        .as_ref()
        .with_context(|| format!("Fixture '{}' missing temp_file_name", fixture.id))?;

    writeln!(code, "    tmpDir := t.TempDir()")?;
    writeln!(code, "    configPath := filepath.Join(tmpDir, \"{}\")", file_name)?;
    writeln!(code)?;
    writeln!(code, "    configContent := `{}`", file_content)?;
    writeln!(
        code,
        "    if err := os.WriteFile(configPath, []byte(configContent), 0644); err != nil {{"
    )?;
    writeln!(code, "        t.Fatalf(\"Failed to write config file: %v\", err)")?;
    writeln!(code, "    }}")?;
    writeln!(code)?;

    let method_name = to_pascal_case(&test_spec.function_call.name);

    writeln!(code, "    config, err := kreuzberg.Config{}(configPath)", method_name)?;
    writeln!(code, "    if err != nil {{")?;
    writeln!(code, "        t.Fatalf(\"{} failed: %v\", err)", method_name)?;
    writeln!(code, "    }}")?;
    writeln!(code)?;

    for prop in &test_spec.assertions.object_properties {
        render_property_assertion(prop, code)?;
    }

    Ok(())
}

/// Render a config_discover test
fn render_config_discover(
    fixture: &Fixture,
    test_spec: &crate::fixtures::PluginTestSpec,
    code: &mut String,
) -> Result<()> {
    let setup = test_spec
        .setup
        .as_ref()
        .with_context(|| format!("Fixture '{}' missing setup for config_discover", fixture.id))?;

    let file_content = setup
        .temp_file_content
        .as_ref()
        .with_context(|| format!("Fixture '{}' missing temp_file_content", fixture.id))?;

    let file_name = setup
        .temp_file_name
        .as_ref()
        .with_context(|| format!("Fixture '{}' missing temp_file_name", fixture.id))?;

    let subdir_name = setup
        .subdirectory_name
        .as_ref()
        .with_context(|| format!("Fixture '{}' missing subdirectory_name", fixture.id))?;

    writeln!(code, "    tmpDir := t.TempDir()")?;
    writeln!(code, "    configPath := filepath.Join(tmpDir, \"{}\")", file_name)?;
    writeln!(code)?;
    writeln!(
        code,
        "    if err := os.WriteFile(configPath, []byte(`{}`), 0644); err != nil {{",
        file_content
    )?;
    writeln!(code, "        t.Fatalf(\"Failed to write config file: %v\", err)")?;
    writeln!(code, "    }}")?;
    writeln!(code)?;
    writeln!(code, "    subDir := filepath.Join(tmpDir, \"{}\")", subdir_name)?;
    writeln!(code, "    if err := os.MkdirAll(subDir, 0755); err != nil {{")?;
    writeln!(code, "        t.Fatalf(\"Failed to create subdirectory: %v\", err)")?;
    writeln!(code, "    }}")?;
    writeln!(code)?;
    writeln!(code, "    originalDir, err := os.Getwd()")?;
    writeln!(code, "    if err != nil {{")?;
    writeln!(code, "        t.Fatalf(\"Failed to get current directory: %v\", err)")?;
    writeln!(code, "    }}")?;
    writeln!(code, "    defer os.Chdir(originalDir)")?;
    writeln!(code)?;
    writeln!(code, "    if err := os.Chdir(subDir); err != nil {{")?;
    writeln!(code, "        t.Fatalf(\"Failed to change directory: %v\", err)")?;
    writeln!(code, "    }}")?;
    writeln!(code)?;

    let method_name = to_pascal_case(&test_spec.function_call.name);

    writeln!(code, "    config, err := kreuzberg.Config{}()", method_name)?;
    writeln!(code, "    if err != nil {{")?;
    writeln!(code, "        t.Fatalf(\"{} failed: %v\", err)", method_name)?;
    writeln!(code, "    }}")?;
    writeln!(code)?;
    writeln!(code, "    if config == nil {{")?;
    writeln!(
        code,
        "        t.Fatal(\"Config should be discovered from parent directory\")"
    )?;
    writeln!(code, "    }}")?;

    for prop in &test_spec.assertions.object_properties {
        render_property_assertion(prop, code)?;
    }

    Ok(())
}

/// Render a mime_from_bytes test
fn render_mime_from_bytes(
    fixture: &Fixture,
    test_spec: &crate::fixtures::PluginTestSpec,
    code: &mut String,
) -> Result<()> {
    let setup = test_spec
        .setup
        .as_ref()
        .with_context(|| format!("Fixture '{}' missing setup for mime_from_bytes", fixture.id))?;

    let test_data = setup
        .test_data
        .as_ref()
        .with_context(|| format!("Fixture '{}' missing test_data", fixture.id))?;

    let func_name = to_pascal_case(&test_spec.function_call.name);

    writeln!(code, "    testData := []byte(\"{}\")", test_data.replace('\\', "\\\\"))?;
    writeln!(code, "    mime, err := kreuzberg.{}(testData)", func_name)?;
    writeln!(code, "    if err != nil {{")?;
    writeln!(code, "        t.Fatalf(\"{} failed: %v\", err)", func_name)?;
    writeln!(code, "    }}")?;
    writeln!(code)?;

    if let Some(contains) = &test_spec.assertions.string_contains {
        writeln!(
            code,
            "    if !strings.Contains(strings.ToLower(mime), \"{}\") {{",
            contains
        )?;
        writeln!(
            code,
            "        t.Errorf(\"Expected MIME to contain '{}', got %q\", mime)",
            contains
        )?;
        writeln!(code, "    }}")?;
    }

    Ok(())
}

/// Render a mime_from_path test
fn render_mime_from_path(
    fixture: &Fixture,
    test_spec: &crate::fixtures::PluginTestSpec,
    code: &mut String,
) -> Result<()> {
    let setup = test_spec
        .setup
        .as_ref()
        .with_context(|| format!("Fixture '{}' missing setup for mime_from_path", fixture.id))?;

    let file_name = setup
        .temp_file_name
        .as_ref()
        .with_context(|| format!("Fixture '{}' missing temp_file_name", fixture.id))?;

    let file_content = setup
        .temp_file_content
        .as_ref()
        .with_context(|| format!("Fixture '{}' missing temp_file_content", fixture.id))?;

    let func_name = to_pascal_case(&test_spec.function_call.name);

    writeln!(code, "    tmpDir := t.TempDir()")?;
    writeln!(code, "    testFile := filepath.Join(tmpDir, \"{}\")", file_name)?;
    writeln!(
        code,
        "    if err := os.WriteFile(testFile, []byte(\"{}\"), 0644); err != nil {{",
        file_content.replace('"', "\\\"")
    )?;
    writeln!(code, "        t.Fatalf(\"Failed to write test file: %v\", err)")?;
    writeln!(code, "    }}")?;
    writeln!(code)?;
    writeln!(code, "    mime, err := kreuzberg.{}(testFile)", func_name)?;
    writeln!(code, "    if err != nil {{")?;
    writeln!(code, "        t.Fatalf(\"{} failed: %v\", err)", func_name)?;
    writeln!(code, "    }}")?;
    writeln!(code)?;

    if let Some(contains) = &test_spec.assertions.string_contains {
        writeln!(
            code,
            "    if !strings.Contains(strings.ToLower(mime), \"{}\") {{",
            contains
        )?;
        writeln!(
            code,
            "        t.Errorf(\"Expected MIME to contain '{}', got %q\", mime)",
            contains
        )?;
        writeln!(code, "    }}")?;
    }

    Ok(())
}

/// Render a mime_extension_lookup test
fn render_mime_extension_lookup(
    fixture: &Fixture,
    test_spec: &crate::fixtures::PluginTestSpec,
    code: &mut String,
) -> Result<()> {
    let func_name = to_pascal_case(&test_spec.function_call.name);
    let arg = test_spec.function_call.args.first().with_context(|| {
        format!(
            "Fixture '{}' function '{}' missing argument",
            fixture.id, test_spec.function_call.name
        )
    })?;
    let mime_type = arg
        .as_str()
        .with_context(|| format!("Fixture '{}' argument is not a string", fixture.id))?;

    writeln!(
        code,
        "    extensions, err := kreuzberg.{}(\"{}\")",
        func_name, mime_type
    )?;
    writeln!(code, "    if err != nil {{")?;
    writeln!(code, "        t.Fatalf(\"{} failed: %v\", err)", func_name)?;
    writeln!(code, "    }}")?;
    writeln!(code)?;
    writeln!(code, "    if extensions == nil {{")?;
    writeln!(code, "        t.Fatal(\"Extensions list should not be nil\")")?;
    writeln!(code, "    }}")?;

    if let Some(contains) = &test_spec.assertions.list_contains {
        writeln!(code)?;
        writeln!(code, "    found := false")?;
        writeln!(code, "    for _, ext := range extensions {{")?;
        writeln!(code, "        if ext == \"{}\" {{", contains)?;
        writeln!(code, "            found = true")?;
        writeln!(code, "            break")?;
        writeln!(code, "        }}")?;
        writeln!(code, "    }}")?;
        writeln!(code, "    if !found {{")?;
        writeln!(
            code,
            "        t.Errorf(\"Expected extensions to contain '{}', got %v\", extensions)",
            contains
        )?;
        writeln!(code, "    }}")?;
    }

    Ok(())
}

/// Render property assertion for config objects
fn render_property_assertion(prop: &crate::fixtures::ObjectPropertyAssertion, code: &mut String) -> Result<()> {
    let parts: Vec<&str> = prop.path.split('.').collect();
    let go_path = parts.iter().map(|p| to_pascal_case(p)).collect::<Vec<_>>().join(".");
    let display_path = parts.join(".");

    if let Some(exists) = prop.exists
        && exists
    {
        // For nested paths, check each intermediate is non-nil
        let mut checked = String::from("config");
        for (i, part) in parts.iter().enumerate() {
            let pascal = to_pascal_case(part);
            let parent = checked.clone();
            checked = format!("{}.{}", checked, pascal);
            if i < parts.len() - 1 {
                // intermediate: must be non-nil to traverse deeper
                writeln!(code, "    if {}.{} == nil {{", parent, pascal)?;
                writeln!(
                    code,
                    "        t.Fatal(\"Config should have {} property\")",
                    parts[..=i].join(".")
                )?;
                writeln!(code, "    }}")?;
            } else {
                // leaf: existence check
                writeln!(code, "    if {}.{} == nil {{", parent, pascal)?;
                writeln!(
                    code,
                    "        t.Fatal(\"Config should have {} property\")",
                    display_path
                )?;
                writeln!(code, "    }}")?;
            }
        }
    }

    if let Some(value) = &prop.value {
        let accessor = format!("config.{}", go_path);
        match value {
            Value::Number(n) => {
                writeln!(code, "    if {} == nil {{", accessor)?;
                writeln!(code, "        t.Errorf(\"Expected {}={}, got nil\")", display_path, n)?;
                writeln!(code, "    }} else if *{} != {} {{", accessor, n)?;
                writeln!(
                    code,
                    "        t.Errorf(\"Expected {}={}, got %v\", *{})",
                    display_path, n, accessor
                )?;
                writeln!(code, "    }}")?;
            }
            Value::Bool(b) => {
                writeln!(code, "    if {} == nil {{", accessor)?;
                writeln!(code, "        t.Errorf(\"Expected {}={}, got nil\")", display_path, b)?;
                writeln!(code, "    }} else if *{} != {} {{", accessor, b)?;
                writeln!(
                    code,
                    "        t.Errorf(\"Expected {}={}, got %v\", *{})",
                    display_path, b, accessor
                )?;
                writeln!(code, "    }}")?;
            }
            Value::String(s) => {
                writeln!(code, "    if {} == nil {{", accessor)?;
                writeln!(code, "        t.Errorf(\"Expected {}={}, got nil\")", display_path, s)?;
                writeln!(code, "    }} else if *{} != \"{}\" {{", accessor, s)?;
                writeln!(
                    code,
                    "        t.Errorf(\"Expected {}={}, got %v\", *{})",
                    display_path, s, accessor
                )?;
                writeln!(code, "    }}")?;
            }
            _ => {}
        }
    }

    Ok(())
}

fn render_render_category(fixtures: &[&Fixture]) -> Result<String> {
    let mut buffer = String::new();
    writeln!(buffer, "// Code generated by kreuzberg-e2e-generator. DO NOT EDIT.")?;
    writeln!(buffer, "// Category: render")?;
    writeln!(buffer)?;
    writeln!(buffer, "package e2e")?;
    writeln!(buffer)?;
    writeln!(buffer, "import (")?;
    writeln!(buffer, "    \"os\"")?;
    writeln!(buffer, "    \"testing\"")?;
    writeln!(buffer)?;
    writeln!(buffer, "    \"github.com/kreuzberg-dev/kreuzberg/packages/go/v4\"")?;
    writeln!(buffer, ")")?;
    writeln!(buffer)?;

    for fixture in fixtures {
        buffer.push_str(&render_render_test(fixture)?);
        buffer.push('\n');
    }

    Ok(indent_with_tabs(&buffer))
}

fn render_render_test(fixture: &Fixture) -> Result<String> {
    let mut code = String::new();
    let render = fixture.render.as_ref().expect("render spec required");
    let assertions = fixture.assertions().render.unwrap_or_default();

    let test_name = format!("TestRender{}", to_go_pascal_case(&fixture.id));
    let doc_path = go_string_literal(&fixture.document().path);

    writeln!(code, "func {test_name}(t *testing.T) {{")?;
    writeln!(code, "    documentPath := ensureDocument(t, {doc_path}, true)")?;
    writeln!(code, "    if _, err := os.Stat(documentPath); os.IsNotExist(err) {{")?;
    writeln!(
        code,
        "        t.Skipf(\"Skipping %s: missing document at %s\", \"{}\", documentPath)",
        fixture.id
    )?;
    writeln!(code, "    }}")?;

    let dpi_val = render.dpi.unwrap_or(150);

    match render.mode.as_str() {
        "single_page" => {
            let page_index = render.page_index.unwrap_or(0);
            writeln!(
                code,
                "    pngData, err := kreuzberg.RenderPdfPage(documentPath, {page_index}, {dpi_val})"
            )?;
            writeln!(code, "    if err != nil {{")?;
            writeln!(code, "        t.Fatalf(\"RenderPdfPage failed: %v\", err)")?;
            writeln!(code, "    }}")?;
            render_render_assertions_go(&assertions, "pngData", &mut code)?;
        }
        "iterator" => {
            writeln!(
                code,
                "    iter, err := kreuzberg.NewPdfPageIterator(documentPath, {dpi_val})"
            )?;
            writeln!(code, "    if err != nil {{")?;
            writeln!(code, "        t.Fatalf(\"NewPdfPageIterator failed: %v\", err)")?;
            writeln!(code, "    }}")?;
            writeln!(code, "    defer iter.Close()")?;
            writeln!(code, "    pageCount := 0")?;
            writeln!(code, "    for {{")?;
            writeln!(code, "        _, pngData, ok, err := iter.Next()")?;
            writeln!(code, "        if err != nil {{")?;
            writeln!(code, "            t.Fatalf(\"iterator error: %v\", err)")?;
            writeln!(code, "        }}")?;
            writeln!(code, "        if !ok {{")?;
            writeln!(code, "            break")?;
            writeln!(code, "        }}")?;
            writeln!(code, "        assertIsPng(t, pngData)")?;
            writeln!(code, "        pageCount++")?;
            writeln!(code, "    }}")?;
            if let Some(page_count_gte) = assertions.page_count_gte {
                writeln!(code, "    if pageCount < {page_count_gte} {{")?;
                writeln!(
                    code,
                    "        t.Fatalf(\"expected at least {page_count_gte} pages, got %d\", pageCount)"
                )?;
                writeln!(code, "    }}")?;
            }
        }
        _ => anyhow::bail!("Unknown render mode: {}", render.mode),
    }

    writeln!(code, "}}")?;
    Ok(code)
}

fn render_render_assertions_go(assertions: &RenderAssertions, var: &str, code: &mut String) -> Result<()> {
    if assertions.is_png == Some(true) {
        writeln!(code, "    assertIsPng(t, {var})")?;
    }
    if let Some(min_len) = assertions.min_byte_length {
        writeln!(code, "    assertMinByteLength(t, {var}, {min_len})")?;
    }
    Ok(())
}

fn generate_embed_tests_go(fixtures: &[&Fixture]) -> Result<String> {
    let mut buf = String::new();
    writeln!(buf, "// Code generated by kreuzberg-e2e-generator. DO NOT EDIT.")?;
    writeln!(
        buf,
        "// To regenerate: cargo run -p kreuzberg-e2e-generator -- generate --lang go\n"
    )?;
    writeln!(buf, "package e2e\n")?;
    writeln!(buf, "import (")?;
    writeln!(buf, "\t\"context\"")?;
    writeln!(buf, "\t\"runtime\"")?;
    writeln!(buf, "\t\"testing\"")?;
    writeln!(buf, "\n\t\"github.com/kreuzberg-dev/kreuzberg/packages/go/v4\"")?;
    writeln!(buf, ")\n")?;

    writeln!(buf, "// Tests for standalone embed() fixtures.")?;
    for fixture in fixtures {
        buf.push_str(&render_embed_test_go(fixture)?);
    }
    Ok(buf)
}

fn render_embed_test_go(fixture: &Fixture) -> Result<String> {
    let mut code = String::new();
    let spec = fixture.embed_spec();
    let assertions = fixture.embed_assertions();

    let test_name = format!("TestEmbed_{}", fixture.id.replace("-", "_").replace(".", "_"));
    writeln!(code, "func {test_name}(t *testing.T) {{")?;
    writeln!(code, "\t// {}", fixture.description.replace("\n", " "))?;

    let skip_platforms = &fixture.skip().skip_on_platform;
    if !skip_platforms.is_empty() {
        let conditions: Vec<String> = skip_platforms
            .iter()
            .filter_map(|p| match p.as_str() {
                "x86_64-pc-windows-msvc" => {
                    Some("runtime.GOARCH == \"amd64\" && runtime.GOOS == \"windows\"".to_string())
                }
                "aarch64-apple-darwin" => Some("runtime.GOARCH == \"arm64\" && runtime.GOOS == \"darwin\"".to_string()),
                _ => None,
            })
            .collect();
        if !conditions.is_empty() {
            let combined = conditions.join(" || ");
            writeln!(code, "\tif {combined} {{")?;
            writeln!(code, "\t\tt.Skip(\"Platform restricted\")")?;
            writeln!(code, "\t}}")?;
        }
    }

    let model_name = spec
        .config
        .get("model")
        .and_then(|m| m.get("name"))
        .and_then(|n| n.as_str())
        .unwrap_or("balanced");

    let config = &spec.config;
    writeln!(code, "\tconfig := &kreuzberg.EmbeddingConfig{{")?;
    writeln!(code, "\t\tModel: &kreuzberg.EmbeddingModelType{{")?;
    writeln!(code, "\t\t\tType: \"preset\",")?;
    writeln!(code, "\t\t\tName: \"{model_name}\",")?;
    writeln!(code, "\t\t}},")?;
    if let Some(normalize) = config.get("normalize").and_then(|v| v.as_bool()) {
        writeln!(code, "\t\tNormalize: kreuzberg.BoolPtr({normalize}),")?;
    }
    if let Some(batch_size) = config.get("batch_size").and_then(|v| v.as_u64()) {
        let batch_size_lit = batch_size;
        writeln!(code, "\t\tBatchSize: kreuzberg.IntPtr({batch_size_lit}),")?;
    }
    writeln!(code, "\t}}")?;
    writeln!(code)?;

    let texts_literal = render_string_array_go(&spec.texts);
    writeln!(code, "\ttexts := {texts_literal}")?;

    let is_async = spec.async_variant;
    let func = if is_async { "EmbedTextsAsync" } else { "EmbedTexts" };
    let ctx_arg = if is_async { "context.Background(), " } else { "" };

    if spec
        .config
        .get("model")
        .and_then(|m| m.get("name"))
        .is_some_and(|n| n.as_str() == Some("invalid"))
    {
        writeln!(code, "\t_, err := kreuzberg.{func}({ctx_arg}texts, config)")?;
        writeln!(
            code,
            "\tif err == nil {{\n\t\tt.Fatalf(\"Expected error for invalid model\")\n\t}}"
        )?;
        writeln!(code, "}}\n")?;
        return Ok(code);
    } else {
        writeln!(code, "\tresult, err := kreuzberg.{func}({ctx_arg}texts, config)")?;
        writeln!(
            code,
            "\tif err != nil {{\n\t\tt.Fatalf(\"EmbedTexts failed: %v\", err)\n\t}}"
        )?;
    }

    let count_lit = assertions
        .count
        .map(|c| format!("kreuzberg.IntPtr({c})"))
        .unwrap_or_else(|| "nil".to_string());
    let dims_lit = assertions
        .dimensions
        .map(|d| format!("kreuzberg.IntPtr({d})"))
        .unwrap_or_else(|| "nil".to_string());
    writeln!(
        code,
        "\tassertEmbedResult(t, result, {count_lit}, {dims_lit}, {}, {}, {}, {})",
        if assertions.no_nan { "true" } else { "false" },
        if assertions.no_inf { "true" } else { "false" },
        if assertions.non_zero { "true" } else { "false" },
        if assertions.normalized { "true" } else { "false" }
    )?;
    writeln!(code, "}}\n")?;

    Ok(code)
}

fn render_string_array_go(items: &[String]) -> String {
    let parts: Vec<String> = items.iter().map(|s| go_string_literal(s)).collect();
    format!("[]string{{{}}}", parts.join(", "))
}

/// Map manifest type names to their Go binding type names.
///
/// Returns `None` if the type does not exist in the Go binding package.
fn go_type_name(manifest_name: &str) -> Option<&'static str> {
    match manifest_name {
        "ArchiveEntry" => None, // Not present in Go binding
        "Keyword" => Some("ExtractedKeyword"),
        "Uri" => Some("URI"),
        _ => None, // Use the manifest name as-is (handled by caller)
    }
}

/// Resolve the Go type name for a manifest type, returning the mapped name
/// or the original name if no mapping exists.  Returns `None` when the type
/// should be skipped entirely.
fn resolve_go_type_name(manifest_name: &str) -> Option<String> {
    match go_type_name(manifest_name) {
        Some(mapped) => Some(mapped.to_string()),
        None => {
            // Check if the type is explicitly skipped (returned None from mapping)
            if matches!(manifest_name, "ArchiveEntry") {
                return None;
            }
            Some(manifest_name.to_string())
        }
    }
}

/// Generate parity tests for the Go binding.
///
/// Produces `e2e/go/parity_test.go` that verifies all manifest struct types
/// expose the expected fields via reflection.
pub fn generate_parity(manifest: &ParityManifest, output_root: &Utf8Path, _mode: &GenerationMode) -> Result<()> {
    let go_root = output_root.join("go");
    fs::create_dir_all(&go_root).context("Failed to create Go e2e directory for parity")?;

    let lang = "go";
    let profile_name = parity::profile_for_language(lang);
    let enabled_features = manifest.feature_profiles.get(profile_name).cloned().unwrap_or_default();

    let mut buffer = String::new();
    writeln!(buffer, "// Code generated by kreuzberg-e2e-generator. DO NOT EDIT.")?;
    writeln!(
        buffer,
        "// To regenerate: cargo run -p kreuzberg-e2e-generator -- generate --lang go"
    )?;
    writeln!(buffer)?;
    writeln!(buffer, "package e2e")?;
    writeln!(buffer)?;
    writeln!(buffer, "import (")?;
    writeln!(buffer, "\t\"reflect\"")?;
    writeln!(buffer, "\t\"testing\"")?;
    writeln!(buffer)?;
    writeln!(
        buffer,
        "\tkreuzberg \"github.com/kreuzberg-dev/kreuzberg/packages/go/v4\""
    )?;
    writeln!(buffer, ")")?;
    writeln!(buffer)?;

    // ExtractionResult parity
    if let Some(fields) = parity::fields_for_type_and_lang(manifest, "ExtractionResult", lang) {
        writeln!(buffer, "func TestExtractionResultFieldParity(t *testing.T) {{")?;
        writeln!(buffer, "\ttyp := reflect.TypeOf(kreuzberg.ExtractionResult{{}})")?;
        writeln!(buffer, "\texpectedFields := map[string]string{{")?;
        for field_name in fields.keys() {
            let go_name = parity::to_go_pascal_case(field_name);
            writeln!(buffer, "\t\t\"{go_name}\": \"{field_name}\",")?;
        }
        writeln!(buffer, "\t}}")?;
        writeln!(buffer, "\tfor goName, jsonTag := range expectedFields {{")?;
        writeln!(buffer, "\t\tfield, ok := typ.FieldByName(goName)")?;
        writeln!(buffer, "\t\tif !ok {{")?;
        writeln!(
            buffer,
            "\t\t\tt.Errorf(\"ExtractionResult missing field: %s (json: %s)\", goName, jsonTag)"
        )?;
        writeln!(buffer, "\t\t\tcontinue")?;
        writeln!(buffer, "\t\t}}")?;
        writeln!(buffer, "\t\ttag := field.Tag.Get(\"json\")")?;
        writeln!(
            buffer,
            "\t\tif tag == \"\" || (tag != jsonTag && !contains(tag, jsonTag)) {{"
        )?;
        writeln!(
            buffer,
            "\t\t\tt.Errorf(\"field %s has wrong json tag: got %q, want to contain %q\", goName, tag, jsonTag)"
        )?;
        writeln!(buffer, "\t\t}}")?;
        writeln!(buffer, "\t}}")?;
        writeln!(buffer, "}}")?;
        writeln!(buffer)?;
    }

    // ExtractionConfig parity
    if let Some(fields) = parity::fields_for_type_and_lang(manifest, "ExtractionConfig", lang) {
        writeln!(buffer, "func TestExtractionConfigFieldParity(t *testing.T) {{")?;
        writeln!(buffer, "\ttyp := reflect.TypeOf(kreuzberg.ExtractionConfig{{}})")?;
        writeln!(buffer, "\texpectedFields := map[string]string{{")?;
        for field_name in fields.keys() {
            let go_name = parity::to_go_pascal_case(field_name);
            writeln!(buffer, "\t\t\"{go_name}\": \"{field_name}\",")?;
        }
        writeln!(buffer, "\t}}")?;
        writeln!(buffer, "\tfor goName, jsonTag := range expectedFields {{")?;
        writeln!(buffer, "\t\tfield, ok := typ.FieldByName(goName)")?;
        writeln!(buffer, "\t\tif !ok {{")?;
        writeln!(
            buffer,
            "\t\t\tt.Errorf(\"ExtractionConfig missing field: %s (json: %s)\", goName, jsonTag)"
        )?;
        writeln!(buffer, "\t\t\tcontinue")?;
        writeln!(buffer, "\t\t}}")?;
        writeln!(buffer, "\t\ttag := field.Tag.Get(\"json\")")?;
        writeln!(
            buffer,
            "\t\tif tag == \"\" || (tag != jsonTag && !contains(tag, jsonTag)) {{"
        )?;
        writeln!(
            buffer,
            "\t\t\tt.Errorf(\"field %s has wrong json tag: got %q, want to contain %q\", goName, tag, jsonTag)"
        )?;
        writeln!(buffer, "\t\t}}")?;
        writeln!(buffer, "\t}}")?;
        writeln!(buffer, "}}")?;
        writeln!(buffer)?;
    }

    // Additional struct types
    for (type_name, type_def) in &manifest.types {
        if type_name == "ExtractionResult" || type_name == "ExtractionConfig" {
            continue;
        }
        let go_name = match resolve_go_type_name(type_name) {
            Some(name) => name,
            None => continue, // Type does not exist in Go binding
        };
        let fields = match type_def {
            TypeDef::Struct { fields } => parity::filter_fields_for_profile(fields, &enabled_features),
            _ => continue,
        };
        if fields.is_empty() {
            continue;
        }

        writeln!(buffer, "func Test{go_name}FieldParity(t *testing.T) {{")?;
        writeln!(buffer, "\ttyp := reflect.TypeOf(kreuzberg.{go_name}{{}})")?;
        writeln!(buffer, "\texpectedFields := map[string]string{{")?;
        for field_name in fields.keys() {
            let go_name = parity::to_go_pascal_case(field_name);
            writeln!(buffer, "\t\t\"{go_name}\": \"{field_name}\",")?;
        }
        writeln!(buffer, "\t}}")?;
        writeln!(buffer, "\tfor goName, jsonTag := range expectedFields {{")?;
        writeln!(buffer, "\t\tfield, ok := typ.FieldByName(goName)")?;
        writeln!(buffer, "\t\tif !ok {{")?;
        writeln!(
            buffer,
            "\t\t\tt.Errorf(\"{go_name} missing field: %s (json: %s)\", goName, jsonTag)"
        )?;
        writeln!(buffer, "\t\t\tcontinue")?;
        writeln!(buffer, "\t\t}}")?;
        writeln!(buffer, "\t\ttag := field.Tag.Get(\"json\")")?;
        writeln!(
            buffer,
            "\t\tif tag == \"\" || (tag != jsonTag && !contains(tag, jsonTag)) {{"
        )?;
        writeln!(
            buffer,
            "\t\t\tt.Errorf(\"field %s has wrong json tag: got %q, want to contain %q\", goName, tag, jsonTag)"
        )?;
        writeln!(buffer, "\t\t}}")?;
        writeln!(buffer, "\t}}")?;
        writeln!(buffer, "}}")?;
        writeln!(buffer)?;
    }

    // Helper function for string containment
    writeln!(buffer, "func contains(s, substr string) bool {{")?;
    writeln!(buffer, "\tfor i := 0; i <= len(s)-len(substr); i++ {{")?;
    writeln!(buffer, "\t\tif s[i:i+len(substr)] == substr {{")?;
    writeln!(buffer, "\t\t\treturn true")?;
    writeln!(buffer, "\t\t}}")?;
    writeln!(buffer, "\t}}")?;
    writeln!(buffer, "\treturn false")?;
    writeln!(buffer, "}}")?;

    fs::write(go_root.join("parity_test.go"), &buffer).context("Failed to write Go parity test")?;

    Ok(())
}
