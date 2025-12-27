package main

import (
	"context"
	"errors"
	"fmt"
	"os"
	"path/filepath"
	"strings"

	kb "github.com/kreuzberg-dev/kreuzberg/packages/go/v4"
)

func main() {
	fmt.Println(strings.Repeat("=", 80))
	fmt.Println("KREUZBERG GO BINDINGS COMPREHENSIVE TEST SUITE")
	fmt.Println(strings.Repeat("=", 80))

	runner := NewTestRunner()

	runner.StartSection("Configuration Structs")
	testConfigurationStructs(runner)

	runner.StartSection("Pointer Helpers")
	testPointerHelpers(runner)

	runner.StartSection("Config Functions (JSON/File/Merge)")
	testConfigFunctions(runner)

	runner.StartSection("MIME Type Functions")
	testMimeTypeFunctions(runner)

	runner.StartSection("Validation Functions")
	testValidationFunctions(runner)

	runner.StartSection("Error Types and Handling")
	testErrorTypes(runner)

	runner.StartSection("FFI Error Code Functions")
	testFFIErrorCodes(runner)

	runner.StartSection("Extraction Functions (Sync)")
	testExtractionSync(runner)

	runner.StartSection("Extraction Functions (Context)")
	testExtractionContext(runner)

	runner.StartSection("Batch Extraction")
	testBatchExtraction(runner)

	runner.StartSection("Library Info")
	testLibraryInfo(runner)

	runner.StartSection("Result Types and Accessors")
	testResultTypes(runner)

	runner.StartSection("Plugin Registry Functions")
	testPluginRegistry(runner)

	runner.StartSection("Embedding Preset Functions")
	testEmbeddingPresets(runner)

	exitCode := runner.Summary()
	os.Exit(exitCode)
}

// TestRunner tracks test results and provides reporting.
type TestRunner struct {
	passed  int
	failed  int
	skipped int
	section int
}

// NewTestRunner creates a new test runner.
func NewTestRunner() *TestRunner {
	return &TestRunner{}
}

// StartSection begins a new test section.
func (tr *TestRunner) StartSection(name string) {
	tr.section++
	fmt.Printf("\n[SECTION %d] %s\n", tr.section, name)
	fmt.Println(strings.Repeat("-", 80))
}

// Test runs a single test case.
func (tr *TestRunner) Test(description string, fn func() error) {
	if err := fn(); err != nil {
		fmt.Printf("  ✗ %s\n", description)
		if err != nil {
			fmt.Printf("    Error: %v\n", err)
		}
		tr.failed++
	} else {
		fmt.Printf("  ✓ %s\n", description)
		tr.passed++
	}
}

// TestBool runs a test that returns a boolean.
func (tr *TestRunner) TestBool(description string, fn func() bool) {
	if fn() {
		fmt.Printf("  ✓ %s\n", description)
		tr.passed++
	} else {
		fmt.Printf("  ✗ %s\n", description)
		tr.failed++
	}
}

// Skip marks a test as skipped.
func (tr *TestRunner) Skip(description, reason string) {
	fmt.Printf("  ⊘ %s (skipped: %s)\n", description, reason)
	tr.skipped++
}

// Summary prints the test summary and returns the exit code.
func (tr *TestRunner) Summary() int {
	fmt.Println("\n" + strings.Repeat("=", 80))
	fmt.Println("TEST SUMMARY")
	fmt.Println(strings.Repeat("=", 80))
	total := tr.passed + tr.failed + tr.skipped
	fmt.Printf("Total Tests: %d\n", total)
	fmt.Printf("  Passed:  %d\n", tr.passed)
	fmt.Printf("  Failed:  %d\n", tr.failed)
	fmt.Printf("  Skipped: %d\n", tr.skipped)

	if tr.failed == 0 {
		fmt.Println("\n✓✓✓ ALL TESTS PASSED ✓✓✓")
		return 0
	}
	fmt.Printf("\n✗✗✗ %d TEST(S) FAILED ✗✗✗\n", tr.failed)
	return 1
}

