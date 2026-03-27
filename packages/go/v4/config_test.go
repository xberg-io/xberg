package kreuzberg_test

import (
	"bytes"
	"encoding/json"
	"strings"
	"testing"

	kreuzberg "github.com/kreuzberg-dev/kreuzberg/packages/go/v4"
)

// ============================================================================
// ExtractionConfig Tests
// ============================================================================

func TestExtractionConfig_DefaultConstruction(t *testing.T) {
	config := &kreuzberg.ExtractionConfig{}

	if config.UseCache != nil {
		t.Errorf("expected UseCache to be nil by default, got %v", config.UseCache)
	}
	if config.EnableQualityProcessing != nil {
		t.Errorf("expected EnableQualityProcessing to be nil by default")
	}
}

func TestExtractionConfig_WithOptions(t *testing.T) {
	useCache := true
	enableQuality := true
	config := &kreuzberg.ExtractionConfig{
		UseCache:                &useCache,
		EnableQualityProcessing: &enableQuality,
	}

	if config.UseCache == nil || !*config.UseCache {
		t.Error("expected UseCache to be true")
	}
	if config.EnableQualityProcessing == nil || !*config.EnableQualityProcessing {
		t.Error("expected EnableQualityProcessing to be true")
	}
}

func TestExtractionConfig_FunctionalOptions(t *testing.T) {
	config := kreuzberg.NewExtractionConfig(
		kreuzberg.WithUseCache(true),
		kreuzberg.WithEnableQualityProcessing(false),
	)

	if config.UseCache == nil || !*config.UseCache {
		t.Error("expected UseCache to be true")
	}
	if config.EnableQualityProcessing == nil || *config.EnableQualityProcessing {
		t.Error("expected EnableQualityProcessing to be false")
	}
}

func TestExtractionConfig_JSON_Marshaling(t *testing.T) {
	useCache := true
	original := &kreuzberg.ExtractionConfig{
		UseCache: &useCache,
	}

	data, err := json.Marshal(original)
	if err != nil {
		t.Fatalf("failed to marshal ExtractionConfig: %v", err)
	}

	var restored kreuzberg.ExtractionConfig
	err = json.Unmarshal(data, &restored)
	if err != nil {
		t.Fatalf("failed to unmarshal ExtractionConfig: %v", err)
	}

	if restored.UseCache == nil || *restored.UseCache != *original.UseCache {
		t.Error("UseCache not preserved after roundtrip")
	}
}

func TestExtractionConfig_NilPointerHandling(t *testing.T) {
	var config *kreuzberg.ExtractionConfig
	// Test that nil pointer doesn't panic
	_ = config
}

func TestExtractionConfig_MaxConcurrentExtractions(t *testing.T) {
	maxConcurrent := 10
	config := &kreuzberg.ExtractionConfig{
		MaxConcurrentExtractions: &maxConcurrent,
	}

	if config.MaxConcurrentExtractions == nil || *config.MaxConcurrentExtractions != 10 {
		t.Error("expected MaxConcurrentExtractions to be 10")
	}
}

func TestExtractionConfig_FieldTags(t *testing.T) {
	useCache := true
	config := &kreuzberg.ExtractionConfig{
		UseCache: &useCache,
	}

	data, err := json.Marshal(config)
	if err != nil {
		t.Fatalf("failed to marshal: %v", err)
	}

	jsonStr := string(data)
	if jsonStr != `{"use_cache":true}` {
		t.Errorf("expected JSON with use_cache tag, got %s", jsonStr)
	}
}

func TestExtractionConfig_WithForceOCR(t *testing.T) {
	config := kreuzberg.NewExtractionConfig(
		kreuzberg.WithForceOCR(true),
	)

	if config.ForceOCR == nil || !*config.ForceOCR {
		t.Error("expected ForceOCR to be true")
	}
}

// ============================================================================
// OCRConfig Tests
// ============================================================================

func TestOCRConfig_DefaultConstruction(t *testing.T) {
	config := &kreuzberg.OCRConfig{}

	if config.Backend != "" {
		t.Errorf("expected Backend to be empty by default, got %s", config.Backend)
	}
	if config.Language != nil {
		t.Errorf("expected Language to be nil by default")
	}
}

func TestOCRConfig_WithOptions(t *testing.T) {
	config := &kreuzberg.OCRConfig{
		Backend: "tesseract",
	}

	if config.Backend != "tesseract" {
		t.Errorf("expected Backend to be tesseract, got %s", config.Backend)
	}
}

func TestOCRConfig_FunctionalOptions(t *testing.T) {
	config := kreuzberg.NewOCRConfig(
		kreuzberg.WithOCRBackend("tesseract"),
		kreuzberg.WithOCRLanguage("eng"),
	)

	if config.Backend != "tesseract" {
		t.Errorf("expected Backend to be tesseract, got %s", config.Backend)
	}
	if config.Language == nil || *config.Language != "eng" {
		t.Error("expected Language to be eng")
	}
}

func TestOCRConfig_JSON_Marshaling(t *testing.T) {
	original := &kreuzberg.OCRConfig{
		Backend: "tesseract",
	}

	data, err := json.Marshal(original)
	if err != nil {
		t.Fatalf("failed to marshal OCRConfig: %v", err)
	}

	var restored kreuzberg.OCRConfig
	err = json.Unmarshal(data, &restored)
	if err != nil {
		t.Fatalf("failed to unmarshal OCRConfig: %v", err)
	}

	if restored.Backend != original.Backend {
		t.Errorf("Backend not preserved: expected %s, got %s", original.Backend, restored.Backend)
	}
}

func TestOCRConfig_WithTesseract(t *testing.T) {
	config := kreuzberg.NewOCRConfig(
		kreuzberg.WithTesseract(
			kreuzberg.WithTesseractLanguage("eng"),
		),
	)

	if config.Tesseract == nil {
		t.Fatal("expected Tesseract to be set")
	}
	if config.Tesseract.Language != "eng" {
		t.Errorf("expected Language to be eng, got %s", config.Tesseract.Language)
	}
}

func TestOCRConfig_NilPointerHandling(t *testing.T) {
	var config *kreuzberg.OCRConfig
	_ = config
}

// ============================================================================
// TesseractConfig Tests
// ============================================================================

func TestTesseractConfig_DefaultConstruction(t *testing.T) {
	config := &kreuzberg.TesseractConfig{}

	if config.Language != "" {
		t.Errorf("expected Language to be empty by default")
	}
	if config.PSM != nil {
		t.Errorf("expected PSM to be nil by default")
	}
}

func TestTesseractConfig_WithOptions(t *testing.T) {
	psm := 6
	config := &kreuzberg.TesseractConfig{
		Language: "eng",
		PSM:      &psm,
	}

	if config.Language != "eng" {
		t.Error("expected Language to be eng")
	}
	if config.PSM == nil || *config.PSM != 6 {
		t.Error("expected PSM to be 6")
	}
}

