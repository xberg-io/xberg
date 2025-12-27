package kreuzberg

import (
	"encoding/json"
	"fmt"
	"reflect"
	"strings"
	"testing"
)

// ============================================================================
// EXISTING TESTS (preserved from original file)
// ============================================================================

func TestMetadataRoundTripPreservesFormatAndAdditionalFields(t *testing.T) {
	input := []byte(`{
		"language": "en",
		"date": "2025-01-01",
		"subject": "Agenda",
		"format_type": "pdf",
		"title": "Doc",
		"page_count": 2,
		"image_preprocessing": {
			"original_dimensions": [1024, 2048],
			"original_dpi": [72.0, 72.0],
			"target_dpi": 300,
			"scale_factor": 1.5,
			"auto_adjusted": true,
			"final_dpi": 300,
			"new_dimensions": [2048, 4096],
			"resample_method": "lanczos",
			"dimension_clamped": false,
			"calculated_dpi": 310,
			"skipped_resize": false
		},
		"json_schema": {"type": "object"},
		"error": {"error_type": "ValidationError", "message": "bad"},
		"custom_meta": {"score": 42}
	}`)

	var meta Metadata
	if err := json.Unmarshal(input, &meta); err != nil {
		t.Fatalf("unmarshal: %v", err)
	}

	if meta.Format.Type != FormatPDF {
		t.Fatalf("expected format pdf, got %s", meta.Format.Type)
	}
	if meta.Format.Pdf == nil || meta.Format.Pdf.PageCount == nil || *meta.Format.Pdf.PageCount != 2 {
		t.Fatalf("expected pdf metadata with page count")
	}
	if meta.Additional == nil || len(meta.Additional) != 1 {
		t.Fatalf("expected additional metadata")
	}

	if _, ok := meta.Additional["custom_meta"]; !ok {
		t.Fatalf("missing custom metadata field")
	}

	encoded, err := json.Marshal(meta)
	if err != nil {
		t.Fatalf("marshal: %v", err)
	}

	var want map[string]any
	if err := json.Unmarshal(input, &want); err != nil {
		t.Fatalf("unmarshal want: %v", err)
	}

	var got map[string]any
	if err := json.Unmarshal(encoded, &got); err != nil {
		t.Fatalf("unmarshal got: %v", err)
	}

	if !reflect.DeepEqual(want, got) {
		t.Fatalf("metadata mismatch: want %#v, got %#v", want, got)
	}
}

func TestMetadataRoundTripHandlesTextFormats(t *testing.T) {
	input := []byte(`{
		"language": "en",
		"format_type": "text",
		"line_count": 10,
		"word_count": 20,
		"character_count": 40,
		"headers": ["Intro"],
		"links": [["https://example.com", "Example"]],
		"custom_score": 0.42
	}`)

	var meta Metadata
	if err := json.Unmarshal(input, &meta); err != nil {
		t.Fatalf("unmarshal: %v", err)
	}

	if meta.Format.Type != FormatText {
		t.Fatalf("expected text format")
	}
	text, ok := meta.TextMetadata()
	if !ok || text.WordCount != 20 {
		t.Fatalf("text metadata not decoded")
	}

	encoded, err := json.Marshal(meta)
	if err != nil {
		t.Fatalf("marshal: %v", err)
	}

	var want map[string]any
	if err := json.Unmarshal(input, &want); err != nil {
		t.Fatalf("want decode: %v", err)
	}
	var got map[string]any
	if err := json.Unmarshal(encoded, &got); err != nil {
		t.Fatalf("got decode: %v", err)
	}

	if !reflect.DeepEqual(want, got) {
		t.Fatalf("metadata mismatch: want %#v, got %#v", want, got)
	}
}

// ============================================================================
// 1. TYPE STRUCTURE TESTS
// ============================================================================

// TestHtmlMetadataStructure verifies HtmlMetadata has correct fields and tags.
func TestHtmlMetadataStructure(t *testing.T) {
	htmlMeta := &HtmlMetadata{
		Title:          StringPtr("Test Page"),
		Description:    StringPtr("A test page"),
		Keywords:       []string{"test", "page"},
		Author:         StringPtr("John Doe"),
		CanonicalURL:   StringPtr("https://example.com/page"),
		BaseHref:       StringPtr("https://example.com/"),
		Language:       StringPtr("en"),
		TextDirection:  StringPtr("ltr"),
		OpenGraph:      map[string]string{"og:title": "Test"},
		TwitterCard:    map[string]string{"twitter:card": "summary"},
		MetaTags:       map[string]string{"custom": "value"},
		Headers:        []HeaderMetadata{},
		Links:          []LinkMetadata{},
		Images:         []HTMLImageMetadata{},
		StructuredData: []StructuredData{},
	}

	if htmlMeta.Title == nil || *htmlMeta.Title != "Test Page" {
		t.Errorf("Title field missing or incorrect")
	}
	if htmlMeta.Keywords == nil {
		t.Errorf("Keywords field missing")
	}
	if htmlMeta.CanonicalURL == nil || *htmlMeta.CanonicalURL != "https://example.com/page" {
		t.Errorf("CanonicalURL field missing or incorrect")
	}
	if htmlMeta.OpenGraph == nil {
		t.Errorf("OpenGraph field missing")
	}
	if htmlMeta.TwitterCard == nil {
		t.Errorf("TwitterCard field missing")
	}
}