// testConfigurationStructs tests all configuration struct initialization.
func testConfigurationStructs(tr *TestRunner) {
	tr.Test("ExtractionConfig default construction", func() error {
		cfg := &kb.ExtractionConfig{}
		if cfg == nil {
			return errors.New("config is nil")
		}
		return nil
	})

	tr.Test("ExtractionConfig with UseCache", func() error {
		cfg := &kb.ExtractionConfig{
			UseCache: kb.BoolPtr(true),
		}
		if cfg.UseCache == nil || !*cfg.UseCache {
			return errors.New("UseCache not set correctly")
		}
		return nil
	})

	tr.Test("OCRConfig construction", func() error {
		cfg := &kb.OCRConfig{
			Backend: "tesseract",
		}
		if cfg.Backend != "tesseract" {
			return errors.New("backend not set")
		}
		return nil
	})

	tr.Test("OCRConfig with Language", func() error {
		cfg := &kb.OCRConfig{
			Backend:  "tesseract",
			Language: kb.StringPtr("eng"),
		}
		if cfg.Language == nil || *cfg.Language != "eng" {
			return errors.New("language not set")
		}
		return nil
	})

	tr.Test("TesseractConfig construction", func() error {
		cfg := &kb.TesseractConfig{
			Language: "eng",
		}
		if cfg.Language != "eng" {
			return errors.New("language not set")
		}
		return nil
	})

	tr.Test("TesseractConfig with PSM", func() error {
		cfg := &kb.TesseractConfig{
			Language: "eng",
			PSM:      kb.IntPtr(6),
		}
		if cfg.PSM == nil || *cfg.PSM != 6 {
			return errors.New("PSM not set")
		}
		return nil
	})

	tr.Test("ImagePreprocessingConfig construction", func() error {
		cfg := &kb.ImagePreprocessingConfig{
			TargetDPI: kb.IntPtr(300),
		}
		if cfg.TargetDPI == nil || *cfg.TargetDPI != 300 {
			return errors.New("TargetDPI not set")
		}
		return nil
	})

	tr.Test("ChunkingConfig construction", func() error {
		cfg := &kb.ChunkingConfig{
			MaxChars: kb.IntPtr(1024),
		}
		if cfg.MaxChars == nil || *cfg.MaxChars != 1024 {
			return errors.New("MaxChars not set")
		}
		return nil
	})

	tr.Test("ImageExtractionConfig construction", func() error {
		cfg := &kb.ImageExtractionConfig{
			ExtractImages: kb.BoolPtr(true),
		}
		if cfg.ExtractImages == nil || !*cfg.ExtractImages {
			return errors.New("ExtractImages not set")
		}
		return nil
	})

	tr.Test("PdfConfig construction", func() error {
		cfg := &kb.PdfConfig{
			ExtractImages: kb.BoolPtr(true),
		}
		if cfg.ExtractImages == nil || !*cfg.ExtractImages {
			return errors.New("ExtractImages not set")
		}
		return nil
	})

	tr.Test("TokenReductionConfig construction", func() error {
		cfg := &kb.TokenReductionConfig{
			Mode: "conservative",
		}
		if cfg.Mode != "conservative" {
			return errors.New("mode not set")
		}
		return nil
	})

	tr.Test("LanguageDetectionConfig construction", func() error {
		cfg := &kb.LanguageDetectionConfig{
			Enabled: kb.BoolPtr(true),
		}
		if cfg.Enabled == nil || !*cfg.Enabled {
			return errors.New("Enabled not set")
		}
		return nil
	})

	tr.Test("KeywordConfig construction", func() error {
		cfg := &kb.KeywordConfig{
			Algorithm: "yake",
		}
		if cfg.Algorithm != "yake" {
			return errors.New("algorithm not set")
		}
		return nil
	})

	tr.Test("YakeParams construction", func() error {
		cfg := &kb.YakeParams{
			WindowSize: kb.IntPtr(2),
		}
		if cfg.WindowSize == nil || *cfg.WindowSize != 2 {
			return errors.New("WindowSize not set")
		}
		return nil
	})

	tr.Test("RakeParams construction", func() error {
		cfg := &kb.RakeParams{
			MinWordLength: kb.IntPtr(3),
		}
		if cfg.MinWordLength == nil || *cfg.MinWordLength != 3 {
			return errors.New("MinWordLength not set")
		}
		return nil
	})

	tr.Test("PostProcessorConfig construction", func() error {
		cfg := &kb.PostProcessorConfig{
			Enabled: kb.BoolPtr(true),
		}
		if cfg.Enabled == nil || !*cfg.Enabled {
			return errors.New("Enabled not set")
		}
		return nil
	})

	tr.Test("HTMLConversionOptions construction", func() error {
		cfg := &kb.HTMLConversionOptions{
			HeadingStyle: kb.StringPtr("atx"),
		}
		if cfg.HeadingStyle == nil || *cfg.HeadingStyle != "atx" {
			return errors.New("HeadingStyle not set")
		}
		return nil
	})

	tr.Test("PageConfig construction", func() error {
		cfg := &kb.PageConfig{
			ExtractPages: kb.BoolPtr(true),
		}
		if cfg.ExtractPages == nil || !*cfg.ExtractPages {
			return errors.New("ExtractPages not set")
		}
		return nil
	})

	tr.Test("EmbeddingConfig construction", func() error {
		cfg := &kb.EmbeddingConfig{
			Normalize: kb.BoolPtr(true),
		}
		if cfg.Normalize == nil || !*cfg.Normalize {
			return errors.New("Normalize not set")
		}
		return nil
	})

	tr.Test("EmbeddingModelType construction", func() error {
		cfg := &kb.EmbeddingModelType{
			Type: "sentence-transformers",
		}
		if cfg.Type != "sentence-transformers" {
			return errors.New("Type not set")
		}
		return nil
	})
}