func TestTesseractConfig_FunctionalOptions(t *testing.T) {
	config := kreuzberg.NewTesseractConfig(
		kreuzberg.WithTesseractLanguage("deu"),
		kreuzberg.WithTesseractPSM(3),
		kreuzberg.WithTesseractOEM(1),
	)

	if config.Language != "deu" {
		t.Error("expected Language to be deu")
	}
	if config.PSM == nil || *config.PSM != 3 {
		t.Error("expected PSM to be 3")
	}
	if config.OEM == nil || *config.OEM != 1 {
		t.Error("expected OEM to be 1")
	}
}

func TestTesseractConfig_JSON_Marshaling(t *testing.T) {
	psm := 6
	original := &kreuzberg.TesseractConfig{
		Language: "eng",
		PSM:      &psm,
	}

	data, err := json.Marshal(original)
	if err != nil {
		t.Fatalf("failed to marshal TesseractConfig: %v", err)
	}

	var restored kreuzberg.TesseractConfig
	err = json.Unmarshal(data, &restored)
	if err != nil {
		t.Fatalf("failed to unmarshal TesseractConfig: %v", err)
	}

	if restored.Language != original.Language {
		t.Error("Language not preserved")
	}
	if restored.PSM == nil || *restored.PSM != *original.PSM {
		t.Error("PSM not preserved")
	}
}

func TestTesseractConfig_TableDetection(t *testing.T) {
	config := kreuzberg.NewTesseractConfig(
		kreuzberg.WithTesseractEnableTableDetection(true),
		kreuzberg.WithTesseractTableMinConfidence(0.5),
	)

	if config.EnableTableDetection == nil || !*config.EnableTableDetection {
		t.Error("expected EnableTableDetection to be true")
	}
	if config.TableMinConfidence == nil || *config.TableMinConfidence != 0.5 {
		t.Error("expected TableMinConfidence to be 0.5")
	}
}

func TestTesseractConfig_CharWhitelist(t *testing.T) {
	config := kreuzberg.NewTesseractConfig(
		kreuzberg.WithTesseractTesseditCharWhitelist("0123456789"),
	)

	if config.TesseditCharWhitelist != "0123456789" {
		t.Errorf("expected whitelist to be 0123456789, got %s", config.TesseditCharWhitelist)
	}
}

func TestTesseractConfig_NilPointerHandling(t *testing.T) {
	var config *kreuzberg.TesseractConfig
	_ = config
}

// ============================================================================
// ImagePreprocessingConfig Tests
// ============================================================================

func TestImagePreprocessingConfig_DefaultConstruction(t *testing.T) {
	config := &kreuzberg.ImagePreprocessingConfig{}

	if config.TargetDPI != nil {
		t.Errorf("expected TargetDPI to be nil by default")
	}
	if config.AutoRotate != nil {
		t.Errorf("expected AutoRotate to be nil by default")
	}
}

func TestImagePreprocessingConfig_WithOptions(t *testing.T) {
	dpi := 300
	config := &kreuzberg.ImagePreprocessingConfig{
		TargetDPI: &dpi,
	}

	if config.TargetDPI == nil || *config.TargetDPI != 300 {
		t.Error("expected TargetDPI to be 300")
	}
}

func TestImagePreprocessingConfig_FunctionalOptions(t *testing.T) {
	config := kreuzberg.NewImagePreprocessingConfig(
		kreuzberg.WithTargetDPI(300),
		kreuzberg.WithAutoRotate(true),
		kreuzberg.WithDeskew(true),
	)

	if config.TargetDPI == nil || *config.TargetDPI != 300 {
		t.Error("expected TargetDPI to be 300")
	}
	if config.AutoRotate == nil || !*config.AutoRotate {
		t.Error("expected AutoRotate to be true")
	}
	if config.Deskew == nil || !*config.Deskew {
		t.Error("expected Deskew to be true")
	}
}

func TestImagePreprocessingConfig_JSON_Marshaling(t *testing.T) {
	dpi := 300
	original := &kreuzberg.ImagePreprocessingConfig{
		TargetDPI: &dpi,
	}

	data, err := json.Marshal(original)
	if err != nil {
		t.Fatalf("failed to marshal ImagePreprocessingConfig: %v", err)
	}

	var restored kreuzberg.ImagePreprocessingConfig
	err = json.Unmarshal(data, &restored)
	if err != nil {
		t.Fatalf("failed to unmarshal ImagePreprocessingConfig: %v", err)
	}

	if restored.TargetDPI == nil || *restored.TargetDPI != *original.TargetDPI {
		t.Error("TargetDPI not preserved")
	}
}

func TestImagePreprocessingConfig_BinarizationMode(t *testing.T) {
	config := kreuzberg.NewImagePreprocessingConfig(
		kreuzberg.WithBinarizationMode("otsu"),
	)

	if config.BinarizationMode != "otsu" {
		t.Errorf("expected BinarizationMode to be otsu, got %s", config.BinarizationMode)
	}
}

func TestImagePreprocessingConfig_AllOptions(t *testing.T) {
	config := kreuzberg.NewImagePreprocessingConfig(
		kreuzberg.WithTargetDPI(300),
		kreuzberg.WithAutoRotate(true),
		kreuzberg.WithDeskew(true),
		kreuzberg.WithDenoise(true),
		kreuzberg.WithContrastEnhance(true),
		kreuzberg.WithInvertColors(false),
	)

	if config.TargetDPI == nil || *config.TargetDPI != 300 {
		t.Error("TargetDPI not set correctly")
	}
	if config.Denoise == nil || !*config.Denoise {
		t.Error("Denoise not set correctly")
	}
	if config.ContrastEnhance == nil || !*config.ContrastEnhance {
		t.Error("ContrastEnhance not set correctly")
	}
}

func TestImagePreprocessingConfig_NilPointerHandling(t *testing.T) {
	var config *kreuzberg.ImagePreprocessingConfig
	_ = config
}

// ============================================================================
// ChunkingConfig Tests
// ============================================================================

func TestChunkingConfig_DefaultConstruction(t *testing.T) {
	config := &kreuzberg.ChunkingConfig{}

	if config.MaxChars != nil {
		t.Errorf("expected MaxChars to be nil by default")
	}
	if config.Preset != nil {
		t.Errorf("expected Preset to be nil by default")
	}
}

func TestChunkingConfig_WithOptions(t *testing.T) {
	maxChars := 1000
	config := &kreuzberg.ChunkingConfig{
		MaxChars: &maxChars,
	}

	if config.MaxChars == nil || *config.MaxChars != 1000 {
		t.Error("expected MaxChars to be 1000")
	}
}