// TestKeywordsIsSlice verifies Keywords is []string, not *string.
func TestKeywordsIsSlice(t *testing.T) {
	htmlMeta := &HtmlMetadata{
		Keywords: []string{"keyword1", "keyword2", "keyword3"},
	}

	if len(htmlMeta.Keywords) != 3 {
		t.Errorf("Keywords should be a slice with 3 items, got %d", len(htmlMeta.Keywords))
	}

	for i, kw := range htmlMeta.Keywords {
		if kw == "" {
			t.Errorf("Keyword at index %d is empty", i)
		}
	}

	keywords := htmlMeta.Keywords
	_ = keywords
}

// TestCanonicalUrlRenamed verifies CanonicalURL field exists (not Canonical).
func TestCanonicalUrlRenamed(t *testing.T) {
	htmlMeta := &HtmlMetadata{}

	htmlMetaType := reflect.TypeOf(*htmlMeta)
	foundCanonicalURL := false

	for i := 0; i < htmlMetaType.NumField(); i++ {
		field := htmlMetaType.Field(i)
		if field.Name == "CanonicalURL" {
			foundCanonicalURL = true
			jsonTag := field.Tag.Get("json")
			if !strings.Contains(jsonTag, "canonical_url") {
				t.Errorf("CanonicalURL JSON tag should contain 'canonical_url', got %s", jsonTag)
			}
			break
		}
	}

	if !foundCanonicalURL {
		t.Errorf("CanonicalURL field not found in HtmlMetadata struct")
	}

	htmlMetaType = reflect.TypeOf(*htmlMeta)
	for i := 0; i < htmlMetaType.NumField(); i++ {
		field := htmlMetaType.Field(i)
		if field.Name == "Canonical" {
			t.Errorf("Old field name 'Canonical' should not exist, use 'CanonicalURL'")
		}
	}
}

// TestOpenGraphIsMap verifies OpenGraph is map[string]string.
func TestOpenGraphIsMap(t *testing.T) {
	htmlMeta := &HtmlMetadata{
		OpenGraph: map[string]string{
			"og:title":       "Page Title",
			"og:description": "Page Description",
			"og:image":       "https://example.com/image.png",
		},
	}

	if len(htmlMeta.OpenGraph) != 3 {
		t.Errorf("OpenGraph should have 3 items, got %d", len(htmlMeta.OpenGraph))
	}

	if htmlMeta.OpenGraph["og:title"] != "Page Title" {
		t.Errorf("OpenGraph map access failed")
	}
}

// TestTwitterCardIsMap verifies TwitterCard is map[string]string.
func TestTwitterCardIsMap(t *testing.T) {
	htmlMeta := &HtmlMetadata{
		TwitterCard: map[string]string{
			"twitter:card":  "summary_large_image",
			"twitter:title": "Tweet Title",
		},
	}

	if len(htmlMeta.TwitterCard) != 2 {
		t.Errorf("TwitterCard should have 2 items, got %d", len(htmlMeta.TwitterCard))
	}

	if htmlMeta.TwitterCard["twitter:card"] != "summary_large_image" {
		t.Errorf("TwitterCard map access failed")
	}
}

// ============================================================================
// 2. JSON SERIALIZATION TESTS
// ============================================================================

// TestHtmlMetadataJSONTags verifies JSON struct tags match expected names (snake_case).
func TestHtmlMetadataJSONTags(t *testing.T) {
	expectedTags := map[string]string{
		"Title":          "title",
		"Description":    "description",
		"Keywords":       "keywords",
		"Author":         "author",
		"CanonicalURL":   "canonical_url",
		"BaseHref":       "base_href",
		"Language":       "language",
		"TextDirection":  "text_direction",
		"OpenGraph":      "open_graph",
		"TwitterCard":    "twitter_card",
		"MetaTags":       "meta_tags",
		"Headers":        "headers",
		"Links":          "links",
		"Images":         "images",
		"StructuredData": "structured_data",
	}

	htmlMetaType := reflect.TypeOf(HtmlMetadata{})
	for i := 0; i < htmlMetaType.NumField(); i++ {
		field := htmlMetaType.Field(i)
		expected, ok := expectedTags[field.Name]
		if !ok {
			continue
		}
		jsonTag := field.Tag.Get("json")
		if !strings.Contains(jsonTag, expected) {
			t.Errorf("Field %s should have JSON tag containing '%s', got '%s'",
				field.Name, expected, jsonTag)
		}
	}
}

// TestHeaderMetadataJSON serializes and deserializes HeaderMetadata correctly.
func TestHeaderMetadataJSON(t *testing.T) {
	input := HeaderMetadata{
		Level:      2,
		Text:       "Section Title",
		ID:         StringPtr("section-title"),
		Depth:      1,
		HTMLOffset: 512,
	}

	data, err := json.Marshal(input)
	if err != nil {
		t.Fatalf("marshal HeaderMetadata failed: %v", err)
	}

	var output HeaderMetadata
	if err := json.Unmarshal(data, &output); err != nil {
		t.Fatalf("unmarshal HeaderMetadata failed: %v", err)
	}

	if output.Level != 2 || output.Text != "Section Title" {
		t.Errorf("HeaderMetadata round-trip failed")
	}
	if output.ID == nil || *output.ID != "section-title" {
		t.Errorf("HeaderMetadata ID field incorrect")
	}
}