// testPointerHelpers tests the pointer helper functions.
func testPointerHelpers(tr *TestRunner) {
	tr.Test("BoolPtr creates pointer to bool", func() error {
		ptr := kb.BoolPtr(true)
		if ptr == nil || *ptr != true {
			return errors.New("BoolPtr failed")
		}
		return nil
	})

	tr.Test("StringPtr creates pointer to string", func() error {
		ptr := kb.StringPtr("test")
		if ptr == nil || *ptr != "test" {
			return errors.New("StringPtr failed")
		}
		return nil
	})

	tr.Test("IntPtr creates pointer to int", func() error {
		ptr := kb.IntPtr(42)
		if ptr == nil || *ptr != 42 {
			return errors.New("IntPtr failed")
		}
		return nil
	})

	tr.Test("FloatPtr creates pointer to float64", func() error {
		ptr := kb.FloatPtr(3.14)
		if ptr == nil || *ptr != 3.14 {
			return errors.New("FloatPtr failed")
		}
		return nil
	})
}

// testConfigFunctions tests config manipulation functions.
func testConfigFunctions(tr *TestRunner) {
	tr.Test("IsValidJSON accepts valid JSON", func() error {
		valid := kb.IsValidJSON(`{"use_cache": true}`)
		if !valid {
			return errors.New("valid JSON rejected")
		}
		return nil
	})

	tr.Test("IsValidJSON rejects invalid JSON", func() error {
		valid := kb.IsValidJSON(`{invalid}`)
		if valid {
			return errors.New("invalid JSON accepted")
		}
		return nil
	})

	tr.Test("IsValidJSON rejects empty string", func() error {
		valid := kb.IsValidJSON("")
		if valid {
			return errors.New("empty string accepted")
		}
		return nil
	})

	tr.Test("ConfigFromJSON parses valid config", func() error {
		cfg, err := kb.ConfigFromJSON(`{"use_cache": true}`)
		if err != nil {
			return fmt.Errorf("ConfigFromJSON failed: %w", err)
		}
		if cfg == nil {
			return errors.New("config is nil")
		}
		if cfg.UseCache == nil || !*cfg.UseCache {
			return errors.New("UseCache not parsed")
		}
		return nil
	})

	tr.Test("ConfigFromJSON rejects empty JSON string", func() error {
		_, err := kb.ConfigFromJSON("")
		if err == nil {
			return errors.New("empty JSON string accepted")
		}
		return nil
	})

	tr.Test("ConfigToJSON serializes config", func() error {
		cfg := &kb.ExtractionConfig{
			UseCache: kb.BoolPtr(true),
		}
		json, err := kb.ConfigToJSON(cfg)
		if err != nil {
			return fmt.Errorf("ConfigToJSON failed: %w", err)
		}
		if json == "" {
			return errors.New("empty JSON output")
		}
		if !strings.Contains(json, "use_cache") {
			return errors.New("use_cache not in JSON")
		}
		return nil
	})

	tr.Test("ConfigGetField retrieves field value", func() error {
		cfg := &kb.ExtractionConfig{
			UseCache: kb.BoolPtr(true),
		}
		val, err := kb.ConfigGetField(cfg, "use_cache")
		if err != nil {
			return fmt.Errorf("ConfigGetField failed: %w", err)
		}
		if val == nil {
			return errors.New("field value is nil")
		}
		return nil
	})

	tr.Test("ConfigMerge combines configs", func() error {
		base := &kb.ExtractionConfig{
			UseCache: kb.BoolPtr(true),
		}
		override := &kb.ExtractionConfig{
			EnableQualityProcessing: kb.BoolPtr(false),
		}
		err := kb.ConfigMerge(base, override)
		if err != nil {
			return fmt.Errorf("ConfigMerge failed: %w", err)
		}
		if base.EnableQualityProcessing == nil || *base.EnableQualityProcessing {
			return errors.New("merge did not update fields")
		}
		return nil
	})
}