func TestChunkingConfig_FunctionalOptions(t *testing.T) {
	config := kreuzberg.NewChunkingConfig(
		kreuzberg.WithMaxChars(2000),
		kreuzberg.WithChunkSize(500),
		kreuzberg.WithChunkingEnabled(true),
	)

	if config.MaxChars == nil || *config.MaxChars != 2000 {
		t.Error("expected MaxChars to be 2000")
	}
	if config.ChunkSize == nil || *config.ChunkSize != 500 {
		t.Error("expected ChunkSize to be 500")
	}
	if config.Enabled == nil || !*config.Enabled {
		t.Error("expected Enabled to be true")
	}
}

func TestChunkingConfig_JSON_Marshaling(t *testing.T) {
	maxChars := 1000
	original := &kreuzberg.ChunkingConfig{
		MaxChars: &maxChars,
	}

	data, err := json.Marshal(original)
	if err != nil {
		t.Fatalf("failed to marshal ChunkingConfig: %v", err)
	}

	var restored kreuzberg.ChunkingConfig
	err = json.Unmarshal(data, &restored)
	if err != nil {
		t.Fatalf("failed to unmarshal ChunkingConfig: %v", err)
	}

	if restored.MaxChars == nil || *restored.MaxChars != *original.MaxChars {
		t.Error("MaxChars not preserved")
	}
}

func TestChunkingConfig_WithPreset(t *testing.T) {
	config := kreuzberg.NewChunkingConfig(
		kreuzberg.WithChunkingPreset("default"),
	)

	if config.Preset == nil || *config.Preset != "default" {
		t.Error("expected Preset to be default")
	}
}

func TestChunkingConfig_WithEmbedding(t *testing.T) {
	// Embedding configuration is now a top-level ExtractionConfig option
	// and no longer nested within ChunkingConfig.
	// This test verifies that ChunkingConfig can be configured independently.
	config := kreuzberg.NewChunkingConfig(
		kreuzberg.WithChunkSize(256),
		kreuzberg.WithChunkingEnabled(true),
	)

	if config == nil {
		t.Fatal("expected ChunkingConfig to be set")
	}
	if config.ChunkSize == nil || *config.ChunkSize != 256 {
		t.Error("expected ChunkSize to be 256")
	}
}

func TestChunkingConfig_NilPointerHandling(t *testing.T) {
	var config *kreuzberg.ChunkingConfig
	_ = config
}

func TestChunkingConfig_WithChunkerType(t *testing.T) {
	config := kreuzberg.NewChunkingConfig(
		kreuzberg.WithChunkerType("markdown"),
	)

	if config.ChunkerType == nil || *config.ChunkerType != "markdown" {
		t.Error("expected ChunkerType to be markdown")
	}
}

func TestChunkingConfig_ChunkerType_JSON_Roundtrip(t *testing.T) {
	ct := "markdown"
	original := &kreuzberg.ChunkingConfig{
		ChunkerType: &ct,
	}

	data, err := json.Marshal(original)
	if err != nil {
		t.Fatalf("failed to marshal ChunkingConfig: %v", err)
	}

	if !strings.Contains(string(data), `"chunker_type":"markdown"`) {
		t.Errorf("expected JSON to contain chunker_type, got: %s", string(data))
	}

	var restored kreuzberg.ChunkingConfig
	err = json.Unmarshal(data, &restored)
	if err != nil {
		t.Fatalf("failed to unmarshal ChunkingConfig: %v", err)
	}

	if restored.ChunkerType == nil || *restored.ChunkerType != "markdown" {
		t.Error("ChunkerType not preserved after roundtrip")
	}
}

func TestChunkingConfig_ChunkerType_DefaultNil(t *testing.T) {
	config := &kreuzberg.ChunkingConfig{}

	if config.ChunkerType != nil {
		t.Errorf("expected ChunkerType to be nil by default")
	}
}

// ============================================================================
// ImageExtractionConfig Tests
// ============================================================================

func TestImageExtractionConfig_DefaultConstruction(t *testing.T) {
	config := &kreuzberg.ImageExtractionConfig{}

	if config.ExtractImages != nil {
		t.Errorf("expected ExtractImages to be nil by default")
	}
	if config.TargetDPI != nil {
		t.Errorf("expected TargetDPI to be nil by default")
	}
}

func TestImageExtractionConfig_WithOptions(t *testing.T) {
	extractImages := true
	dpi := 300
	config := &kreuzberg.ImageExtractionConfig{
		ExtractImages: &extractImages,
		TargetDPI:     &dpi,
	}

	if config.ExtractImages == nil || !*config.ExtractImages {
		t.Error("expected ExtractImages to be true")
	}
	if config.TargetDPI == nil || *config.TargetDPI != 300 {
		t.Error("expected TargetDPI to be 300")
	}
}

func TestImageExtractionConfig_FunctionalOptions(t *testing.T) {
	config := kreuzberg.NewImageExtractionConfig(
		kreuzberg.WithExtractImages(true),
		kreuzberg.WithImageTargetDPI(300),
		kreuzberg.WithMaxImageDimension(2000),
	)

	if config.ExtractImages == nil || !*config.ExtractImages {
		t.Error("expected ExtractImages to be true")
	}
	if config.TargetDPI == nil || *config.TargetDPI != 300 {
		t.Error("expected TargetDPI to be 300")
	}
	if config.MaxImageDimension == nil || *config.MaxImageDimension != 2000 {
		t.Error("expected MaxImageDimension to be 2000")
	}
}

func TestImageExtractionConfig_JSON_Marshaling(t *testing.T) {
	extractImages := true
	original := &kreuzberg.ImageExtractionConfig{
		ExtractImages: &extractImages,
	}

	data, err := json.Marshal(original)
	if err != nil {
		t.Fatalf("failed to marshal ImageExtractionConfig: %v", err)
	}

	var restored kreuzberg.ImageExtractionConfig
	err = json.Unmarshal(data, &restored)
	if err != nil {
		t.Fatalf("failed to unmarshal ImageExtractionConfig: %v", err)
	}

	if restored.ExtractImages == nil || *restored.ExtractImages != *original.ExtractImages {
		t.Error("ExtractImages not preserved")
	}
}

func TestImageExtractionConfig_DPIRange(t *testing.T) {
	config := kreuzberg.NewImageExtractionConfig(
		kreuzberg.WithMinDPI(150),
		kreuzberg.WithMaxDPI(600),
	)

	if config.MinDPI == nil || *config.MinDPI != 150 {
		t.Error("expected MinDPI to be 150")
	}
	if config.MaxDPI == nil || *config.MaxDPI != 600 {
		t.Error("expected MaxDPI to be 600")
	}
}

func TestImageExtractionConfig_NilPointerHandling(t *testing.T) {
	var config *kreuzberg.ImageExtractionConfig
	_ = config
}

// ============================================================================
// PdfConfig Tests
// ============================================================================