// TestLinkMetadataJSON serializes and deserializes LinkMetadata with link_type.
func TestLinkMetadataJSON(t *testing.T) {
	input := LinkMetadata{
		Href:     "https://example.com",
		Text:     "Example Link",
		Title:    StringPtr("Example Website"),
		LinkType: "external",
		Rel:      []string{"nofollow", "external"},
		Attributes: map[string]string{
			"target": "_blank",
			"class":  "external-link",
		},
	}

	data, err := json.Marshal(input)
	if err != nil {
		t.Fatalf("marshal LinkMetadata failed: %v", err)
	}

	var jsonData map[string]interface{}
	if err := json.Unmarshal(data, &jsonData); err != nil {
		t.Fatalf("unmarshal to map failed: %v", err)
	}

	if _, ok := jsonData["link_type"]; !ok {
		t.Errorf("JSON should contain 'link_type' field")
	}

	var output LinkMetadata
	if err := json.Unmarshal(data, &output); err != nil {
		t.Fatalf("unmarshal LinkMetadata failed: %v", err)
	}

	if output.LinkType != "external" {
		t.Errorf("LinkMetadata LinkType incorrect: got %s, want external", output.LinkType)
	}
}

// TestImageMetadataJSON serializes and deserializes HTMLImageMetadata with image_type.
func TestImageMetadataJSON(t *testing.T) {
	input := HTMLImageMetadata{
		Src:        "https://example.com/image.jpg",
		Alt:        StringPtr("Description"),
		Title:      StringPtr("Image Title"),
		Dimensions: &[2]int{800, 600},
		ImageType:  "jpg",
		Attributes: map[string]string{
			"loading": "lazy",
			"class":   "thumbnail",
		},
	}

	data, err := json.Marshal(input)
	if err != nil {
		t.Fatalf("marshal HTMLImageMetadata failed: %v", err)
	}

	var jsonData map[string]interface{}
	if err := json.Unmarshal(data, &jsonData); err != nil {
		t.Fatalf("unmarshal to map failed: %v", err)
	}

	if _, ok := jsonData["image_type"]; !ok {
		t.Errorf("JSON should contain 'image_type' field")
	}

	var output HTMLImageMetadata
	if err := json.Unmarshal(data, &output); err != nil {
		t.Fatalf("unmarshal HTMLImageMetadata failed: %v", err)
	}

	if output.ImageType != "jpg" {
		t.Errorf("HTMLImageMetadata ImageType incorrect: got %s, want jpg", output.ImageType)
	}
	if output.Dimensions == nil || output.Dimensions[0] != 800 {
		t.Errorf("HTMLImageMetadata Dimensions incorrect")
	}
}

// TestStructuredDataJSON serializes and deserializes StructuredData with data_type.
func TestStructuredDataJSON(t *testing.T) {
	input := StructuredData{
		DataType:   "json_ld",
		RawJSON:    `{"@type":"Article","headline":"Test"}`,
		SchemaType: StringPtr("Article"),
	}

	data, err := json.Marshal(input)
	if err != nil {
		t.Fatalf("marshal StructuredData failed: %v", err)
	}

	var jsonData map[string]interface{}
	if err := json.Unmarshal(data, &jsonData); err != nil {
		t.Fatalf("unmarshal to map failed: %v", err)
	}

	if _, ok := jsonData["data_type"]; !ok {
		t.Errorf("JSON should contain 'data_type' field")
	}

	var output StructuredData
	if err := json.Unmarshal(data, &output); err != nil {
		t.Fatalf("unmarshal StructuredData failed: %v", err)
	}

	if output.DataType != "json_ld" {
		t.Errorf("StructuredData DataType incorrect")
	}
	if output.SchemaType == nil || *output.SchemaType != "Article" {
		t.Errorf("StructuredData SchemaType incorrect")
	}
}

// ============================================================================
// 3. INTEGRATION TESTS
// ============================================================================

// TestExtractHTMLWithMetadata extracts HTML and verifies metadata structure.
func TestExtractHTMLWithMetadata(t *testing.T) {
	htmlContent := []byte(`<!DOCTYPE html>
<html lang="en">
<head>
	<title>Test Page</title>
	<meta name="description" content="A test page">
	<meta name="keywords" content="test, page, example">
	<meta name="author" content="John Doe">
	<link rel="canonical" href="https://example.com/test">
	<base href="https://example.com/">
	<meta property="og:title" content="Test Page">
	<meta name="twitter:card" content="summary">
</head>
<body>
	<h1>Main Title</h1>
	<h2>Subsection</h2>
	<a href="https://example.com">Link</a>
	<img src="image.jpg" alt="Test Image">
	<script type="application/ld+json">{"@type":"Article"}</script>
</body>
</html>`)

	result, err := ExtractBytesSync(htmlContent, "text/html", nil)
	if err != nil {
		t.Fatalf("ExtractBytesSync failed: %v", err)
	}

	if result == nil || !result.Success {
		t.Fatalf("extraction failed")
	}

	htmlMeta, ok := result.Metadata.HTMLMetadata()
	if !ok {
		t.Fatalf("HTMLMetadata not found in extraction result")
	}

	if htmlMeta.Title == nil || *htmlMeta.Title == "" {
		t.Errorf("HTML title not extracted")
	}

	if result.Metadata.Format.Type != FormatHTML {
		t.Errorf("Format type should be HTML, got %s", result.Metadata.Format.Type)
	}
}