// testMimeTypeFunctions tests MIME type detection and validation.
func testMimeTypeFunctions(tr *TestRunner) {
	tr.Test("DetectMimeType recognizes PDF", func() error {
		pdfBytes, err := getTestPDFBytes()
		if err != nil {
			return fmt.Errorf("failed to get test PDF: %w", err)
		}
		mimeType, err := kb.DetectMimeType(pdfBytes)
		if err != nil {
			return fmt.Errorf("DetectMimeType failed: %w", err)
		}
		if mimeType == "" {
			return errors.New("empty mime type")
		}
		return nil
	})

	tr.Test("DetectMimeTypeFromPath works for PDF", func() error {
		testDocs := filepath.Join(filepath.Dir(os.Args[0]), "test_documents")
		pdfPath := filepath.Join(testDocs, "tiny.pdf")
		if _, err := os.Stat(pdfPath); os.IsNotExist(err) {
			return fmt.Errorf("test PDF not found: %s", pdfPath)
		}
		mimeType, err := kb.DetectMimeTypeFromPath(pdfPath)
		if err != nil {
			return fmt.Errorf("DetectMimeTypeFromPath failed: %w", err)
		}
		if mimeType == "" {
			return errors.New("empty mime type")
		}
		return nil
	})

	tr.Test("ValidateMimeType accepts valid MIME", func() error {
		result, err := kb.ValidateMimeType("application/pdf")
		if err != nil {
			return fmt.Errorf("ValidateMimeType failed: %w", err)
		}
		if result == "" {
			return errors.New("empty result")
		}
		return nil
	})

	tr.Test("GetExtensionsForMime returns extensions", func() error {
		exts, err := kb.GetExtensionsForMime("application/pdf")
		if err != nil {
			return fmt.Errorf("GetExtensionsForMime failed: %w", err)
		}
		if len(exts) == 0 {
			return errors.New("no extensions returned")
		}
		return nil
	})
}

// testValidationFunctions tests validation functions.
func testValidationFunctions(tr *TestRunner) {
	tr.Test("ValidateOCRBackend accepts 'tesseract'", func() error {
		return kb.ValidateOCRBackend("tesseract")
	})

	tr.Test("ValidateOCRBackend rejects empty string", func() error {
		err := kb.ValidateOCRBackend("")
		if err == nil {
			return errors.New("empty backend accepted")
		}
		return nil
	})

	tr.Test("ValidateLanguageCode accepts 'eng'", func() error {
		return kb.ValidateLanguageCode("eng")
	})

	tr.Test("ValidateLanguageCode accepts 'en'", func() error {
		return kb.ValidateLanguageCode("en")
	})

	tr.Test("ValidateLanguageCode rejects empty string", func() error {
		err := kb.ValidateLanguageCode("")
		if err == nil {
			return errors.New("empty code accepted")
		}
		return nil
	})

	tr.Test("ValidateTesseractPSM accepts 6", func() error {
		return kb.ValidateTesseractPSM(6)
	})

	tr.Test("ValidateTesseractPSM rejects 14", func() error {
		err := kb.ValidateTesseractPSM(14)
		if err == nil {
			return errors.New("invalid PSM accepted")
		}
		return nil
	})

	tr.Test("ValidateTesseractOEM accepts 3", func() error {
		return kb.ValidateTesseractOEM(3)
	})

	tr.Test("ValidateTesseractOEM rejects 4", func() error {
		err := kb.ValidateTesseractOEM(4)
		if err == nil {
			return errors.New("invalid OEM accepted")
		}
		return nil
	})

	tr.Test("ValidateBinarizationMethod accepts 'otsu'", func() error {
		return kb.ValidateBinarizationMethod("otsu")
	})

	tr.Test("ValidateTokenReductionLevel accepts 'aggressive'", func() error {
		return kb.ValidateTokenReductionLevel("aggressive")
	})
}