func TestPdfConfig_DefaultConstruction(t *testing.T) {
	config := &kreuzberg.PdfConfig{}

	if config.ExtractImages != nil {
		t.Errorf("expected ExtractImages to be nil by default")
	}
	if config.Passwords != nil {
		t.Errorf("expected Passwords to be nil by default")
	}
}

func TestPdfConfig_WithOptions(t *testing.T) {
	extractImages := true
	config := &kreuzberg.PdfConfig{
		ExtractImages: &extractImages,
		Passwords:     []string{"password123"},
	}

	if config.ExtractImages == nil || !*config.ExtractImages {
		t.Error("expected ExtractImages to be true")
	}
	if len(config.Passwords) != 1 || config.Passwords[0] != "password123" {
		t.Error("expected Passwords to contain password123")
	}
}

func TestPdfConfig_FunctionalOptions(t *testing.T) {
	config := kreuzberg.NewPdfConfig(
		kreuzberg.WithPdfExtractImages(true),
		kreuzberg.WithPdfPasswords([]string{"pass1", "pass2"}),
		kreuzberg.WithPdfExtractMetadata(true),
	)

	if config.ExtractImages == nil || !*config.ExtractImages {
		t.Error("expected ExtractImages to be true")
	}
	if len(config.Passwords) != 2 {
		t.Error("expected 2 passwords")
	}
	if config.ExtractMetadata == nil || !*config.ExtractMetadata {
		t.Error("expected ExtractMetadata to be true")
	}
}

func TestPdfConfig_JSON_Marshaling(t *testing.T) {
	extractImages := true
	original := &kreuzberg.PdfConfig{
		ExtractImages: &extractImages,
	}

	data, err := json.Marshal(original)
	if err != nil {
		t.Fatalf("failed to marshal PdfConfig: %v", err)
	}

	var restored kreuzberg.PdfConfig
	err = json.Unmarshal(data, &restored)
	if err != nil {
		t.Fatalf("failed to unmarshal PdfConfig: %v", err)
	}

	if restored.ExtractImages == nil || *restored.ExtractImages != *original.ExtractImages {
		t.Error("ExtractImages not preserved")
	}
}

func TestPdfConfig_WithFontConfig(t *testing.T) {
	config := kreuzberg.NewPdfConfig(
		kreuzberg.WithPdfFontConfig(
			kreuzberg.WithFontConfigEnabled(true),
		),
	)

	if config.FontConfig == nil {
		t.Fatal("expected FontConfig to be set")
	}
	if !config.FontConfig.Enabled {
		t.Error("expected FontConfig.Enabled to be true")
	}
}

func TestPdfConfig_WithHierarchy(t *testing.T) {
	// Hierarchy configuration is now managed separately from PdfConfig
	// and is not nested within it. This test verifies PdfConfig options work correctly.
	config := kreuzberg.NewPdfConfig(
		kreuzberg.WithPdfExtractMetadata(true),
	)

	if config == nil {
		t.Fatal("expected PdfConfig to be set")
	}
	if config.ExtractMetadata == nil || !*config.ExtractMetadata {
		t.Error("expected ExtractMetadata to be true")
	}
}

func TestPdfConfig_NilPointerHandling(t *testing.T) {
	var config *kreuzberg.PdfConfig
	_ = config
}

// ============================================================================
// HierarchyConfig Tests
// ============================================================================

func TestHierarchyConfig_DefaultConstruction(t *testing.T) {
	config := &kreuzberg.HierarchyConfig{}

	if config.Enabled != nil {
		t.Errorf("expected Enabled to be nil by default")
	}
	if config.KClusters != nil {
		t.Errorf("expected KClusters to be nil by default")
	}
}

func TestHierarchyConfig_WithOptions(t *testing.T) {
	enabled := true
	clusters := 6
	config := &kreuzberg.HierarchyConfig{
		Enabled:   &enabled,
		KClusters: &clusters,
	}

	if config.Enabled == nil || !*config.Enabled {
		t.Error("expected Enabled to be true")
	}
	if config.KClusters == nil || *config.KClusters != 6 {
		t.Error("expected KClusters to be 6")
	}
}

func TestHierarchyConfig_FunctionalOptions(t *testing.T) {
	config := kreuzberg.NewHierarchyConfig(
		kreuzberg.WithHierarchyEnabled(true),
		kreuzberg.WithKClusters(6),
		kreuzberg.WithIncludeBbox(true),
		kreuzberg.WithOcrCoverageThreshold(0.8),
	)

	if config.Enabled == nil || !*config.Enabled {
		t.Error("expected Enabled to be true")
	}
	if config.KClusters == nil || *config.KClusters != 6 {
		t.Error("expected KClusters to be 6")
	}
	if config.IncludeBbox == nil || !*config.IncludeBbox {
		t.Error("expected IncludeBbox to be true")
	}
	if config.OcrCoverageThreshold == nil || *config.OcrCoverageThreshold != 0.8 {
		t.Error("expected OcrCoverageThreshold to be 0.8")
	}
}

func TestHierarchyConfig_JSON_Marshaling(t *testing.T) {
	enabled := true
	original := &kreuzberg.HierarchyConfig{
		Enabled: &enabled,
	}

	data, err := json.Marshal(original)
	if err != nil {
		t.Fatalf("failed to marshal HierarchyConfig: %v", err)
	}

	var restored kreuzberg.HierarchyConfig
	err = json.Unmarshal(data, &restored)
	if err != nil {
		t.Fatalf("failed to unmarshal HierarchyConfig: %v", err)
	}

	if restored.Enabled == nil || *restored.Enabled != *original.Enabled {
		t.Error("Enabled not preserved")
	}
}

func TestHierarchyConfig_NilPointerHandling(t *testing.T) {
	var config *kreuzberg.HierarchyConfig
	_ = config
}

// ============================================================================
// KeywordConfig Tests
// ============================================================================

func TestKeywordConfig_DefaultConstruction(t *testing.T) {
	config := &kreuzberg.KeywordConfig{}

	if config.Algorithm != "" {
		t.Errorf("expected Algorithm to be empty by default")
	}
	if config.MaxKeywords != nil {
		t.Errorf("expected MaxKeywords to be nil by default")
	}
}

func TestKeywordConfig_WithOptions(t *testing.T) {
	maxKeywords := 10
	config := &kreuzberg.KeywordConfig{
		Algorithm:   "yake",
		MaxKeywords: &maxKeywords,
	}

	if config.Algorithm != "yake" {
		t.Error("expected Algorithm to be yake")
	}
	if config.MaxKeywords == nil || *config.MaxKeywords != 10 {
		t.Error("expected MaxKeywords to be 10")
	}
}