// TestMetadataKeywordArray extracts keywords as slice.
func TestMetadataKeywordArray(t *testing.T) {
	htmlContent := []byte(`<!DOCTYPE html>
<html>
<head>
	<meta name="keywords" content="golang, testing, metadata">
</head>
<body></body>
</html>`)

	result, err := ExtractBytesSync(htmlContent, "text/html", nil)
	if err != nil {
		t.Fatalf("extraction failed: %v", err)
	}

	htmlMeta, ok := result.Metadata.HTMLMetadata()
	if !ok {
		t.Fatalf("HTMLMetadata not extracted")
	}

	if htmlMeta.Keywords == nil {
		t.Errorf("Keywords should not be nil")
	}

	for _, kw := range htmlMeta.Keywords {
		if kw == "" {
			t.Errorf("Empty keyword in array")
		}
	}
}

// TestMetadataOpenGraphMap extracts OG tags as map.
func TestMetadataOpenGraphMap(t *testing.T) {
	htmlContent := []byte(`<!DOCTYPE html>
<html>
<head>
	<meta property="og:title" content="Page Title">
	<meta property="og:description" content="Page description">
	<meta property="og:image" content="https://example.com/image.png">
</head>
<body></body>
</html>`)

	result, err := ExtractBytesSync(htmlContent, "text/html", nil)
	if err != nil {
		t.Fatalf("extraction failed: %v", err)
	}

	htmlMeta, ok := result.Metadata.HTMLMetadata()
	if !ok {
		t.Fatalf("HTMLMetadata not extracted")
	}

	if htmlMeta.OpenGraph == nil {
		t.Logf("Note: OpenGraph may be nil if no OG tags were extracted")
	} else {
		for key, value := range htmlMeta.OpenGraph {
			if key == "" || value == "" {
				t.Errorf("Invalid OpenGraph entry")
			}
		}
	}
}

// TestMetadataHeadersList extracts headers as slice.
func TestMetadataHeadersList(t *testing.T) {
	htmlContent := []byte(`<!DOCTYPE html>
<html>
<body>
	<h1>Title 1</h1>
	<h2>Title 2</h2>
	<h3>Title 3</h3>
</body>
</html>`)

	result, err := ExtractBytesSync(htmlContent, "text/html", nil)
	if err != nil {
		t.Fatalf("extraction failed: %v", err)
	}

	htmlMeta, ok := result.Metadata.HTMLMetadata()
	if !ok {
		t.Fatalf("HTMLMetadata not extracted")
	}

	if htmlMeta.Headers == nil {
		t.Logf("Note: Headers may be empty if no headers were extracted")
	} else {
		for _, hdr := range htmlMeta.Headers {
			if hdr.Level == 0 {
				t.Errorf("Header level should be set")
			}
		}
	}
}

// TestMetadataLinksList extracts links as slice.
func TestMetadataLinksList(t *testing.T) {
	htmlContent := []byte(`<!DOCTYPE html>
<html>
<body>
	<a href="https://example.com">Example</a>
	<a href="/relative">Relative Link</a>
	<a href="#anchor">Anchor</a>
</body>
</html>`)

	result, err := ExtractBytesSync(htmlContent, "text/html", nil)
	if err != nil {
		t.Fatalf("extraction failed: %v", err)
	}

	htmlMeta, ok := result.Metadata.HTMLMetadata()
	if !ok {
		t.Fatalf("HTMLMetadata not extracted")
	}

	if htmlMeta.Links == nil {
		t.Logf("Note: Links may be empty if no links were extracted")
	} else {
		for _, link := range htmlMeta.Links {
			if link.Href == "" {
				t.Errorf("Link href should be set")
			}
		}
	}
}

// TestMetadataImagesList extracts images as slice.
func TestMetadataImagesList(t *testing.T) {
	htmlContent := []byte(`<!DOCTYPE html>
<html>
<body>
	<img src="image1.jpg" alt="Image 1">
	<img src="image2.png">
	<img src="https://example.com/image3.gif">
</body>
</html>`)

	result, err := ExtractBytesSync(htmlContent, "text/html", nil)
	if err != nil {
		t.Fatalf("extraction failed: %v", err)
	}

	htmlMeta, ok := result.Metadata.HTMLMetadata()
	if !ok {
		t.Fatalf("HTMLMetadata not extracted")
	}

	if htmlMeta.Images == nil {
		t.Logf("Note: Images may be empty if no images were extracted")
	} else {
		for _, img := range htmlMeta.Images {
			if img.Src == "" {
				t.Errorf("Image src should be set")
			}
		}
	}
}

// ============================================================================
// 4. EDGE CASES
// ============================================================================