// testErrorTypes tests error type hierarchy.
func testErrorTypes(tr *TestRunner) {
	tr.TestBool("ValidationError is KreuzbergError", func() bool {
		var err kb.KreuzbergError = &kb.ValidationError{}
		return err != nil
	})

	tr.TestBool("ParsingError is KreuzbergError", func() bool {
		var err kb.KreuzbergError = &kb.ParsingError{}
		return err != nil
	})

	tr.TestBool("OCRError is KreuzbergError", func() bool {
		var err kb.KreuzbergError = &kb.OCRError{}
		return err != nil
	})

	tr.TestBool("MissingDependencyError is KreuzbergError", func() bool {
		var err kb.KreuzbergError = &kb.MissingDependencyError{}
		return err != nil
	})

	tr.TestBool("PluginError is KreuzbergError", func() bool {
		var err kb.KreuzbergError = &kb.PluginError{}
		return err != nil
	})

	tr.TestBool("IOError is KreuzbergError", func() bool {
		var err kb.KreuzbergError = &kb.IOError{}
		return err != nil
	})

	tr.TestBool("RuntimeError is KreuzbergError", func() bool {
		var err kb.KreuzbergError = &kb.RuntimeError{}
		return err != nil
	})

	tr.Test("Error.Kind() returns ErrorKind", func() error {
		err := kb.ValidateBinarizationMethod("invalid_method")
		if err == nil {
			return errors.New("expected error from validation")
		}
		kerr, ok := err.(kb.KreuzbergError)
		if !ok {
			return errors.New("error is not KreuzbergError")
		}
		if kerr.Kind() == "" {
			return errors.New("empty kind")
		}
		return nil
	})

	tr.Test("Error.Code() returns ErrorCode", func() error {
		err := &kb.ValidationError{}
		_ = err.Code()
		return nil
	})

	tr.Test("Error.PanicCtx() returns context", func() error {
		err := &kb.ValidationError{}
		_ = err.PanicCtx()
		return nil
	})
}

// testFFIErrorCodes tests FFI error code functions.
func testFFIErrorCodes(tr *TestRunner) {
	tr.Test("ErrorCodeCount returns valid count", func() error {
		count := kb.ErrorCodeCount()
		if count == 0 {
			return errors.New("zero error codes")
		}
		return nil
	})

	tr.Test("ErrorCodeName returns string", func() error {
		name := kb.ErrorCodeName(0)
		if name == "" {
			return errors.New("empty name")
		}
		return nil
	})

	tr.Test("ErrorCodeDescription returns string", func() error {
		desc := kb.ErrorCodeDescription(0)
		if desc == "" {
			return errors.New("empty description")
		}
		return nil
	})
}

// testExtractionSync tests synchronous extraction functions.
func testExtractionSync(tr *TestRunner) {
	testDocs := filepath.Join(filepath.Dir(os.Args[0]), "test_documents")
	pdfPath := filepath.Join(testDocs, "tiny.pdf")

	tr.Test("ExtractFileSync with valid PDF", func() error {
		if _, err := os.Stat(pdfPath); os.IsNotExist(err) {
			return fmt.Errorf("test PDF not found: %s", pdfPath)
		}
		result, err := kb.ExtractFileSync(pdfPath, nil)
		if err != nil {
			return fmt.Errorf("ExtractFileSync failed: %w", err)
		}
		if result == nil {
			return errors.New("result is nil")
		}
		return nil
	})

	tr.Test("ExtractFileSync with missing file", func() error {
		_, err := kb.ExtractFileSync("/nonexistent/file.pdf", nil)
		if err == nil {
			return errors.New("missing file not rejected")
		}
		return nil
	})

	tr.Test("ExtractFileSync with empty path", func() error {
		_, err := kb.ExtractFileSync("", nil)
		if err == nil {
			return errors.New("empty path not rejected")
		}
		return nil
	})

	tr.Test("ExtractBytesSync with valid PDF", func() error {
		data, err := getTestPDFBytes()
		if err != nil {
			return fmt.Errorf("failed to get PDF bytes: %w", err)
		}
		result, err := kb.ExtractBytesSync(data, "application/pdf", nil)
		if err != nil {
			return fmt.Errorf("ExtractBytesSync failed: %w", err)
		}
		if result == nil {
			return errors.New("result is nil")
		}
		return nil
	})

	tr.Test("ExtractBytesSync with config", func() error {
		data, err := getTestPDFBytes()
		if err != nil {
			return fmt.Errorf("failed to get PDF bytes: %w", err)
		}
		cfg := &kb.ExtractionConfig{
			UseCache: kb.BoolPtr(false),
		}
		result, err := kb.ExtractBytesSync(data, "application/pdf", cfg)
		if err != nil {
			return fmt.Errorf("ExtractBytesSync with config failed: %w", err)
		}
		if result == nil {
			return errors.New("result is nil")
		}
		return nil
	})
}