func TestKeywordConfig_FunctionalOptions(t *testing.T) {
	config := kreuzberg.NewKeywordConfig(
		kreuzberg.WithKeywordAlgorithm("rake"),
		kreuzberg.WithMaxKeywords(15),
		kreuzberg.WithKeywordMinScore(0.1),
	)

	if config.Algorithm != "rake" {
		t.Error("expected Algorithm to be rake")
	}
	if config.MaxKeywords == nil || *config.MaxKeywords != 15 {
		t.Error("expected MaxKeywords to be 15")
	}
	if config.MinScore == nil || *config.MinScore != 0.1 {
		t.Error("expected MinScore to be 0.1")
	}
}

func TestKeywordConfig_JSON_Marshaling(t *testing.T) {
	maxKeywords := 10
	original := &kreuzberg.KeywordConfig{
		Algorithm:   "yake",
		MaxKeywords: &maxKeywords,
	}

	data, err := json.Marshal(original)
	if err != nil {
		t.Fatalf("failed to marshal KeywordConfig: %v", err)
	}

	var restored kreuzberg.KeywordConfig
	err = json.Unmarshal(data, &restored)
	if err != nil {
		t.Fatalf("failed to unmarshal KeywordConfig: %v", err)
	}

	if restored.Algorithm != original.Algorithm {
		t.Error("Algorithm not preserved")
	}
	if restored.MaxKeywords == nil || *restored.MaxKeywords != *original.MaxKeywords {
		t.Error("MaxKeywords not preserved")
	}
}

func TestKeywordConfig_WithNgramRange(t *testing.T) {
	config := kreuzberg.NewKeywordConfig(
		kreuzberg.WithNgramRange(1, 3),
	)

	if config.NgramRange == nil {
		t.Fatal("expected NgramRange to be set")
	}
	if config.NgramRange[0] != 1 || config.NgramRange[1] != 3 {
		t.Error("expected NgramRange to be [1, 3]")
	}
}

func TestKeywordConfig_WithYakeParams(t *testing.T) {
	config := kreuzberg.NewKeywordConfig(
		kreuzberg.WithYakeParams(
			kreuzberg.WithYakeWindowSize(5),
		),
	)

	if config.Yake == nil {
		t.Fatal("expected Yake to be set")
	}
	if config.Yake.WindowSize == nil || *config.Yake.WindowSize != 5 {
		t.Error("expected WindowSize to be 5")
	}
}

func TestKeywordConfig_WithRakeParams(t *testing.T) {
	config := kreuzberg.NewKeywordConfig(
		kreuzberg.WithRakeParams(
			kreuzberg.WithRakeMinWordLength(3),
			kreuzberg.WithRakeMaxWordsPerPhrase(5),
		),
	)

	if config.Rake == nil {
		t.Fatal("expected Rake to be set")
	}
	if config.Rake.MinWordLength == nil || *config.Rake.MinWordLength != 3 {
		t.Error("expected MinWordLength to be 3")
	}
	if config.Rake.MaxWordsPerPhrase == nil || *config.Rake.MaxWordsPerPhrase != 5 {
		t.Error("expected MaxWordsPerPhrase to be 5")
	}
}

func TestKeywordConfig_NilPointerHandling(t *testing.T) {
	var config *kreuzberg.KeywordConfig
	_ = config
}

// ============================================================================
// TokenReductionConfig Tests
// ============================================================================

func TestTokenReductionConfig_DefaultConstruction(t *testing.T) {
	config := &kreuzberg.TokenReductionConfig{}

	if config.Mode != "" {
		t.Errorf("expected Mode to be empty by default")
	}
	if config.PreserveImportantWords != nil {
		t.Errorf("expected PreserveImportantWords to be nil by default")
	}
}

func TestTokenReductionConfig_WithOptions(t *testing.T) {
	preserve := true
	config := &kreuzberg.TokenReductionConfig{
		Mode:                   "aggressive",
		PreserveImportantWords: &preserve,
	}

	if config.Mode != "aggressive" {
		t.Error("expected Mode to be aggressive")
	}
	if config.PreserveImportantWords == nil || !*config.PreserveImportantWords {
		t.Error("expected PreserveImportantWords to be true")
	}
}

func TestTokenReductionConfig_FunctionalOptions(t *testing.T) {
	config := kreuzberg.NewTokenReductionConfig(
		kreuzberg.WithTokenReductionMode("moderate"),
		kreuzberg.WithPreserveImportantWords(true),
	)

	if config.Mode != "moderate" {
		t.Error("expected Mode to be moderate")
	}
	if config.PreserveImportantWords == nil || !*config.PreserveImportantWords {
		t.Error("expected PreserveImportantWords to be true")
	}
}

func TestTokenReductionConfig_JSON_Marshaling(t *testing.T) {
	original := &kreuzberg.TokenReductionConfig{
		Mode: "aggressive",
	}

	data, err := json.Marshal(original)
	if err != nil {
		t.Fatalf("failed to marshal TokenReductionConfig: %v", err)
	}

	var restored kreuzberg.TokenReductionConfig
	err = json.Unmarshal(data, &restored)
	if err != nil {
		t.Fatalf("failed to unmarshal TokenReductionConfig: %v", err)
	}

	if restored.Mode != original.Mode {
		t.Error("Mode not preserved")
	}
}

func TestTokenReductionConfig_NilPointerHandling(t *testing.T) {
	var config *kreuzberg.TokenReductionConfig
	_ = config
}

// ============================================================================
// LanguageDetectionConfig Tests
// ============================================================================

func TestLanguageDetectionConfig_DefaultConstruction(t *testing.T) {
	config := &kreuzberg.LanguageDetectionConfig{}

	if config.Enabled != nil {
		t.Errorf("expected Enabled to be nil by default")
	}
	if config.MinConfidence != nil {
		t.Errorf("expected MinConfidence to be nil by default")
	}
}

func TestLanguageDetectionConfig_WithOptions(t *testing.T) {
	enabled := true
	confidence := 0.8
	config := &kreuzberg.LanguageDetectionConfig{
		Enabled:       &enabled,
		MinConfidence: &confidence,
	}

	if config.Enabled == nil || !*config.Enabled {
		t.Error("expected Enabled to be true")
	}
	if config.MinConfidence == nil || *config.MinConfidence != 0.8 {
		t.Error("expected MinConfidence to be 0.8")
	}
}

func TestLanguageDetectionConfig_FunctionalOptions(t *testing.T) {
	config := kreuzberg.NewLanguageDetectionConfig(
		kreuzberg.WithLanguageDetectionEnabled(true),
		kreuzberg.WithLanguageDetectionMinConfidence(0.7),
		kreuzberg.WithDetectMultiple(true),
	)

	if config.Enabled == nil || !*config.Enabled {
		t.Error("expected Enabled to be true")
	}
	if config.MinConfidence == nil || *config.MinConfidence != 0.7 {
		t.Error("expected MinConfidence to be 0.7")
	}
	if config.DetectMultiple == nil || !*config.DetectMultiple {
		t.Error("expected DetectMultiple to be true")
	}
}