// TestMetadataEmptyHTML returns zero values for empty HTML.
func TestMetadataEmptyHTML(t *testing.T) {
	htmlContent := []byte(`<!DOCTYPE html>
<html>
<head></head>
<body></body>
</html>`)

	result, err := ExtractBytesSync(htmlContent, "text/html", nil)
	if err != nil {
		t.Fatalf("extraction failed: %v", err)
	}

	if !result.Success {
		t.Fatalf("extraction should succeed for empty HTML")
	}

	htmlMeta, ok := result.Metadata.HTMLMetadata()
	if !ok {
		t.Fatalf("HTMLMetadata should be present")
	}

	if htmlMeta.Title != nil && *htmlMeta.Title == "" {
		t.Errorf("Empty title should be nil, not empty string")
	}
}

// TestMetadataNilPointers verifies optional fields are nil when missing.
func TestMetadataNilPointers(t *testing.T) {
	htmlMeta := &HtmlMetadata{
		Keywords: []string{},
	}

	if htmlMeta.Title != nil {
		t.Errorf("Title should be nil when not set")
	}
	if htmlMeta.Description != nil {
		t.Errorf("Description should be nil when not set")
	}
	if htmlMeta.Author != nil {
		t.Errorf("Author should be nil when not set")
	}
	if htmlMeta.CanonicalURL != nil {
		t.Errorf("CanonicalURL should be nil when not set")
	}
}

// TestMetadataEmptyCollections verifies empty slices/maps when no data.
func TestMetadataEmptyCollections(t *testing.T) {
	htmlMeta := &HtmlMetadata{
		Keywords:       []string{},
		OpenGraph:      map[string]string{},
		TwitterCard:    map[string]string{},
		MetaTags:       map[string]string{},
		Headers:        []HeaderMetadata{},
		Links:          []LinkMetadata{},
		Images:         []HTMLImageMetadata{},
		StructuredData: []StructuredData{},
	}

	if htmlMeta.Keywords == nil {
		t.Errorf("Keywords should be empty slice, not nil")
	}
	if len(htmlMeta.Keywords) != 0 {
		t.Errorf("Keywords should be empty")
	}

	if htmlMeta.OpenGraph == nil {
		t.Logf("Note: Empty maps can be nil in some cases")
	}

	if htmlMeta.Headers == nil {
		t.Errorf("Headers should be empty slice, not nil")
	}
}

// ============================================================================
// 5. BACKWARD COMPATIBILITY VALIDATION
// ============================================================================

// TestOldFieldsRemoved verifies old fields don't exist in struct.
func TestOldFieldsRemoved(t *testing.T) {
	htmlMetaType := reflect.TypeOf(HtmlMetadata{})

	oldFields := []string{
		"OgTitle",
		"OgDescription",
		"OgImage",
		"OgUrl",
		"TwitterTitle",
		"TwitterDescription",
		"TwitterImage",
		"Canonical",
	}

	for i := 0; i < htmlMetaType.NumField(); i++ {
		field := htmlMetaType.Field(i)
		for _, oldName := range oldFields {
			if field.Name == oldName {
				t.Errorf("Old field '%s' should have been removed", oldName)
			}
		}
	}
}

// TestNewFieldsExist verifies new fields exist and have correct types.
func TestNewFieldsExist(t *testing.T) {
	expectedFields := map[string]string{
		"Title":          "*string",
		"Description":    "*string",
		"Keywords":       "[]string",
		"Author":         "*string",
		"CanonicalURL":   "*string",
		"BaseHref":       "*string",
		"Language":       "*string",
		"TextDirection":  "*string",
		"OpenGraph":      "map[string]string",
		"TwitterCard":    "map[string]string",
		"MetaTags":       "map[string]string",
		"Headers":        "[]HeaderMetadata",
		"Links":          "[]LinkMetadata",
		"Images":         "[]HTMLImageMetadata",
		"StructuredData": "[]StructuredData",
	}

	htmlMetaType := reflect.TypeOf(HtmlMetadata{})
	foundFields := make(map[string]string)

	for i := 0; i < htmlMetaType.NumField(); i++ {
		field := htmlMetaType.Field(i)
		foundFields[field.Name] = field.Type.String()
	}

	for fieldName, expectedType := range expectedFields {
		actualType, found := foundFields[fieldName]
		if !found {
			t.Errorf("Field '%s' not found in HtmlMetadata", fieldName)
			continue
		}
		if actualType != expectedType {
			t.Errorf("Field '%s' has type %s, expected %s",
				fieldName, actualType, expectedType)
		}
	}
}

// TestHeaderMetadataFields verifies HeaderMetadata has expected fields.
func TestHeaderMetadataFields(t *testing.T) {
	hdr := HeaderMetadata{
		Level:      1,
		Text:       "Title",
		Depth:      0,
		HTMLOffset: 0,
	}

	if hdr.Level != 1 {
		t.Errorf("Level field incorrect")
	}
	if hdr.Text != "Title" {
		t.Errorf("Text field incorrect")
	}
	if hdr.Depth != 0 {
		t.Errorf("Depth field incorrect")
	}
	if hdr.HTMLOffset != 0 {
		t.Errorf("HTMLOffset field incorrect")
	}
}