// testExtractionContext tests context-aware extraction functions.
func testExtractionContext(tr *TestRunner) {
	tr.Test("ExtractFileWithContext completes", func() error {
		ctx, cancel := context.WithCancel(context.Background())
		defer cancel()

		testDocs := filepath.Join(filepath.Dir(os.Args[0]), "test_documents")
		pdfPath := filepath.Join(testDocs, "tiny.pdf")

		if _, err := os.Stat(pdfPath); os.IsNotExist(err) {
			return fmt.Errorf("test PDF not found: %s", pdfPath)
		}

		result, err := kb.ExtractFileWithContext(ctx, pdfPath, nil)
		if err != nil {
			return fmt.Errorf("ExtractFileWithContext failed: %w", err)
		}
		if result == nil {
			return errors.New("result is nil")
		}
		return nil
	})

	tr.Test("ExtractBytesWithContext completes", func() error {
		ctx, cancel := context.WithCancel(context.Background())
		defer cancel()

		data, err := getTestPDFBytes()
		if err != nil {
			return fmt.Errorf("failed to get PDF bytes: %w", err)
		}
		result, err := kb.ExtractBytesWithContext(ctx, data, "application/pdf", nil)
		if err != nil {
			return fmt.Errorf("ExtractBytesWithContext failed: %w", err)
		}
		if result == nil {
			return errors.New("result is nil")
		}
		return nil
	})
}

// testBatchExtraction tests batch extraction functions.
func testBatchExtraction(tr *TestRunner) {
	tr.Test("BatchExtractFilesSync with multiple files", func() error {
		testDocs := filepath.Join(filepath.Dir(os.Args[0]), "test_documents")
		pdfPath := filepath.Join(testDocs, "tiny.pdf")

		if _, err := os.Stat(pdfPath); os.IsNotExist(err) {
			return fmt.Errorf("test PDF not found: %s", pdfPath)
		}

		paths := []string{pdfPath}
		results, err := kb.BatchExtractFilesSync(paths, nil)
		if err != nil {
			return fmt.Errorf("BatchExtractFilesSync failed: %w", err)
		}
		if len(results) != len(paths) {
			return errors.New("result count mismatch")
		}
		return nil
	})

	tr.Test("BatchExtractBytesSync with multiple items", func() error {
		data, err := getTestPDFBytes()
		if err != nil {
			return fmt.Errorf("failed to get PDF bytes: %w", err)
		}

		items := []kb.BytesWithMime{
			{Data: data, MimeType: "application/pdf"},
		}
		results, err := kb.BatchExtractBytesSync(items, nil)
		if err != nil {
			return fmt.Errorf("BatchExtractBytesSync failed: %w", err)
		}
		if len(results) != len(items) {
			return errors.New("result count mismatch")
		}
		return nil
	})
}

// testLibraryInfo tests library info functions.
func testLibraryInfo(tr *TestRunner) {
	tr.Test("LibraryVersion returns non-empty string", func() error {
		version := kb.LibraryVersion()
		if version == "" {
			return errors.New("empty version")
		}
		return nil
	})

	tr.Test("LastErrorCode returns valid code", func() error {
		code := kb.LastErrorCode()
		_ = code
		return nil
	})

	tr.Test("LastPanicContext returns context or nil", func() error {
		ctx := kb.LastPanicContext()
		_ = ctx
		return nil
	})
}