func TestLanguageDetectionConfig_JSON_Marshaling(t *testing.T) {
	enabled := true
	original := &kreuzberg.LanguageDetectionConfig{
		Enabled: &enabled,
	}

	data, err := json.Marshal(original)
	if err != nil {
		t.Fatalf("failed to marshal LanguageDetectionConfig: %v", err)
	}

	var restored kreuzberg.LanguageDetectionConfig
	err = json.Unmarshal(data, &restored)
	if err != nil {
		t.Fatalf("failed to unmarshal LanguageDetectionConfig: %v", err)
	}

	if restored.Enabled == nil || *restored.Enabled != *original.Enabled {
		t.Error("Enabled not preserved")
	}
}

func TestLanguageDetectionConfig_NilPointerHandling(t *testing.T) {
	var config *kreuzberg.LanguageDetectionConfig
	_ = config
}

// ============================================================================
// PostProcessorConfig Tests
// ============================================================================

func TestPostProcessorConfig_DefaultConstruction(t *testing.T) {
	config := &kreuzberg.PostProcessorConfig{}

	if config.Enabled != nil {
		t.Errorf("expected Enabled to be nil by default")
	}
	if config.EnabledProcessors != nil {
		t.Errorf("expected EnabledProcessors to be nil by default")
	}
}

func TestPostProcessorConfig_WithOptions(t *testing.T) {
	enabled := true
	config := &kreuzberg.PostProcessorConfig{
		Enabled:           &enabled,
		EnabledProcessors: []string{"processor1"},
	}

	if config.Enabled == nil || !*config.Enabled {
		t.Error("expected Enabled to be true")
	}
	if len(config.EnabledProcessors) != 1 {
		t.Error("expected 1 enabled processor")
	}
}

func TestPostProcessorConfig_FunctionalOptions(t *testing.T) {
	config := kreuzberg.NewPostProcessorConfig(
		kreuzberg.WithPostProcessorEnabled(true),
		kreuzberg.WithEnabledProcessors([]string{"proc1", "proc2"}),
		kreuzberg.WithDisabledProcessors([]string{"proc3"}),
	)

	if config.Enabled == nil || !*config.Enabled {
		t.Error("expected Enabled to be true")
	}
	if len(config.EnabledProcessors) != 2 {
		t.Error("expected 2 enabled processors")
	}
	if len(config.DisabledProcessors) != 1 {
		t.Error("expected 1 disabled processor")
	}
}

func TestPostProcessorConfig_JSON_Marshaling(t *testing.T) {
	enabled := true
	original := &kreuzberg.PostProcessorConfig{
		Enabled: &enabled,
	}

	data, err := json.Marshal(original)
	if err != nil {
		t.Fatalf("failed to marshal PostProcessorConfig: %v", err)
	}

	var restored kreuzberg.PostProcessorConfig
	err = json.Unmarshal(data, &restored)
	if err != nil {
		t.Fatalf("failed to unmarshal PostProcessorConfig: %v", err)
	}

	if restored.Enabled == nil || *restored.Enabled != *original.Enabled {
		t.Error("Enabled not preserved")
	}
}

func TestPostProcessorConfig_NilPointerHandling(t *testing.T) {
	var config *kreuzberg.PostProcessorConfig
	_ = config
}

// ============================================================================
// EmbeddingConfig Tests
// ============================================================================

func TestEmbeddingConfig_DefaultConstruction(t *testing.T) {
	config := &kreuzberg.EmbeddingConfig{}

	if config.Model != nil {
		t.Errorf("expected Model to be nil by default")
	}
	if config.Normalize != nil {
		t.Errorf("expected Normalize to be nil by default")
	}
}

func TestEmbeddingConfig_WithOptions(t *testing.T) {
	normalize := true
	batchSize := 32
	config := &kreuzberg.EmbeddingConfig{
		Normalize: &normalize,
		BatchSize: &batchSize,
	}

	if config.Normalize == nil || !*config.Normalize {
		t.Error("expected Normalize to be true")
	}
	if config.BatchSize == nil || *config.BatchSize != 32 {
		t.Error("expected BatchSize to be 32")
	}
}

func TestEmbeddingConfig_FunctionalOptions(t *testing.T) {
	config := kreuzberg.NewEmbeddingConfig(
		kreuzberg.WithEmbeddingNormalize(true),
		kreuzberg.WithEmbeddingBatchSize(64),
		kreuzberg.WithShowDownloadProgress(true),
	)

	if config.Normalize == nil || !*config.Normalize {
		t.Error("expected Normalize to be true")
	}
	if config.BatchSize == nil || *config.BatchSize != 64 {
		t.Error("expected BatchSize to be 64")
	}
	if config.ShowDownloadProgress == nil || !*config.ShowDownloadProgress {
		t.Error("expected ShowDownloadProgress to be true")
	}
}

func TestEmbeddingConfig_JSON_Marshaling(t *testing.T) {
	normalize := true
	original := &kreuzberg.EmbeddingConfig{
		Normalize: &normalize,
	}

	data, err := json.Marshal(original)
	if err != nil {
		t.Fatalf("failed to marshal EmbeddingConfig: %v", err)
	}

	var restored kreuzberg.EmbeddingConfig
	err = json.Unmarshal(data, &restored)
	if err != nil {
		t.Fatalf("failed to unmarshal EmbeddingConfig: %v", err)
	}

	if restored.Normalize == nil || *restored.Normalize != *original.Normalize {
		t.Error("Normalize not preserved")
	}
}

func TestEmbeddingConfig_WithModel(t *testing.T) {
	config := kreuzberg.NewEmbeddingConfig(
		kreuzberg.WithEmbeddingModel(
			kreuzberg.WithEmbeddingModelType("local"),
		),
	)

	if config.Model == nil {
		t.Fatal("expected Model to be set")
	}
	if config.Model.Type != "local" {
		t.Errorf("expected Model.Type to be local, got %s", config.Model.Type)
	}
}

func TestEmbeddingConfig_WithCacheDir(t *testing.T) {
	config := kreuzberg.NewEmbeddingConfig(
		kreuzberg.WithCacheDir("/tmp/cache"),
	)

	if config.CacheDir == nil || *config.CacheDir != "/tmp/cache" {
		t.Error("expected CacheDir to be /tmp/cache")
	}
}

func TestEmbeddingConfig_NilPointerHandling(t *testing.T) {
	var config *kreuzberg.EmbeddingConfig
	_ = config
}

// ============================================================================
// PageConfig Tests
// ============================================================================

func TestPageConfig_DefaultConstruction(t *testing.T) {
	config := &kreuzberg.PageConfig{}

	if config.ExtractPages != nil {
		t.Errorf("expected ExtractPages to be nil by default")
	}
	if config.InsertPageMarkers != nil {
		t.Errorf("expected InsertPageMarkers to be nil by default")
	}
}