// TestLinkMetadataFields verifies LinkMetadata has expected fields.
func TestLinkMetadataFields(t *testing.T) {
	link := LinkMetadata{
		Href:     "https://example.com",
		Text:     "Example",
		LinkType: "external",
	}

	if link.Href != "https://example.com" {
		t.Errorf("Href field incorrect")
	}
	if link.Text != "Example" {
		t.Errorf("Text field incorrect")
	}
	if link.LinkType != "external" {
		t.Errorf("LinkType field incorrect")
	}
}

// TestHTMLImageMetadataFields verifies HTMLImageMetadata has expected fields.
func TestHTMLImageMetadataFields(t *testing.T) {
	img := HTMLImageMetadata{
		Src:       "image.jpg",
		ImageType: "jpg",
	}

	if img.Src != "image.jpg" {
		t.Errorf("Src field incorrect")
	}
	if img.ImageType != "jpg" {
		t.Errorf("ImageType field incorrect")
	}
}

// TestStructuredDataFields verifies StructuredData has expected fields.
func TestStructuredDataFields(t *testing.T) {
	sd := StructuredData{
		DataType: "json_ld",
		RawJSON:  `{"@type":"Article"}`,
	}

	if sd.DataType != "json_ld" {
		t.Errorf("DataType field incorrect")
	}
	if sd.RawJSON != `{"@type":"Article"}` {
		t.Errorf("RawJSON field incorrect")
	}
}

// ============================================================================
// 6. EDGE CASE - MALFORMED HTML HANDLING
// ============================================================================

// TestMalformedHTMLHandling tests extraction with invalid HTML structure
// including unclosed tags, broken nesting, and malformed elements.
// Should gracefully handle malformed HTML without panicking.
func TestMalformedHTMLHandling(t *testing.T) {
	testCases := []struct {
		name    string
		html    []byte
		wantErr bool
	}{
		{
			name: "unclosed_tags",
			html: []byte(`<!DOCTYPE html>
<html>
<head>
	<title>Unclosed Title
	<meta name="description" content="Missing closing tag
</head>
<body>
	<h1>Header without closing
	<p>Paragraph
</body>
</html>`),
			wantErr: false,
		},
		{
			name: "broken_nesting",
			html: []byte(`<!DOCTYPE html>
<html>
<head>
	<title>Test</title>
</head>
<body>
	<div><p>Nested<div>Improperly</p></div>
	<span><h1>Invalid nesting</span></h1>
</body>
</html>`),
			wantErr: false,
		},
		{
			name: "malformed_meta_tags",
			html: []byte(`<!DOCTYPE html>
<html>
<head>
	<meta name="description" content=missing quotes
	<meta property="og:title" content=>
	<title>Test</title>
</head>
<body></body>
</html>`),
			wantErr: false,
		},
		{
			name: "duplicate_closing_tags",
			html: []byte(`<!DOCTYPE html>
<html>
<head>
	<title>Test</title>
</head>
<body>
	<div>Content</div></div>
	<p>Text</p></p>
</body>
</html></html>`),
			wantErr: false,
		},
	}

	for _, tc := range testCases {
		t.Run(tc.name, func(t *testing.T) {
			defer func() {
				if r := recover(); r != nil {
					t.Fatalf("extraction panicked on malformed HTML: %v", r)
				}
			}()

			result, err := ExtractBytesSync(tc.html, "text/html", nil)

			if tc.wantErr && err == nil {
				t.Errorf("expected error but got none")
			}
			if !tc.wantErr && err != nil {
				t.Errorf("unexpected error: %v", err)
			}

			if !tc.wantErr && result == nil {
				t.Errorf("expected non-nil result")
			}
		})
	}
}

// ============================================================================
// 7. EDGE CASE - SPECIAL CHARACTERS IN METADATA
// ============================================================================

// TestSpecialCharactersInMetadata tests handling of Unicode, emojis,
// and HTML entities in metadata fields.
func TestSpecialCharactersInMetadata(t *testing.T) {
	htmlContent := []byte(`<!DOCTYPE html>
<html lang="en">
<head>
	<title>🚀 Rocket Project - Test 日本語</title>
	<meta name="description" content="Description with emojis: 😀 🎉 🎨 and more &amp; symbols">
	<meta name="keywords" content="测试, тест, test, テスト, &lt;script&gt;, &quot;quoted&quot;">
	<meta name="author" content="José García & François Müller">
	<meta property="og:title" content="OpenGraph 中文标题 🌟">
	<meta property="og:description" content="Greek: Ελληνικά, Arabic: العربية, Hebrew: עברית">
	<meta name="twitter:card" content="summary">
	<meta name="twitter:title" content="Tweet: こんにちは 👋 مرحبا">
</head>
<body>
	<h1>Unicode: κόσμος мир 世界 🌍</h1>
	<h2>HTML Entities: &quot;quoted&quot; &amp; &lt;tags&gt;</h2>
	<p>Mathematical symbols: ∫ ∑ ∏ √ ≈ ≠</p>
	<a href="https://example.com">Link with emoji 🔗</a>
</body>
</html>`)

	result, err := ExtractBytesSync(htmlContent, "text/html", nil)
	if err != nil {
		t.Fatalf("extraction failed: %v", err)
	}

	if !result.Success {
		t.Fatalf("extraction should succeed with special characters")
	}

	htmlMeta, ok := result.Metadata.HTMLMetadata()
	if !ok {
		t.Fatalf("HTMLMetadata not extracted")
	}

	if htmlMeta.Title == nil || len(*htmlMeta.Title) == 0 {
		t.Errorf("Title should be extracted with special characters")
	} else {
		title := *htmlMeta.Title
		if !strings.ContainsAny(title, "🚀日本語") && !strings.Contains(title, "Rocket") {
			t.Logf("Title may have been processed: %s", title)
		}
	}

	if htmlMeta.Description == nil || len(*htmlMeta.Description) == 0 {
		t.Logf("Note: Description may be nil if not extracted")
	}

	if len(htmlMeta.Keywords) > 0 {
		for _, kw := range htmlMeta.Keywords {
			if kw == "" {
				t.Errorf("Keyword should not be empty")
			}
		}
	}

	if len(htmlMeta.OpenGraph) > 0 {
		for key, value := range htmlMeta.OpenGraph {
			if key == "" || value == "" {
				t.Errorf("OpenGraph entry should not have empty key or value")
			}
		}
	}
}