// testResultTypes tests result struct types and accessors.
func testResultTypes(tr *TestRunner) {
	tr.Test("ExtractionResult has Content field", func() error {
		result := &kb.ExtractionResult{
			Content: "test",
		}
		if result.Content != "test" {
			return errors.New("Content field not set")
		}
		return nil
	})

	tr.Test("ExtractionResult has MimeType field", func() error {
		result := &kb.ExtractionResult{
			MimeType: "application/pdf",
		}
		if result.MimeType != "application/pdf" {
			return errors.New("MimeType field not set")
		}
		return nil
	})

	tr.Test("ExtractionResult has Metadata field", func() error {
		result := &kb.ExtractionResult{
			Metadata: kb.Metadata{},
		}
		_ = result.Metadata
		return nil
	})

	tr.Test("Metadata.FormatType returns FormatType", func() error {
		meta := kb.Metadata{}
		ft := meta.FormatType()
		_ = ft
		return nil
	})

	tr.Test("Metadata.PdfMetadata accessor works", func() error {
		meta := kb.Metadata{}
		_, ok := meta.PdfMetadata()
		_ = ok
		return nil
	})

	tr.Test("Table struct construction", func() error {
		table := &kb.Table{
			Cells:      [][]string{{"a", "b"}},
			Markdown:   "| a | b |",
			PageNumber: 1,
		}
		if len(table.Cells) != 1 {
			return errors.New("Cells not set")
		}
		return nil
	})

	tr.Test("Chunk struct construction", func() error {
		chunk := &kb.Chunk{
			Content: "test chunk",
			Metadata: kb.ChunkMetadata{
				ByteStart: 0,
				ByteEnd:   10,
			},
		}
		if chunk.Content != "test chunk" {
			return errors.New("Content not set")
		}
		return nil
	})

	tr.Test("ExtractedImage struct construction", func() error {
		img := &kb.ExtractedImage{
			Data:       []byte("fake image"),
			Format:     "jpeg",
			ImageIndex: 0,
		}
		if len(img.Data) == 0 {
			return errors.New("Data not set")
		}
		return nil
	})
}

// testPluginRegistry tests plugin registry functions.
func testPluginRegistry(tr *TestRunner) {
	tr.Test("ListOCRBackends returns list", func() error {
		backends, err := kb.ListOCRBackends()
		if err != nil {
			return fmt.Errorf("ListOCRBackends failed: %w", err)
		}
		if backends == nil {
			return errors.New("backends list is nil")
		}
		return nil
	})

	tr.Test("ListPostProcessors returns list", func() error {
		processors, err := kb.ListPostProcessors()
		if err != nil {
			return fmt.Errorf("ListPostProcessors failed: %w", err)
		}
		if processors == nil {
			return errors.New("processors list is nil")
		}
		return nil
	})

	tr.Test("ListValidators returns list", func() error {
		validators, err := kb.ListValidators()
		if err != nil {
			return fmt.Errorf("ListValidators failed: %w", err)
		}
		if validators == nil {
			return errors.New("validators list is nil")
		}
		return nil
	})

	tr.Test("ListDocumentExtractors returns list", func() error {
		extractors, err := kb.ListDocumentExtractors()
		if err != nil {
			return fmt.Errorf("ListDocumentExtractors failed: %w", err)
		}
		if extractors == nil {
			return errors.New("extractors list is nil")
		}
		return nil
	})
}

// testEmbeddingPresets tests embedding preset functions.
func testEmbeddingPresets(tr *TestRunner) {
	tr.Test("ListEmbeddingPresets returns list", func() error {
		presets, err := kb.ListEmbeddingPresets()
		if err != nil {
			return fmt.Errorf("ListEmbeddingPresets failed: %w", err)
		}
		if presets == nil {
			return errors.New("presets list is nil")
		}
		return nil
	})

	tr.Test("GetEmbeddingPreset returns valid preset", func() error {
		presets, err := kb.ListEmbeddingPresets()
		if err != nil {
			return fmt.Errorf("ListEmbeddingPresets failed: %w", err)
		}
		if len(presets) == 0 {
			return errors.New("no presets available")
		}
		preset, err := kb.GetEmbeddingPreset(presets[0])
		if err != nil {
			return fmt.Errorf("GetEmbeddingPreset failed: %w", err)
		}
		if preset == nil {
			return errors.New("preset is nil")
		}
		return nil
	})
}

// getTestPDFBytes returns bytes from the test PDF file.
func getTestPDFBytes() ([]byte, error) {
	testDocs := filepath.Join(filepath.Dir(os.Args[0]), "test_documents")
	pdfPath := filepath.Join(testDocs, "tiny.pdf")
	return os.ReadFile(pdfPath)
}