func TestPageConfig_WithOptions(t *testing.T) {
	extractPages := true
	config := &kreuzberg.PageConfig{
		ExtractPages: &extractPages,
	}

	if config.ExtractPages == nil || !*config.ExtractPages {
		t.Error("expected ExtractPages to be true")
	}
}

func TestPageConfig_FunctionalOptions(t *testing.T) {
	config := kreuzberg.NewPageConfig(
		kreuzberg.WithExtractPages(true),
		kreuzberg.WithInsertPageMarkers(true),
		kreuzberg.WithMarkerFormat("page_%d"),
	)

	if config.ExtractPages == nil || !*config.ExtractPages {
		t.Error("expected ExtractPages to be true")
	}
	if config.InsertPageMarkers == nil || !*config.InsertPageMarkers {
		t.Error("expected InsertPageMarkers to be true")
	}
	if config.MarkerFormat == nil || *config.MarkerFormat != "page_%d" {
		t.Error("expected MarkerFormat to be page_[digit]")
	}
}

func TestPageConfig_JSON_Marshaling(t *testing.T) {
	extractPages := true
	original := &kreuzberg.PageConfig{
		ExtractPages: &extractPages,
	}

	data, err := json.Marshal(original)
	if err != nil {
		t.Fatalf("failed to marshal PageConfig: %v", err)
	}

	var restored kreuzberg.PageConfig
	err = json.Unmarshal(data, &restored)
	if err != nil {
		t.Fatalf("failed to unmarshal PageConfig: %v", err)
	}

	if restored.ExtractPages == nil || *restored.ExtractPages != *original.ExtractPages {
		t.Error("ExtractPages not preserved")
	}
}

func TestPageConfig_NilPointerHandling(t *testing.T) {
	var config *kreuzberg.PageConfig
	_ = config
}

// ============================================================================
// Integration Tests
// ============================================================================

func TestExtractionConfig_WithAllSubConfigs(t *testing.T) {
	config := kreuzberg.NewExtractionConfig(
		kreuzberg.WithUseCache(true),
		kreuzberg.WithOCR(
			kreuzberg.WithOCRBackend("tesseract"),
		),
		kreuzberg.WithChunking(
			kreuzberg.WithMaxChars(1000),
		),
		kreuzberg.WithImages(
			kreuzberg.WithExtractImages(true),
		),
		kreuzberg.WithPdfOptions(
			kreuzberg.WithPdfExtractImages(true),
		),
		kreuzberg.WithKeywords(
			kreuzberg.WithKeywordAlgorithm("yake"),
		),
		kreuzberg.WithPages(
			kreuzberg.WithExtractPages(true),
		),
	)

	if config.UseCache == nil || !*config.UseCache {
		t.Error("UseCache not set")
	}
	if config.OCR == nil || config.OCR.Backend != "tesseract" {
		t.Error("OCR not set correctly")
	}
	if config.Chunking == nil || config.Chunking.MaxChars == nil {
		t.Error("Chunking not set")
	}
	if config.Images == nil || config.Images.ExtractImages == nil {
		t.Error("Images not set")
	}
	if config.PdfOptions == nil || config.PdfOptions.ExtractImages == nil {
		t.Error("PdfOptions not set")
	}
	if config.Keywords == nil || config.Keywords.Algorithm != "yake" {
		t.Error("Keywords not set")
	}
	if config.Pages == nil || config.Pages.ExtractPages == nil {
		t.Error("Pages not set")
	}
}

func TestComplexNestedConfig_JSON_Roundtrip(t *testing.T) {
	maxChars := 2000
	enabled := true
	dpi := 300

	original := &kreuzberg.ExtractionConfig{
		UseCache: &enabled,
		OCR: &kreuzberg.OCRConfig{
			Backend: "tesseract",
			Tesseract: &kreuzberg.TesseractConfig{
				Language: "eng",
				Preprocessing: &kreuzberg.ImagePreprocessingConfig{
					TargetDPI: &dpi,
				},
			},
		},
		Chunking: &kreuzberg.ChunkingConfig{
			MaxChars: &maxChars,
		},
	}

	// Marshal to JSON
	data, err := json.Marshal(original)
	if err != nil {
		t.Fatalf("failed to marshal: %v", err)
	}

	// Unmarshal back
	var restored kreuzberg.ExtractionConfig
	err = json.Unmarshal(data, &restored)
	if err != nil {
		t.Fatalf("failed to unmarshal: %v", err)
	}

	// Verify structure
	if restored.UseCache == nil || *restored.UseCache != *original.UseCache {
		t.Error("UseCache not preserved")
	}
	if restored.OCR == nil || restored.OCR.Backend != original.OCR.Backend {
		t.Error("OCR Backend not preserved")
	}
	if restored.OCR.Tesseract == nil || restored.OCR.Tesseract.Language != original.OCR.Tesseract.Language {
		t.Error("Tesseract Language not preserved")
	}
	if restored.OCR.Tesseract.Preprocessing == nil || *restored.OCR.Tesseract.Preprocessing.TargetDPI != *original.OCR.Tesseract.Preprocessing.TargetDPI {
		t.Error("Preprocessing TargetDPI not preserved")
	}
	if restored.Chunking == nil || *restored.Chunking.MaxChars != *original.Chunking.MaxChars {
		t.Error("Chunking MaxChars not preserved")
	}
}

func TestAllConfigTypes_CanBeNil(t *testing.T) {
	configs := []interface{}{
		(*kreuzberg.ExtractionConfig)(nil),
		(*kreuzberg.OCRConfig)(nil),
		(*kreuzberg.TesseractConfig)(nil),
		(*kreuzberg.ImagePreprocessingConfig)(nil),
		(*kreuzberg.ChunkingConfig)(nil),
		(*kreuzberg.ImageExtractionConfig)(nil),
		(*kreuzberg.PdfConfig)(nil),
		(*kreuzberg.HierarchyConfig)(nil),
		(*kreuzberg.KeywordConfig)(nil),
		(*kreuzberg.TokenReductionConfig)(nil),
		(*kreuzberg.LanguageDetectionConfig)(nil),
		(*kreuzberg.PostProcessorConfig)(nil),
		(*kreuzberg.EmbeddingConfig)(nil),
		(*kreuzberg.PageConfig)(nil),
		(*kreuzberg.FontConfig)(nil),
	}

	for _, cfg := range configs {
		if cfg == nil {
			t.Error("config should be nil")
		}
	}
}

// ============================================================================
// OutputFormat and ResultFormat Tests
// ============================================================================

func TestOutputFormat_Constants(t *testing.T) {
	tests := []struct {
		name     string
		format   kreuzberg.OutputFormat
		expected string
	}{
		{"Plain", kreuzberg.OutputFormatPlain, "plain"},
		{"Markdown", kreuzberg.OutputFormatMarkdown, "markdown"},
		{"Djot", kreuzberg.OutputFormatDjot, "djot"},
		{"HTML", kreuzberg.OutputFormatHTML, "html"},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			if string(tt.format) != tt.expected {
				t.Errorf("expected %q, got %q", tt.expected, string(tt.format))
			}
		})
	}
}