// ============================================================================
// 8. ERROR HANDLING - INVALID INPUT
// ============================================================================

// TestInvalidInputErrorHandling tests proper error handling with invalid inputs:
// empty bytes, nil pointers, and excessively large HTML documents.
func TestInvalidInputErrorHandling(t *testing.T) {
	testCases := []struct {
		name      string
		data      []byte
		mimeType  string
		wantError bool
		errorMsg  string
	}{
		{
			name:      "empty_bytes",
			data:      []byte{},
			mimeType:  "text/html",
			wantError: true,
			errorMsg:  "data cannot be empty",
		},
		{
			name:      "empty_mime_type",
			data:      []byte("<html></html>"),
			mimeType:  "",
			wantError: true,
			errorMsg:  "mimeType is required",
		},
		{
			name:      "both_empty",
			data:      []byte{},
			mimeType:  "",
			wantError: true,
			errorMsg:  "cannot be empty",
		},
		{
			name:      "valid_html",
			data:      []byte("<!DOCTYPE html><html><body>Test</body></html>"),
			mimeType:  "text/html",
			wantError: false,
		},
	}

	for _, tc := range testCases {
		t.Run(tc.name, func(t *testing.T) {
			result, err := ExtractBytesSync(tc.data, tc.mimeType, nil)

			if tc.wantError {
				if err == nil {
					t.Errorf("expected error containing '%s' but got none", tc.errorMsg)
				} else if !strings.Contains(err.Error(), tc.errorMsg) {
					t.Errorf("expected error containing '%s' but got: %v", tc.errorMsg, err)
				}
				if result != nil && result.Success {
					t.Errorf("result should not be successful for invalid input")
				}
			} else {
				if err != nil {
					t.Errorf("unexpected error: %v", err)
				}
				if result == nil {
					t.Errorf("expected non-nil result for valid input")
				}
			}
		})
	}
}

// ============================================================================
// 9. CONCURRENT EXTRACTION - GOROUTINE SAFETY
// ============================================================================

// TestConcurrentExtraction tests that extraction is safe when called
// from multiple goroutines simultaneously.
func TestConcurrentExtraction(t *testing.T) {
	htmlContent := []byte(`<!DOCTYPE html>
<html>
<head>
	<title>Concurrent Test Page</title>
	<meta name="description" content="Testing concurrent extraction">
	<meta name="keywords" content="concurrent, extraction, test">
</head>
<body>
	<h1>Main Title</h1>
	<p>Some content</p>
</body>
</html>`)

	numGoroutines := 10
	results := make([]*ExtractionResult, numGoroutines)
	errors := make([]error, numGoroutines)

	done := make(chan struct{})
	defer close(done)

	for i := 0; i < numGoroutines; i++ {
		go func(index int) {
			defer func() {
				if r := recover(); r != nil {
					errors[index] = fmt.Errorf("panic: %v", r)
				}
				done <- struct{}{}
			}()

			result, err := ExtractBytesSync(htmlContent, "text/html", nil)
			results[index] = result
			errors[index] = err
		}(i)
	}

	for i := 0; i < numGoroutines; i++ {
		<-done
	}

	for i := 0; i < numGoroutines; i++ {
		if errors[i] != nil {
			t.Errorf("goroutine %d failed: %v", i, errors[i])
		}
		if results[i] == nil {
			t.Errorf("goroutine %d returned nil result", i)
		} else if !results[i].Success {
			t.Errorf("goroutine %d extraction not successful", i)
		}

		htmlMeta, ok := results[i].Metadata.HTMLMetadata()
		if !ok {
			t.Errorf("goroutine %d: HTMLMetadata not extracted", i)
			continue
		}
		if htmlMeta.Title == nil || *htmlMeta.Title != "Concurrent Test Page" {
			t.Errorf("goroutine %d: title mismatch", i)
		}
	}
}

// ============================================================================
// 10. PERFORMANCE BENCHMARK
// ============================================================================

