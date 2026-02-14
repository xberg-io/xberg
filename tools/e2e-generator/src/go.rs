use crate::fixtures::{Assertions, ExtractionMethod, Fixture, InputType};
use anyhow::{Context, Result};
use camino::Utf8Path;
use itertools::Itertools;
use serde_json::{Map, Value};
use std::fmt::Write as _;
use std::fs;

const GO_HELPERS_TEMPLATE: &str = r#"package e2e

import (
	"context"
	"encoding/json"
	"fmt"
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

	if strings.Contains(message, "missing dependency") {
		return true
	}
	return false
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

func assertMetadataExpectation(t *testing.T, result *kreuzberg.ExtractionResult, path string, expectation []byte) {
	t.Helper()
	if len(expectation) == 0 {
		return
	}

	metadata := metadataAsMap(t, result.Metadata)
	value := lookupMetadataValue(metadata, path)
	if value == nil {
		t.Fatalf("metadata path %q missing", path)
	}

	var spec map[string]any
	if err := json.Unmarshal(expectation, &spec); err != nil {
		t.Fatalf("failed to decode metadata expectation for %s: %v", path, err)
	}

	if expected, ok := spec["eq"]; ok {
		if !valuesEqual(value, expected) {
			t.Fatalf("expected metadata %q == %v, got %v", path, expected, value)
		}
	}
	if gte, ok := spec["gte"]; ok {
		if !compareFloat(value, gte, true) {
			t.Fatalf("expected metadata %q >= %v, got %v", path, gte, value)
		}
	}
	if lte, ok := spec["lte"]; ok {
		if !compareFloat(value, lte, false) {
			t.Fatalf("expected metadata %q <= %v, got %v", path, lte, value)
		}
	}
	if contains, ok := spec["contains"]; ok {
		if !valueContains(value, contains) {
			t.Fatalf("expected metadata %q to contain %v, got %v", path, contains, value)
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

func valuesEqual(a, b any) bool {
	switch av := a.(type) {
	case string:
		if bv, ok := b.(string); ok {
			return av == bv
		}
	case float64:
		if bv, ok := b.(float64); ok {
			return av == bv
		}
	case bool:
		if bv, ok := b.(bool); ok {
			return av == bv
		}
	case []any:
		bv, ok := b.([]any)
		if !ok || len(av) != len(bv) {
			return false
		}
		for i := range av {
			if !valuesEqual(av[i], bv[i]) {
				return false
			}
		}
		return true
	}
	return false
}

func compareFloat(actual any, expected any, gte bool) bool {
	actualFloat, ok := toFloat(actual)
	if !ok {
		return false
	}
	expectedFloat, ok := toFloat(expected)
	if !ok {
		return false
	}
	if gte {
		return actualFloat >= expectedFloat
	}
	return actualFloat <= expectedFloat
}

func toFloat(value any) (float64, bool) {
	switch v := value.(type) {
	case float64:
		return v, true
	case int:
		return float64(v), true
	case int64:
		return float64(v), true
	case json.Number:
		f, err := v.Float64()
		if err != nil {
			return 0, false
		}
		return f, true
	default:
		return 0, false
	}
}

func valueContains(value any, expectation any) bool {
	switch v := value.(type) {
	case string:
		if needle, ok := expectation.(string); ok {
			return strings.Contains(strings.ToLower(v), strings.ToLower(needle))
		}
	case []any:
		switch needle := expectation.(type) {
		case []any:
			for _, candidate := range needle {
				if !valueContains(v, candidate) {
					return false
				}
			}
			return true
		default:
			for _, item := range v {
				if valuesEqual(item, needle) {
					return true
				}
			}
		}
	}
	return false
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

func skipIfPaddleOcrUnavailable(t *testing.T) {
	t.Helper()
	flag := os.Getenv("KREUZBERG_PADDLE_OCR_AVAILABLE")
	if flag == "" || flag == "0" || strings.EqualFold(flag, "false") {
		t.Skip("Skipping: PaddleOCR not available (set KREUZBERG_PADDLE_OCR_AVAILABLE=1)")
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
	} else {
		if doc != nil {
			t.Fatal("Expected document to be nil but got a document")
		}
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
			if result.Keywords == nil || len(result.Keywords) == 0 {
				t.Fatalf("expected keywords in result but Keywords field is nil or empty")
			}
		} else {
			if result.Keywords != nil && len(result.Keywords) > 0 {
				t.Fatalf("expected no keywords but found %d", len(result.Keywords))
			}
		}
	}
	count := len(result.Keywords)
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
"#;

pub fn generate(fixtures: &[Fixture], output_root: &Utf8Path) -> Result<()> {
    let go_root = output_root.join("go");
    fs::create_dir_all(&go_root).context("failed to create go e2e directory")?;

    write_go_mod(&go_root)?;
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

    Ok(())
}

fn write_go_mod(go_root: &Utf8Path) -> Result<()> {
    let go_mod = go_root.join("go.mod");
    let template = r#"module github.com/kreuzberg-dev/kreuzberg/e2e/go

go 1.25

require github.com/kreuzberg-dev/kreuzberg/packages/go/v4 v4.0.0

replace github.com/kreuzberg-dev/kreuzberg/packages/go/v4 => ../../packages/go/v4
"#;
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

    // Skip if fixture requires paddle-ocr and it's not available
    let requires_paddle = fixture.skip().requires_feature.iter().any(|f| f == "paddle-ocr")
        || doc.requires_external_tool.iter().any(|t| t == "paddle-ocr");
    if requires_paddle {
        writeln!(code, "    skipIfPaddleOcrUnavailable(t)")?;
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
        writeln!(
            buffer,
            "    assertChunks(t, result, {}, {}, {}, {})",
            min_count, max_count, each_has_content, each_has_embedding
        )
        .unwrap();
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
        writeln!(buffer, "    assertPages(t, result, {}, {})", min_count, exact_count).unwrap();
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
            "OCR" => "OCR".to_string(),
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
    let list_func = clear_func.replace("Clear", "List");

    writeln!(code, "    err := kreuzberg.{}()", clear_func)?;
    writeln!(code, "    if err != nil {{")?;
    writeln!(code, "        t.Fatalf(\"{} failed: %v\", err)", clear_func)?;
    writeln!(code, "    }}")?;
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

    if parts.len() == 1 {
        if let Some(exists) = prop.exists
            && exists
        {
            writeln!(code, "    if config.{} == nil {{", to_pascal_case(parts[0]))?;
            writeln!(code, "        t.Fatal(\"Config should have {} property\")", parts[0])?;
            writeln!(code, "    }}")?;
        }
    } else if parts.len() == 2 {
        let parent = to_pascal_case(parts[0]);
        let child = to_pascal_case(parts[1]);

        if let Some(exists) = prop.exists
            && exists
        {
            writeln!(code, "    if config.{} == nil {{", parent)?;
            writeln!(code, "        t.Fatal(\"Config should have {} property\")", parts[0])?;
            writeln!(code, "    }}")?;
        }

        if let Some(value) = &prop.value {
            match value {
                Value::Number(n) => {
                    writeln!(code, "    if config.{}.{} == nil {{", parent, child)?;
                    writeln!(
                        code,
                        "        t.Errorf(\"Expected {}.{}={}, got nil\")",
                        parts[0], parts[1], n
                    )?;
                    writeln!(code, "    }} else if *config.{}.{} != {} {{", parent, child, n)?;
                    writeln!(
                        code,
                        "        t.Errorf(\"Expected {}.{}={}, got %v\", *config.{}.{})",
                        parts[0], parts[1], n, parent, child
                    )?;
                    writeln!(code, "    }}")?;
                }
                Value::Bool(b) => {
                    writeln!(code, "    if config.{}.{} == nil {{", parent, child)?;
                    writeln!(
                        code,
                        "        t.Errorf(\"Expected {}.{}={}, got nil\")",
                        parts[0], parts[1], b
                    )?;
                    writeln!(code, "    }} else if *config.{}.{} != {} {{", parent, child, b)?;
                    writeln!(
                        code,
                        "        t.Errorf(\"Expected {}.{}={}, got %v\", *config.{}.{})",
                        parts[0], parts[1], b, parent, child
                    )?;
                    writeln!(code, "    }}")?;
                }
                Value::String(s) => {
                    writeln!(code, "    if config.{}.{} == nil {{", parent, child)?;
                    writeln!(
                        code,
                        "        t.Errorf(\"Expected {}.{}={}, got nil\")",
                        parts[0], parts[1], s
                    )?;
                    writeln!(code, "    }} else if *config.{}.{} != \"{}\" {{", parent, child, s)?;
                    writeln!(
                        code,
                        "        t.Errorf(\"Expected {}.{}={}, got %v\", *config.{}.{})",
                        parts[0], parts[1], s, parent, child
                    )?;
                    writeln!(code, "    }}")?;
                }
                _ => {}
            }
        }
    }

    Ok(())
}