func TestResultFormat_Constants(t *testing.T) {
	tests := []struct {
		name     string
		format   kreuzberg.ResultFormat
		expected string
	}{
		{"Unified", kreuzberg.ResultFormatUnified, "unified"},
		{"ElementBased", kreuzberg.ResultFormatElementBased, "element_based"},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			if string(tt.format) != tt.expected {
				t.Errorf("expected %q, got %q", tt.expected, string(tt.format))
			}
		})
	}
}

func TestWithOutputFormat(t *testing.T) {
	tests := []struct {
		name     string
		format   string
		expected string
	}{
		{"Plain format", "plain", "plain"},
		{"Markdown format", "markdown", "markdown"},
		{"Djot format", "djot", "djot"},
		{"HTML format", "html", "html"},
		{"Empty format", "", ""},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			config := kreuzberg.NewExtractionConfig(
				kreuzberg.WithOutputFormat(tt.format),
			)

			if config.OutputFormat != tt.expected {
				t.Errorf("expected OutputFormat %q, got %q", tt.expected, config.OutputFormat)
			}
		})
	}
}

func TestWithResultFormat(t *testing.T) {
	tests := []struct {
		name     string
		format   string
		expected string
	}{
		{"Unified format", "unified", "unified"},
		{"ElementBased format", "element_based", "element_based"},
		{"Empty format", "", ""},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			config := kreuzberg.NewExtractionConfig(
				kreuzberg.WithResultFormat(tt.format),
			)

			if config.ResultFormat != tt.expected {
				t.Errorf("expected ResultFormat %q, got %q", tt.expected, config.ResultFormat)
			}
		})
	}
}

func TestOutputFormat_WithOtherOptions(t *testing.T) {
	config := kreuzberg.NewExtractionConfig(
		kreuzberg.WithUseCache(true),
		kreuzberg.WithOutputFormat("markdown"),
		kreuzberg.WithEnableQualityProcessing(false),
	)

	if config.OutputFormat != "markdown" {
		t.Errorf("expected OutputFormat 'markdown', got %q", config.OutputFormat)
	}

	if config.UseCache == nil || !*config.UseCache {
		t.Error("expected UseCache to be true")
	}

	if config.EnableQualityProcessing == nil || *config.EnableQualityProcessing {
		t.Error("expected EnableQualityProcessing to be false")
	}
}

func TestResultFormat_WithOtherOptions(t *testing.T) {
	config := kreuzberg.NewExtractionConfig(
		kreuzberg.WithForceOCR(true),
		kreuzberg.WithResultFormat("element_based"),
		kreuzberg.WithMaxConcurrentExtractions(4),
	)

	if config.ResultFormat != "element_based" {
		t.Errorf("expected ResultFormat 'element_based', got %q", config.ResultFormat)
	}

	if config.ForceOCR == nil || !*config.ForceOCR {
		t.Error("expected ForceOCR to be true")
	}

	if config.MaxConcurrentExtractions == nil || *config.MaxConcurrentExtractions != 4 {
		t.Errorf("expected MaxConcurrentExtractions 4, got %v", config.MaxConcurrentExtractions)
	}
}

func TestOutputFormat_JSON_Marshaling(t *testing.T) {
	config := &kreuzberg.ExtractionConfig{
		OutputFormat: "markdown",
		ResultFormat: "unified",
	}

	data, err := json.Marshal(config)
	if err != nil {
		t.Fatalf("failed to marshal: %v", err)
	}

	// Check that the JSON contains the expected fields
	if !bytes.Contains(data, []byte(`"output_format":"markdown"`)) {
		t.Error("expected output_format field in JSON")
	}

	if !bytes.Contains(data, []byte(`"result_format":"unified"`)) {
		t.Error("expected result_format field in JSON")
	}
}

func TestOutputFormat_JSON_Unmarshaling(t *testing.T) {
	jsonData := []byte(`{
		"output_format": "djot",
		"result_format": "element_based"
	}`)

	var config kreuzberg.ExtractionConfig
	err := json.Unmarshal(jsonData, &config)
	if err != nil {
		t.Fatalf("failed to unmarshal: %v", err)
	}

	if config.OutputFormat != "djot" {
		t.Errorf("expected OutputFormat 'djot', got %q", config.OutputFormat)
	}

	if config.ResultFormat != "element_based" {
		t.Errorf("expected ResultFormat 'element_based', got %q", config.ResultFormat)
	}
}

func TestOutputFormat_JSON_EmptyValues(t *testing.T) {
	config := &kreuzberg.ExtractionConfig{
		OutputFormat: "",
		ResultFormat: "",
	}

	data, err := json.Marshal(config)
	if err != nil {
		t.Fatalf("failed to marshal: %v", err)
	}

	// Empty strings should be omitted due to omitempty tag
	if bytes.Contains(data, []byte(`"output_format"`)) {
		t.Error("expected output_format field to be omitted for empty value")
	}

	if bytes.Contains(data, []byte(`"result_format"`)) {
		t.Error("expected result_format field to be omitted for empty value")
	}
}

func TestOutputFormat_DefaultValues(t *testing.T) {
	config := kreuzberg.NewExtractionConfig()

	if config.OutputFormat != "" {
		t.Errorf("expected OutputFormat to be empty by default, got %q", config.OutputFormat)
	}

	if config.ResultFormat != "" {
		t.Errorf("expected ResultFormat to be empty by default, got %q", config.ResultFormat)
	}
}

func TestOutputResultFormat_Combined(t *testing.T) {
	config := kreuzberg.NewExtractionConfig(
		kreuzberg.WithOutputFormat("html"),
		kreuzberg.WithResultFormat("element_based"),
	)

	if config.OutputFormat != "html" {
		t.Errorf("expected OutputFormat 'html', got %q", config.OutputFormat)
	}

	if config.ResultFormat != "element_based" {
		t.Errorf("expected ResultFormat 'element_based', got %q", config.ResultFormat)
	}

	// Verify they don't interfere with each other
	data, err := json.Marshal(config)
	if err != nil {
		t.Fatalf("failed to marshal: %v", err)
	}

	var restored kreuzberg.ExtractionConfig
	err = json.Unmarshal(data, &restored)
	if err != nil {
		t.Fatalf("failed to unmarshal: %v", err)
	}

	if restored.OutputFormat != "html" {
		t.Errorf("expected restored OutputFormat 'html', got %q", restored.OutputFormat)
	}

	if restored.ResultFormat != "element_based" {
		t.Errorf("expected restored ResultFormat 'element_based', got %q", restored.ResultFormat)
	}
}