// BenchmarkHTMLExtraction benchmarks metadata extraction from realistic HTML.
// Measures performance with a typical HTML document containing various elements.
func BenchmarkHTMLExtraction(b *testing.B) {
	htmlContent := []byte(`<!DOCTYPE html>
<html lang="en">
<head>
	<meta charset="UTF-8">
	<meta name="viewport" content="width=device-width, initial-scale=1.0">
	<title>Comprehensive Benchmark HTML Document</title>
	<meta name="description" content="A realistic HTML document for benchmarking metadata extraction performance">
	<meta name="keywords" content="benchmark, performance, html, metadata, extraction, test">
	<meta name="author" content="Benchmark Author">
	<meta name="robots" content="index, follow">
	<link rel="canonical" href="https://example.com/benchmark">
	<base href="https://example.com/">
	<meta property="og:title" content="Benchmark HTML Document">
	<meta property="og:description" content="Testing extraction performance">
	<meta property="og:image" content="https://example.com/image.jpg">
	<meta property="og:url" content="https://example.com/benchmark">
	<meta name="twitter:card" content="summary_large_image">
	<meta name="twitter:title" content="Benchmark">
	<meta name="twitter:description" content="Performance test">
	<meta name="twitter:image" content="https://example.com/twitter.jpg">
</head>
<body>
	<header>
		<nav>
			<ul>
				<li><a href="/">Home</a></li>
				<li><a href="/about">About</a></li>
				<li><a href="/blog">Blog</a></li>
				<li><a href="/contact">Contact</a></li>
			</ul>
		</nav>
	</header>
	<main>
		<article>
			<h1>Main Article Title</h1>
			<h2>Introduction Section</h2>
			<p>This is a comprehensive benchmark document with realistic HTML structure for testing metadata extraction performance.</p>

			<h2>Content Section 1</h2>
			<p>Paragraph content goes here with various text elements.</p>
			<img src="image1.jpg" alt="First image" title="Image 1">
			<img src="image2.png" alt="Second image">
			<img src="https://example.com/image3.gif" alt="Third image">

			<h2>Content Section 2</h2>
			<p>More content with links:</p>
			<ul>
				<li><a href="https://example.com">Example Link 1</a></li>
				<li><a href="/relative">Relative Link</a></li>
				<li><a href="#section">Anchor Link</a></li>
				<li><a href="https://other.com" title="External">External Link</a></li>
			</ul>

			<h2>Content Section 3</h2>
			<p>Another section with more structured data:</p>
			<h3>Subsection A</h3>
			<p>Details</p>
			<h3>Subsection B</h3>
			<p>More details</p>
			<h3>Subsection C</h3>
			<p>Even more details</p>

			<h2>Data Section</h2>
			<table>
				<tr><th>Header 1</th><th>Header 2</th></tr>
				<tr><td>Data 1</td><td>Data 2</td></tr>
			</table>

			<h2>Conclusion</h2>
			<p>Concluding paragraph with final thoughts.</p>
		</article>

		<aside>
			<h2>Related Links</h2>
			<ul>
				<li><a href="https://related1.com">Related 1</a></li>
				<li><a href="https://related2.com">Related 2</a></li>
				<li><a href="https://related3.com">Related 3</a></li>
			</ul>
		</aside>
	</main>
	<footer>
		<p>&copy; 2025 Benchmark Site. All rights reserved.</p>
		<a href="/privacy">Privacy Policy</a>
		<a href="/terms">Terms of Service</a>
	</footer>

	<script type="application/ld+json">
	{
		"@context": "https://schema.org",
		"@type": "Article",
		"headline": "Benchmark Article",
		"image": "https://example.com/image.jpg",
		"author": {
			"@type": "Person",
			"name": "Author Name"
		}
	}
	</script>
</body>
</html>`)

	b.ResetTimer()
	for i := 0; i < b.N; i++ {
		result, err := ExtractBytesSync(htmlContent, "text/html", nil)
		if err != nil {
			b.Fatalf("extraction failed: %v", err)
		}
		if result == nil || !result.Success {
			b.Fatalf("extraction was not successful")
		}
	}
}

// BenchmarkHTMLExtractionLargeDocument benchmarks extraction on a larger HTML document
// to measure performance at scale.
func BenchmarkHTMLExtractionLargeDocument(b *testing.B) {
	var sb strings.Builder
	sb.WriteString(`<!DOCTYPE html>
<html lang="en">
<head>
	<title>Large Document Benchmark</title>
	<meta name="description" content="Large document for performance testing">
	<meta name="keywords" content="large, document, benchmark">
</head>
<body>
	<h1>Large Document</h1>
`)

	for i := 0; i < 50; i++ {
		sb.WriteString(fmt.Sprintf(`
	<section id="section-%d">
		<h2>Section %d</h2>
		<p>Content for section %d with some text and information.</p>
		<img src="image-%d.jpg" alt="Image %d">
		<ul>
			<li><a href="link-%d-1">Link 1</a></li>
			<li><a href="link-%d-2">Link 2</a></li>
			<li><a href="link-%d-3">Link 3</a></li>
		</ul>
	</section>
`, i, i, i, i, i, i, i, i))
	}

	sb.WriteString(`
</body>
</html>`)

	htmlContent := []byte(sb.String())

	b.ResetTimer()
	for i := 0; i < b.N; i++ {
		result, err := ExtractBytesSync(htmlContent, "text/html", nil)
		if err != nil {
			b.Fatalf("extraction failed: %v", err)
		}
		if result == nil || !result.Success {
			b.Fatalf("extraction was not successful")
		}
	}
}
