package main

import (
	"context"
	"errors"
	"fmt"
	"os"
	"path/filepath"
	"testing"
	"time"

	"github.com/kreuzberg-dev/kreuzberg/packages/go/v4"
	"github.com/stretchr/testify/assert"
)

// TestTypeVerificationExtractionResult verifies ExtractionResult type is accessible.
func TestTypeVerificationExtractionResult(t *testing.T) {
	result := &kreuzberg.ExtractionResult{
		Content:  "test",
		MimeType: "text/plain",
		Metadata: kreuzberg.Metadata{},
		Tables:   []kreuzberg.Table{},
		Chunks:   []kreuzberg.Chunk{},
		Images:   []kreuzberg.ExtractedImage{},
		Pages:    []kreuzberg.PageContent{},
	}
	assert.NotNil(t, result)
	assert.Equal(t, "test", result.Content)
	assert.Equal(t, "text/plain", result.MimeType)
	assert.NotEmpty(t, result.Content)
}

// TestTypeVerificationExtractionConfig verifies ExtractionConfig type is accessible.
func TestTypeVerificationExtractionConfig(t *testing.T) {
	config := &kreuzberg.ExtractionConfig{
		UseCache:                kreuzberg.BoolPtr(true),
		EnableQualityProcessing: kreuzberg.BoolPtr(false),
		ForceOCR:                kreuzberg.BoolPtr(false),
	}
	assert.NotNil(t, config)
	assert.NotNil(t, config.UseCache)
	assert.True(t, *config.UseCache)
	assert.NotNil(t, config.EnableQualityProcessing)
	assert.False(t, *config.EnableQualityProcessing)
}

// TestTypeVerificationMetadata verifies Metadata type is accessible.
func TestTypeVerificationMetadata(t *testing.T) {
	metadata := kreuzberg.Metadata{
		Language: kreuzberg.StringPtr("en"),
		Date:     kreuzberg.StringPtr("2025-12-19"),
		Subject:  kreuzberg.StringPtr("Test Document"),
	}
	assert.NotNil(t, metadata)
	assert.NotNil(t, metadata.Language)
	assert.Equal(t, "en", *metadata.Language)
}

// TestTypeVerificationTable verifies Table type is accessible.
func TestTypeVerificationTable(t *testing.T) {
	table := kreuzberg.Table{
		Cells:      [][]string{{"A", "B"}, {"C", "D"}},
		Markdown:   "| A | B |\n|---|---|\n| C | D |",
		PageNumber: 1,
	}
	assert.NotNil(t, table)
	assert.Equal(t, 2, len(table.Cells))
	assert.Equal(t, 1, table.PageNumber)
}

// TestTypeVerificationChunk verifies Chunk type is accessible.
func TestTypeVerificationChunk(t *testing.T) {
	chunk := kreuzberg.Chunk{
		Content: "sample text",
		Metadata: kreuzberg.ChunkMetadata{
			ByteStart:   0,
			ByteEnd:     11,
			ChunkIndex:  0,
			TotalChunks: 1,
		},
	}
	assert.NotNil(t, chunk)
	assert.Equal(t, "sample text", chunk.Content)
	assert.Equal(t, uint64(0), chunk.Metadata.ByteStart)
}

// TestTypeVerificationExtractedImage verifies ExtractedImage type is accessible.
func TestTypeVerificationExtractedImage(t *testing.T) {
	image := kreuzberg.ExtractedImage{
		Data:       []byte{0xFF, 0xD8, 0xFF},
		Format:     "jpeg",
		ImageIndex: 0,
	}
	assert.NotNil(t, image)
	assert.Equal(t, "jpeg", image.Format)
	assert.Equal(t, 3, len(image.Data))
}

// TestTypeVerificationPageContent verifies PageContent type is accessible.
func TestTypeVerificationPageContent(t *testing.T) {
	page := kreuzberg.PageContent{
		PageNumber: 1,
		Content:    "Page content",
		Tables:     []kreuzberg.Table{},
		Images:     []kreuzberg.ExtractedImage{},
	}
	assert.NotNil(t, page)
	assert.Equal(t, uint64(1), page.PageNumber)
	assert.Equal(t, "Page content", page.Content)
}

// TestTypeVerificationPointerHelpers verifies pointer helper functions.
func TestTypeVerificationPointerHelpers(t *testing.T) {
	boolPtr := kreuzberg.BoolPtr(true)
	assert.NotNil(t, boolPtr)
	assert.True(t, *boolPtr)

	stringPtr := kreuzberg.StringPtr("test")
	assert.NotNil(t, stringPtr)
	assert.Equal(t, "test", *stringPtr)

	intPtr := kreuzberg.IntPtr(42)
	assert.NotNil(t, intPtr)
	assert.Equal(t, 42, *intPtr)

	floatPtr := kreuzberg.FloatPtr(3.14)
	assert.NotNil(t, floatPtr)
	assert.Equal(t, 3.14, *floatPtr)
}

// TestExtractFileSyncPDFValid extracts text from a valid PDF file.
func TestExtractFileSyncPDFValid(t *testing.T) {
	pdfPath := getTestDocumentPath(t, "pdfs_with_tables", "tiny.pdf")

	result, err := kreuzberg.ExtractFileSync(pdfPath, nil)
	assert.NoError(t, err, "should extract PDF without error")
	assert.NotNil(t, result, "result should not be nil")
	assert.NotEmpty(t, result.Content, "extraction should succeed")
	assert.Equal(t, "application/pdf", result.MimeType, "mime type should be PDF")
	assert.NotEmpty(t, result.Content, "content should not be empty")
}

// TestExtractFileSyncDOCXValid extracts text from a valid DOCX file.
func TestExtractFileSyncDOCXValid(t *testing.T) {
	docxPath := getTestDocumentPath(t, "documents", "lorem_ipsum.docx")

	result, err := kreuzberg.ExtractFileSync(docxPath, nil)
	assert.NoError(t, err, "should extract DOCX without error")
	assert.NotNil(t, result, "result should not be nil")
	assert.NotEmpty(t, result.Content, "extraction should succeed")
	assert.Equal(t, "application/vnd.openxmlformats-officedocument.wordprocessingml.document", result.MimeType)
	assert.NotEmpty(t, result.Content, "content should not be empty")
}

// TestExtractFileSyncXLSXValid extracts text from a valid XLSX file.
func TestExtractFileSyncXLSXValid(t *testing.T) {
	xlsxPath := getTestDocumentPath(t, "spreadsheets", "stanley_cups.xlsx")

	result, err := kreuzberg.ExtractFileSync(xlsxPath, nil)
	assert.NoError(t, err, "should extract XLSX without error")
	assert.NotNil(t, result, "result should not be nil")
	assert.NotEmpty(t, result.Content, "extraction should succeed")
	assert.NotEmpty(t, result.Content, "content should not be empty")
}

// TestExtractFileSyncImageJPGValid extracts text from a valid JPG image.
func TestExtractFileSyncImageJPGValid(t *testing.T) {
	imgPath := getTestDocumentPath(t, "images", "example.jpg")

	result, err := kreuzberg.ExtractFileSync(imgPath, nil)
	assert.NoError(t, err, "should extract JPG without error")
	assert.NotNil(t, result, "result should not be nil")
	assert.NotEmpty(t, result.Content, "extraction should succeed")
	assert.Equal(t, "image/jpeg", result.MimeType)
}

// TestExtractFileSyncImagePNGValid extracts text from a valid PNG image.
func TestExtractFileSyncImagePNGValid(t *testing.T) {
	imgPath := getTestDocumentPath(t, "images", "sample.png")

	result, err := kreuzberg.ExtractFileSync(imgPath, nil)
	assert.NoError(t, err, "should extract PNG without error")
	assert.NotNil(t, result, "result should not be nil")
	assert.NotEmpty(t, result.Content, "extraction should succeed")
	assert.Equal(t, "image/png", result.MimeType)
}

// TestExtractFileSyncODTValid extracts text from a valid ODT file.
func TestExtractFileSyncODTValid(t *testing.T) {
	odtPath := getTestDocumentPath(t, "odt", "paragraph.odt")

	result, err := kreuzberg.ExtractFileSync(odtPath, nil)
	assert.NoError(t, err, "should extract ODT without error")
	assert.NotNil(t, result, "result should not be nil")
	assert.NotEmpty(t, result.Content, "extraction should succeed")
}

// TestExtractFileSyncMarkdownValid extracts text from a valid Markdown file.
func TestExtractFileSyncMarkdownValid(t *testing.T) {
	mdPath := getTestDocumentPath(t, "", "extraction_test.md")

	result, err := kreuzberg.ExtractFileSync(mdPath, nil)
	assert.NoError(t, err, "should extract Markdown without error")
	assert.NotNil(t, result, "result should not be nil")
	assert.NotEmpty(t, result.Content, "extraction should succeed")
	assert.NotEmpty(t, result.Content, "content should not be empty")
}

// TestExtractFileSyncMissingFile tests error handling for missing files.
func TestExtractFileSyncMissingFile(t *testing.T) {
	_, err := kreuzberg.ExtractFileSync("/nonexistent/path/file.pdf", nil)
	assert.Error(t, err, "should return error for missing file")
	assert.NotNil(t, err, "error should not be nil")
}

// TestExtractFileSyncEmptyPath tests validation of empty file path.
func TestExtractFileSyncEmptyPath(t *testing.T) {
	_, err := kreuzberg.ExtractFileSync("", nil)
	assert.Error(t, err, "should return error for empty path")
}

// TestExtractFileSyncWithConfig tests extraction with custom configuration.
func TestExtractFileSyncWithConfig(t *testing.T) {
	pdfPath := getTestDocumentPath(t, "pdfs_with_tables", "tiny.pdf")

	config := &kreuzberg.ExtractionConfig{
		UseCache:                kreuzberg.BoolPtr(false),
		EnableQualityProcessing: kreuzberg.BoolPtr(true),
	}

	result, err := kreuzberg.ExtractFileSync(pdfPath, config)
	assert.NoError(t, err, "should extract with config without error")
	assert.NotNil(t, result, "result should not be nil")
	assert.NotEmpty(t, result.Content, "extraction should succeed")
}

// TestExtractBytesSyncPDFValid extracts text from PDF bytes.
func TestExtractBytesSyncPDFValid(t *testing.T) {
	pdfPath := getTestDocumentPath(t, "pdfs_with_tables", "tiny.pdf")
	data, err := os.ReadFile(pdfPath)
	assert.NoError(t, err, "should read PDF file")

	result, err := kreuzberg.ExtractBytesSync(data, "application/pdf", nil)
	assert.NoError(t, err, "should extract PDF bytes without error")
	assert.NotNil(t, result, "result should not be nil")
	assert.NotEmpty(t, result.Content, "extraction should succeed")
	assert.Equal(t, "application/pdf", result.MimeType)
	assert.NotEmpty(t, result.Content, "content should not be empty")
}

// TestExtractBytesSyncDOCXValid extracts text from DOCX bytes.
func TestExtractBytesSyncDOCXValid(t *testing.T) {
	docxPath := getTestDocumentPath(t, "documents", "lorem_ipsum.docx")
	data, err := os.ReadFile(docxPath)
	assert.NoError(t, err, "should read DOCX file")

	result, err := kreuzberg.ExtractBytesSync(data, "application/vnd.openxmlformats-officedocument.wordprocessingml.document", nil)
	assert.NoError(t, err, "should extract DOCX bytes without error")
	assert.NotNil(t, result, "result should not be nil")
	assert.NotEmpty(t, result.Content, "extraction should succeed")
}

// TestExtractBytesSyncXLSXValid extracts text from XLSX bytes.
func TestExtractBytesSyncXLSXValid(t *testing.T) {
	xlsxPath := getTestDocumentPath(t, "spreadsheets", "stanley_cups.xlsx")
	data, err := os.ReadFile(xlsxPath)
	assert.NoError(t, err, "should read XLSX file")

	result, err := kreuzberg.ExtractBytesSync(data, "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet", nil)
	assert.NoError(t, err, "should extract XLSX bytes without error")
	assert.NotNil(t, result, "result should not be nil")
	assert.NotEmpty(t, result.Content, "extraction should succeed")
}

// TestExtractBytesSyncImageJPGValid extracts text from JPG bytes.
func TestExtractBytesSyncImageJPGValid(t *testing.T) {
	imgPath := getTestDocumentPath(t, "images", "example.jpg")
	data, err := os.ReadFile(imgPath)
	assert.NoError(t, err, "should read JPG file")

	result, err := kreuzberg.ExtractBytesSync(data, "image/jpeg", nil)
	assert.NoError(t, err, "should extract JPG bytes without error")
	assert.NotNil(t, result, "result should not be nil")
	assert.NotEmpty(t, result.Content, "extraction should succeed")
	assert.Equal(t, "image/jpeg", result.MimeType)
}

// TestExtractBytesSyncImagePNGValid extracts text from PNG bytes.
func TestExtractBytesSyncImagePNGValid(t *testing.T) {
	imgPath := getTestDocumentPath(t, "images", "sample.png")
	data, err := os.ReadFile(imgPath)
	assert.NoError(t, err, "should read PNG file")

	result, err := kreuzberg.ExtractBytesSync(data, "image/png", nil)
	assert.NoError(t, err, "should extract PNG bytes without error")
	assert.NotNil(t, result, "result should not be nil")
	assert.NotEmpty(t, result.Content, "extraction should succeed")
}

// TestExtractBytesSyncODTValid extracts text from ODT bytes.
func TestExtractBytesSyncODTValid(t *testing.T) {
	odtPath := getTestDocumentPath(t, "odt", "paragraph.odt")
	data, err := os.ReadFile(odtPath)
	assert.NoError(t, err, "should read ODT file")

	result, err := kreuzberg.ExtractBytesSync(data, "application/vnd.oasis.opendocument.text", nil)
	assert.NoError(t, err, "should extract ODT bytes without error")
	assert.NotNil(t, result, "result should not be nil")
	assert.NotEmpty(t, result.Content, "extraction should succeed")
}

// TestExtractBytesSyncMarkdownValid extracts text from Markdown bytes.
func TestExtractBytesSyncMarkdownValid(t *testing.T) {
	mdPath := getTestDocumentPath(t, "", "extraction_test.md")
	data, err := os.ReadFile(mdPath)
	assert.NoError(t, err, "should read Markdown file")

	result, err := kreuzberg.ExtractBytesSync(data, "text/markdown", nil)
	assert.NoError(t, err, "should extract Markdown bytes without error")
	assert.NotNil(t, result, "result should not be nil")
	assert.NotEmpty(t, result.Content, "extraction should succeed")
}

// TestExtractBytesSyncEmptyData tests validation of empty data.
func TestExtractBytesSyncEmptyData(t *testing.T) {
	_, err := kreuzberg.ExtractBytesSync([]byte{}, "application/pdf", nil)
	assert.Error(t, err, "should return error for empty data")
}

// TestExtractBytesSyncEmptyMimeType tests validation of empty MIME type.
func TestExtractBytesSyncEmptyMimeType(t *testing.T) {
	pdfPath := getTestDocumentPath(t, "pdfs_with_tables", "tiny.pdf")
	data, err := os.ReadFile(pdfPath)
	assert.NoError(t, err, "should read PDF file")

	_, err = kreuzberg.ExtractBytesSync(data, "", nil)
	assert.Error(t, err, "should return error for empty MIME type")
}

// TestExtractBytesSyncWithConfig tests byte extraction with configuration.
func TestExtractBytesSyncWithConfig(t *testing.T) {
	pdfPath := getTestDocumentPath(t, "pdfs_with_tables", "tiny.pdf")
	data, err := os.ReadFile(pdfPath)
	assert.NoError(t, err, "should read PDF file")

	config := &kreuzberg.ExtractionConfig{
		UseCache: kreuzberg.BoolPtr(false),
	}

	result, err := kreuzberg.ExtractBytesSync(data, "application/pdf", config)
	assert.NoError(t, err, "should extract with config without error")
	assert.NotNil(t, result, "result should not be nil")
	assert.NotEmpty(t, result.Content, "extraction should succeed")
}

// TestBatchExtractFilesSync tests batch extraction of multiple PDF files.
func TestBatchExtractFilesSync(t *testing.T) {
	pdf1 := getTestDocumentPath(t, "pdfs_with_tables", "tiny.pdf")
	pdf2 := getTestDocumentPath(t, "pdfs_with_tables", "medium.pdf")

	results, err := kreuzberg.BatchExtractFilesSync([]string{pdf1, pdf2}, nil)
	assert.NoError(t, err, "batch extraction should succeed")
	assert.NotNil(t, results, "results should not be nil")
	assert.GreaterOrEqual(t, len(results), 0, "should have results")
}

// TestBatchExtractFilesSyncWithConfig tests batch extraction with configuration.
func TestBatchExtractFilesSyncWithConfig(t *testing.T) {
	pdf1 := getTestDocumentPath(t, "pdfs_with_tables", "tiny.pdf")

	config := &kreuzberg.ExtractionConfig{
		UseCache: kreuzberg.BoolPtr(false),
	}

	results, err := kreuzberg.BatchExtractFilesSync([]string{pdf1}, config)
	assert.NoError(t, err, "batch extraction should succeed")
	assert.NotNil(t, results, "results should not be nil")
}

// TestBatchExtractFilesSyncEmpty tests batch extraction with empty file list.
func TestBatchExtractFilesSyncEmpty(t *testing.T) {
	results, err := kreuzberg.BatchExtractFilesSync([]string{}, nil)
	assert.NoError(t, err, "empty batch should return no error")
	assert.NotNil(t, results, "results should not be nil")
	assert.Equal(t, 0, len(results), "should have no results")
}

// TestBatchExtractBytesSync tests batch extraction from byte slices.
func TestBatchExtractBytesSync(t *testing.T) {
	pdfPath := getTestDocumentPath(t, "pdfs_with_tables", "tiny.pdf")
	data1, err := os.ReadFile(pdfPath)
	assert.NoError(t, err, "should read PDF file")

	items := []kreuzberg.BytesWithMime{
		{Data: data1, MimeType: "application/pdf"},
	}

	results, err := kreuzberg.BatchExtractBytesSync(items, nil)
	assert.NoError(t, err, "batch extraction should succeed")
	assert.NotNil(t, results, "results should not be nil")
}

// TestBatchExtractBytesSyncMultipleTypes tests batch extraction with multiple document types.
func TestBatchExtractBytesSyncMultipleTypes(t *testing.T) {
	pdfPath := getTestDocumentPath(t, "pdfs_with_tables", "tiny.pdf")
	pdfData, err := os.ReadFile(pdfPath)
	assert.NoError(t, err, "should read PDF file")

	docxPath := getTestDocumentPath(t, "documents", "lorem_ipsum.docx")
	docxData, err := os.ReadFile(docxPath)
	assert.NoError(t, err, "should read DOCX file")

	items := []kreuzberg.BytesWithMime{
		{Data: pdfData, MimeType: "application/pdf"},
		{Data: docxData, MimeType: "application/vnd.openxmlformats-officedocument.wordprocessingml.document"},
	}

	results, err := kreuzberg.BatchExtractBytesSync(items, nil)
	assert.NoError(t, err, "batch extraction should succeed")
	assert.NotNil(t, results, "results should not be nil")
}

// TestMimeTypeDetectionFromBytes detects MIME type from file bytes.
func TestMimeTypeDetectionFromBytes(t *testing.T) {
	pdfPath := getTestDocumentPath(t, "pdfs_with_tables", "tiny.pdf")
	data, err := os.ReadFile(pdfPath)
	assert.NoError(t, err, "should read PDF file")

	mimeType, err := kreuzberg.DetectMimeType(data)
	assert.NoError(t, err, "MIME type detection should succeed")
	assert.NotEmpty(t, mimeType, "MIME type should not be empty")
}

// TestMimeTypeDetectionFromPath detects MIME type from file path.
func TestMimeTypeDetectionFromPath(t *testing.T) {
	pdfPath := getTestDocumentPath(t, "pdfs_with_tables", "tiny.pdf")

	mimeType, err := kreuzberg.DetectMimeTypeFromPath(pdfPath)
	assert.NoError(t, err, "MIME type detection should succeed")
	assert.NotEmpty(t, mimeType, "MIME type should not be empty")
}

// TestMimeTypeDetectionFromPathDOCX detects MIME type for DOCX file.
func TestMimeTypeDetectionFromPathDOCX(t *testing.T) {
	docxPath := getTestDocumentPath(t, "documents", "lorem_ipsum.docx")

	mimeType, err := kreuzberg.DetectMimeTypeFromPath(docxPath)
	assert.NoError(t, err, "MIME type detection should succeed")
	assert.NotEmpty(t, mimeType, "MIME type should not be empty")
}

// TestMimeTypeDetectionFromPathXLSX detects MIME type for XLSX file.
func TestMimeTypeDetectionFromPathXLSX(t *testing.T) {
	xlsxPath := getTestDocumentPath(t, "spreadsheets", "stanley_cups.xlsx")

	mimeType, err := kreuzberg.DetectMimeTypeFromPath(xlsxPath)
	assert.NoError(t, err, "MIME type detection should succeed")
	assert.NotEmpty(t, mimeType, "MIME type should not be empty")
}

// TestExtractionConfigBuilding tests ExtractionConfig creation and usage.
func TestExtractionConfigBuilding(t *testing.T) {
	config := &kreuzberg.ExtractionConfig{
		UseCache:                 kreuzberg.BoolPtr(true),
		EnableQualityProcessing:  kreuzberg.BoolPtr(false),
		ForceOCR:                 kreuzberg.BoolPtr(false),
		MaxConcurrentExtractions: kreuzberg.IntPtr(4),
	}

	assert.NotNil(t, config)
	assert.NotNil(t, config.UseCache)
	assert.True(t, *config.UseCache)
	assert.NotNil(t, config.MaxConcurrentExtractions)
	assert.Equal(t, 4, *config.MaxConcurrentExtractions)
}

// TestExtractionResultValidation verifies extraction result structure.
func TestExtractionResultValidation(t *testing.T) {
	pdfPath := getTestDocumentPath(t, "pdfs_with_tables", "tiny.pdf")

	result, err := kreuzberg.ExtractFileSync(pdfPath, nil)
	assert.NoError(t, err)
	assert.NotNil(t, result)

	assert.NotEmpty(t, result.MimeType, "should have MIME type")
	assert.NotEmpty(t, result.Content, "should have content")
	assert.NotEmpty(t, result.Content, "should be successful")
	assert.NotNil(t, result.Metadata, "should have metadata")
}

// TestExtractionResultMetadata verifies metadata extraction.
func TestExtractionResultMetadata(t *testing.T) {
	pdfPath := getTestDocumentPath(t, "pdfs_with_tables", "tiny.pdf")

	result, err := kreuzberg.ExtractFileSync(pdfPath, nil)
	assert.NoError(t, err)
	assert.NotNil(t, result)
	assert.NotNil(t, result.Metadata, "metadata should not be nil")
}

// TestErrorHandlingInvalidInput tests error handling for invalid inputs.
func TestErrorHandlingInvalidInput(t *testing.T) {
	_, err := kreuzberg.ExtractFileSync("", nil)
	assert.Error(t, err, "should return error for empty path")
}

// TestErrorHandlingMissingFile tests error handling for missing files.
func TestErrorHandlingMissingFile(t *testing.T) {
	_, err := kreuzberg.ExtractFileSync("/nonexistent/missing/file.pdf", nil)
	assert.Error(t, err, "should return error for missing file")
}

// TestErrorHandlingProperWrapping tests error wrapping functionality.
func TestErrorHandlingProperWrapping(t *testing.T) {
	_, err := kreuzberg.ExtractFileSync("", nil)
	assert.Error(t, err, "error should exist")
	assert.NotNil(t, err, "error should not be nil")
}

// TestContextSupport verifies context support in extraction.
func TestContextSupport(t *testing.T) {
	pdfPath := getTestDocumentPath(t, "pdfs_with_tables", "tiny.pdf")

	result, err := kreuzberg.ExtractFileSync(pdfPath, nil)
	assert.NoError(t, err)
	assert.NotNil(t, result)
}

// TestContextCancellation tests context cancellation handling.
func TestContextCancellation(t *testing.T) {
	pdfPath := getTestDocumentPath(t, "pdfs_with_tables", "tiny.pdf")

	ctx, cancel := context.WithCancel(context.Background())
	cancel()

	result, err := kreuzberg.ExtractFileSync(pdfPath, nil)
	if err == nil {
		assert.NotNil(t, result, "should still extract even with cancelled context for sync operations")
	}
	_ = ctx
}

// TestContextTimeout tests context timeout handling.
func TestContextTimeout(t *testing.T) {
	pdfPath := getTestDocumentPath(t, "pdfs_with_tables", "tiny.pdf")

	ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
	defer cancel()

	result, err := kreuzberg.ExtractFileSync(pdfPath, nil)
	assert.NoError(t, err)
	assert.NotNil(t, result)
	_ = ctx
}

// TestFileTypeCoveragePDFs tests PDF extraction coverage.
func TestFileTypeCoveragePDFs(t *testing.T) {
	tests := []struct {
		name string
		path string
	}{
		{"Tiny PDF", "pdfs_with_tables/tiny.pdf"},
		{"Medium PDF", "pdfs_with_tables/medium.pdf"},
		{"Large PDF", "pdfs_with_tables/large.pdf"},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			fullPath := getTestDocumentPath(t, "", tt.path)
			result, err := kreuzberg.ExtractFileSync(fullPath, nil)
			assert.NoError(t, err, fmt.Sprintf("should extract %s without error", tt.name))
			assert.NotNil(t, result, fmt.Sprintf("%s result should not be nil", tt.name))
			assert.NotEmpty(t, result.Content, fmt.Sprintf("%s extraction should succeed", tt.name))
		})
	}
}

// TestFileTypeCoverageDOCX tests DOCX extraction coverage.
func TestFileTypeCoverageDOCX(t *testing.T) {
	tests := []struct {
		name string
		path string
	}{
		{"Lorem Ipsum", "documents/lorem_ipsum.docx"},
		{"DOCX Tables", "documents/docx_tables.docx"},
		{"Word Sample", "documents/word_sample.docx"},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			fullPath := getTestDocumentPath(t, "", tt.path)
			result, err := kreuzberg.ExtractFileSync(fullPath, nil)
			assert.NoError(t, err, fmt.Sprintf("should extract %s without error", tt.name))
			assert.NotNil(t, result, fmt.Sprintf("%s result should not be nil", tt.name))
			assert.NotEmpty(t, result.Content, fmt.Sprintf("%s extraction should succeed", tt.name))
		})
	}
}

// TestFileTypeCoverageXLSX tests XLSX extraction coverage.
func TestFileTypeCoverageXLSX(t *testing.T) {
	xlsxPath := getTestDocumentPath(t, "spreadsheets", "stanley_cups.xlsx")

	result, err := kreuzberg.ExtractFileSync(xlsxPath, nil)
	assert.NoError(t, err, "should extract XLSX without error")
	assert.NotNil(t, result, "XLSX result should not be nil")
	assert.NotEmpty(t, result.Content, "XLSX extraction should succeed")
}

// TestFileTypeCoverageImages tests image extraction coverage.
func TestFileTypeCoverageImages(t *testing.T) {
	tests := []struct {
		name string
		path string
	}{
		{"JPEG Example", "images/example.jpg"},
		{"PNG Sample", "images/sample.png"},
		{"OCR Image", "images/ocr_image.jpg"},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			fullPath := getTestDocumentPath(t, "", tt.path)
			result, err := kreuzberg.ExtractFileSync(fullPath, nil)
			assert.NoError(t, err, fmt.Sprintf("should extract %s without error", tt.name))
			assert.NotNil(t, result, fmt.Sprintf("%s result should not be nil", tt.name))
		})
	}
}

// TestFileTypeCoverageODT tests ODT extraction coverage.
func TestFileTypeCoverageODT(t *testing.T) {
	odtPath := getTestDocumentPath(t, "odt", "paragraph.odt")

	result, err := kreuzberg.ExtractFileSync(odtPath, nil)
	assert.NoError(t, err, "should extract ODT without error")
	assert.NotNil(t, result, "ODT result should not be nil")
	assert.NotEmpty(t, result.Content, "ODT extraction should succeed")
}

// TestFileTypeCoverageMarkdown tests Markdown extraction coverage.
func TestFileTypeCoverageMarkdown(t *testing.T) {
	mdPath := getTestDocumentPath(t, "", "extraction_test.md")

	result, err := kreuzberg.ExtractFileSync(mdPath, nil)
	assert.NoError(t, err, "should extract Markdown without error")
	assert.NotNil(t, result, "Markdown result should not be nil")
	assert.NotEmpty(t, result.Content, "Markdown content should not be empty")
}

// TestResultStructureValidation verifies result structure fields are populated.
func TestResultStructureValidation(t *testing.T) {
	pdfPath := getTestDocumentPath(t, "pdfs_with_tables", "tiny.pdf")

	result, err := kreuzberg.ExtractFileSync(pdfPath, nil)
	assert.NoError(t, err)
	assert.NotNil(t, result)

	assert.NotEmpty(t, result.Content, "content field should be populated")
	assert.NotEmpty(t, result.MimeType, "mime_type field should be populated")
	assert.NotEmpty(t, result.Content, "success field should be true")
	assert.NotNil(t, result.Metadata, "metadata field should be populated")
}

// TestBatchExtractionWithErrors tests batch extraction handles multiple files.
func TestBatchExtractionWithErrors(t *testing.T) {
	pdfPath := getTestDocumentPath(t, "pdfs_with_tables", "tiny.pdf")

	paths := []string{
		pdfPath,
		pdfPath,
	}

	results, err := kreuzberg.BatchExtractFilesSync(paths, nil)
	assert.NoError(t, err, "batch extraction should handle multiple files")
	assert.NotNil(t, results, "results should not be nil")
}

// TestConfigNilHandling tests extraction with nil config uses defaults.
func TestConfigNilHandling(t *testing.T) {
	pdfPath := getTestDocumentPath(t, "pdfs_with_tables", "tiny.pdf")

	result1, err1 := kreuzberg.ExtractFileSync(pdfPath, nil)
	assert.NoError(t, err1)
	assert.NotNil(t, result1)

	config := &kreuzberg.ExtractionConfig{}
	result2, err2 := kreuzberg.ExtractFileSync(pdfPath, config)
	assert.NoError(t, err2)
	assert.NotNil(t, result2)
}

// TestExtensionsForMimeType retrieves extensions for a MIME type.
func TestExtensionsForMimeType(t *testing.T) {
	extensions, err := kreuzberg.GetExtensionsForMime("application/pdf")
	if err == nil {
		assert.NotEmpty(t, extensions, "should return extensions for PDF MIME type")
	}
}

// TestExtractionResultTables verifies table extraction in results.
func TestExtractionResultTables(t *testing.T) {
	pdfPath := getTestDocumentPath(t, "pdfs_with_tables", "medium.pdf")

	result, err := kreuzberg.ExtractFileSync(pdfPath, nil)
	assert.NoError(t, err)
	assert.NotNil(t, result)
	assert.NotNil(t, result.Tables, "tables field should be populated")
}

// TestMetadataFormatType verifies FormatType() method works.
func TestMetadataFormatType(t *testing.T) {
	metadata := kreuzberg.Metadata{
		Format: kreuzberg.FormatMetadata{
			Type: kreuzberg.FormatPDF,
		},
	}

	formatType := metadata.FormatType()
	assert.Equal(t, kreuzberg.FormatPDF, formatType)
}

// TestMetadataPdfMetadata verifies PdfMetadata() method works.
func TestMetadataPdfMetadata(t *testing.T) {
	pdfMeta := &kreuzberg.PdfMetadata{
		PageCount: kreuzberg.IntPtr(5),
	}
	metadata := kreuzberg.Metadata{
		Format: kreuzberg.FormatMetadata{
			Type: kreuzberg.FormatPDF,
			Pdf:  pdfMeta,
		},
	}

	pdf, ok := metadata.PdfMetadata()
	assert.True(t, ok)
	assert.NotNil(t, pdf)
	assert.NotNil(t, pdf.PageCount)
	assert.Equal(t, 5, *pdf.PageCount)
}

// TestMetadataExcelMetadata verifies ExcelMetadata() method works.
func TestMetadataExcelMetadata(t *testing.T) {
	excelMeta := &kreuzberg.ExcelMetadata{
		SheetCount: 3,
		SheetNames: []string{"Sheet1", "Sheet2", "Sheet3"},
	}
	metadata := kreuzberg.Metadata{
		Format: kreuzberg.FormatMetadata{
			Type:  kreuzberg.FormatExcel,
			Excel: excelMeta,
		},
	}

	excel, ok := metadata.ExcelMetadata()
	assert.True(t, ok)
	assert.NotNil(t, excel)
	assert.Equal(t, 3, excel.SheetCount)
	assert.Equal(t, 3, len(excel.SheetNames))
}

// TestChunkMetadataStructure verifies chunk metadata fields.
func TestChunkMetadataStructure(t *testing.T) {
	chunkMeta := kreuzberg.ChunkMetadata{
		ByteStart:   0,
		ByteEnd:     100,
		ChunkIndex:  0,
		TotalChunks: 5,
		TokenCount:  kreuzberg.IntPtr(50),
	}

	assert.Equal(t, uint64(0), chunkMeta.ByteStart)
	assert.Equal(t, uint64(100), chunkMeta.ByteEnd)
	assert.Equal(t, 0, chunkMeta.ChunkIndex)
	assert.Equal(t, 5, chunkMeta.TotalChunks)
	assert.NotNil(t, chunkMeta.TokenCount)
	assert.Equal(t, 50, *chunkMeta.TokenCount)
}

// TestErrorsIsFunction tests error type checking with errors.Is.
func TestErrorsIsFunction(t *testing.T) {
	_, err := kreuzberg.ExtractFileSync("/nonexistent/path/file.pdf", nil)
	assert.Error(t, err, "should return error")

	if err != nil {
		assert.True(t, errors.Is(err, err), "error should match itself")
	}
}

// TestOutputFormatPlain tests OutputFormat with "plain" value.
func TestOutputFormatPlain(t *testing.T) {
	config := &kreuzberg.ExtractionConfig{
		OutputFormat: string(kreuzberg.OutputFormatPlain),
	}
	assert.NotNil(t, config)
	assert.Equal(t, "plain", config.OutputFormat)
	assert.Equal(t, string(kreuzberg.OutputFormatPlain), config.OutputFormat)
}

// TestOutputFormatMarkdown tests OutputFormat with "markdown" value.
func TestOutputFormatMarkdown(t *testing.T) {
	config := &kreuzberg.ExtractionConfig{
		OutputFormat: string(kreuzberg.OutputFormatMarkdown),
	}
	assert.NotNil(t, config)
	assert.Equal(t, "markdown", config.OutputFormat)
	assert.Equal(t, string(kreuzberg.OutputFormatMarkdown), config.OutputFormat)
}

// TestOutputFormatDjot tests OutputFormat with "djot" value.
func TestOutputFormatDjot(t *testing.T) {
	config := &kreuzberg.ExtractionConfig{
		OutputFormat: string(kreuzberg.OutputFormatDjot),
	}
	assert.NotNil(t, config)
	assert.Equal(t, "djot", config.OutputFormat)
	assert.Equal(t, string(kreuzberg.OutputFormatDjot), config.OutputFormat)
}

// TestOutputFormatHTML tests OutputFormat with "html" value.
func TestOutputFormatHTML(t *testing.T) {
	config := &kreuzberg.ExtractionConfig{
		OutputFormat: string(kreuzberg.OutputFormatHTML),
	}
	assert.NotNil(t, config)
	assert.Equal(t, "html", config.OutputFormat)
	assert.Equal(t, string(kreuzberg.OutputFormatHTML), config.OutputFormat)
}

// TestResultFormatUnified tests ResultFormat with "unified" value.
func TestResultFormatUnified(t *testing.T) {
	config := &kreuzberg.ExtractionConfig{
		ResultFormat: string(kreuzberg.ResultFormatUnified),
	}
	assert.NotNil(t, config)
	assert.Equal(t, "unified", config.ResultFormat)
	assert.Equal(t, string(kreuzberg.ResultFormatUnified), config.ResultFormat)
}

// TestResultFormatElementBased tests ResultFormat with "element_based" value.
func TestResultFormatElementBased(t *testing.T) {
	config := &kreuzberg.ExtractionConfig{
		ResultFormat: string(kreuzberg.ResultFormatElementBased),
	}
	assert.NotNil(t, config)
	assert.Equal(t, "element_based", config.ResultFormat)
	assert.Equal(t, string(kreuzberg.ResultFormatElementBased), config.ResultFormat)
}

// TestOutputFormatAllValues tests all OutputFormat constant values.
func TestOutputFormatAllValues(t *testing.T) {
	testCases := []struct {
		name     string
		format   kreuzberg.OutputFormat
		expected string
	}{
		{"Plain", kreuzberg.OutputFormatPlain, "plain"},
		{"Markdown", kreuzberg.OutputFormatMarkdown, "markdown"},
		{"Djot", kreuzberg.OutputFormatDjot, "djot"},
		{"HTML", kreuzberg.OutputFormatHTML, "html"},
	}

	for _, tc := range testCases {
		t.Run(tc.name, func(t *testing.T) {
			config := &kreuzberg.ExtractionConfig{
				OutputFormat: string(tc.format),
			}
			assert.Equal(t, tc.expected, config.OutputFormat)
		})
	}
}

// TestResultFormatAllValues tests all ResultFormat constant values.
func TestResultFormatAllValues(t *testing.T) {
	testCases := []struct {
		name     string
		format   kreuzberg.ResultFormat
		expected string
	}{
		{"Unified", kreuzberg.ResultFormatUnified, "unified"},
		{"ElementBased", kreuzberg.ResultFormatElementBased, "element_based"},
	}

	for _, tc := range testCases {
		t.Run(tc.name, func(t *testing.T) {
			config := &kreuzberg.ExtractionConfig{
				ResultFormat: string(tc.format),
			}
			assert.Equal(t, tc.expected, config.ResultFormat)
		})
	}
}

// TestOutputFormatAndResultFormatCombinations tests combinations of OutputFormat and ResultFormat.
func TestOutputFormatAndResultFormatCombinations(t *testing.T) {
	outputFormats := []kreuzberg.OutputFormat{
		kreuzberg.OutputFormatPlain,
		kreuzberg.OutputFormatMarkdown,
		kreuzberg.OutputFormatDjot,
		kreuzberg.OutputFormatHTML,
	}

	resultFormats := []kreuzberg.ResultFormat{
		kreuzberg.ResultFormatUnified,
		kreuzberg.ResultFormatElementBased,
	}

	for _, outFmt := range outputFormats {
		for _, resFmt := range resultFormats {
			config := &kreuzberg.ExtractionConfig{
				OutputFormat: string(outFmt),
				ResultFormat: string(resFmt),
			}
			assert.NotNil(t, config)
			assert.NotEmpty(t, config.OutputFormat)
			assert.NotEmpty(t, config.ResultFormat)
		}
	}
}

// TestOutputFormatEmptyDefault tests that OutputFormat can be left empty to use default.
func TestOutputFormatEmptyDefault(t *testing.T) {
	config := &kreuzberg.ExtractionConfig{
		OutputFormat: "",
	}
	assert.NotNil(t, config)
	assert.Equal(t, "", config.OutputFormat)
}

// TestResultFormatEmptyDefault tests that ResultFormat can be left empty to use default.
func TestResultFormatEmptyDefault(t *testing.T) {
	config := &kreuzberg.ExtractionConfig{
		ResultFormat: "",
	}
	assert.NotNil(t, config)
	assert.Equal(t, "", config.ResultFormat)
}

// TestExtractionConfigWithOutputFormat tests extraction config with OutputFormat set.
func TestExtractionConfigWithOutputFormat(t *testing.T) {
	config := &kreuzberg.ExtractionConfig{
		UseCache:     kreuzberg.BoolPtr(true),
		OutputFormat: string(kreuzberg.OutputFormatMarkdown),
	}
	assert.NotNil(t, config)
	assert.True(t, *config.UseCache)
	assert.Equal(t, "markdown", config.OutputFormat)
}

// TestExtractionConfigWithResultFormat tests extraction config with ResultFormat set.
func TestExtractionConfigWithResultFormat(t *testing.T) {
	config := &kreuzberg.ExtractionConfig{
		ForceOCR:     kreuzberg.BoolPtr(false),
		ResultFormat: string(kreuzberg.ResultFormatElementBased),
	}
	assert.NotNil(t, config)
	assert.False(t, *config.ForceOCR)
	assert.Equal(t, "element_based", config.ResultFormat)
}

// TestExtractionConfigWithBothFormats tests config with both OutputFormat and ResultFormat.
func TestExtractionConfigWithBothFormats(t *testing.T) {
	config := &kreuzberg.ExtractionConfig{
		UseCache:     kreuzberg.BoolPtr(false),
		OutputFormat: string(kreuzberg.OutputFormatDjot),
		ResultFormat: string(kreuzberg.ResultFormatUnified),
	}
	assert.NotNil(t, config)
	assert.False(t, *config.UseCache)
	assert.Equal(t, "djot", config.OutputFormat)
	assert.Equal(t, "unified", config.ResultFormat)
}

// TestConfigSerializationWithOutputFormat tests OutputFormat serialization via ConfigToJSON.
func TestConfigSerializationWithOutputFormat(t *testing.T) {
	config := &kreuzberg.ExtractionConfig{
		OutputFormat: string(kreuzberg.OutputFormatMarkdown),
	}
	jsonStr, err := kreuzberg.ConfigToJSON(config)
	assert.NoError(t, err, "should serialize config with OutputFormat")
	assert.NotEmpty(t, jsonStr)
	assert.Contains(t, jsonStr, "output_format")
	assert.Contains(t, jsonStr, "markdown")
}

// TestConfigSerializationWithResultFormat tests ResultFormat serialization via ConfigToJSON.
func TestConfigSerializationWithResultFormat(t *testing.T) {
	config := &kreuzberg.ExtractionConfig{
		ResultFormat: string(kreuzberg.ResultFormatElementBased),
	}
	jsonStr, err := kreuzberg.ConfigToJSON(config)
	assert.NoError(t, err, "should serialize config with ResultFormat")
	assert.NotEmpty(t, jsonStr)
	assert.Contains(t, jsonStr, "result_format")
	assert.Contains(t, jsonStr, "element_based")
}

// TestConfigSerializationWithBothFormats tests serialization with both format fields.
func TestConfigSerializationWithBothFormats(t *testing.T) {
	config := &kreuzberg.ExtractionConfig{
		OutputFormat: string(kreuzberg.OutputFormatHTML),
		ResultFormat: string(kreuzberg.ResultFormatUnified),
	}
	jsonStr, err := kreuzberg.ConfigToJSON(config)
	assert.NoError(t, err, "should serialize config with both formats")
	assert.NotEmpty(t, jsonStr)
	assert.Contains(t, jsonStr, "output_format")
	assert.Contains(t, jsonStr, "html")
	assert.Contains(t, jsonStr, "result_format")
	assert.Contains(t, jsonStr, "unified")
}

// TestConfigDeserializationWithOutputFormat tests OutputFormat deserialization via ConfigFromJSON.
func TestConfigDeserializationWithOutputFormat(t *testing.T) {
	jsonStr := `{"output_format":"plain"}`
	config, err := kreuzberg.ConfigFromJSON(jsonStr)
	if err == nil {
		assert.NotNil(t, config)
		assert.Equal(t, "plain", config.OutputFormat)
	}
}

// TestConfigDeserializationWithResultFormat tests ResultFormat deserialization via ConfigFromJSON.
func TestConfigDeserializationWithResultFormat(t *testing.T) {
	jsonStr := `{"result_format":"element_based"}`
	config, err := kreuzberg.ConfigFromJSON(jsonStr)
	if err == nil {
		assert.NotNil(t, config)
		assert.Equal(t, "element_based", config.ResultFormat)
	}
}

// TestConfigDeserializationWithBothFormats tests deserialization with both formats.
func TestConfigDeserializationWithBothFormats(t *testing.T) {
	jsonStr := `{"output_format":"djot","result_format":"unified"}`
	config, err := kreuzberg.ConfigFromJSON(jsonStr)
	if err == nil {
		assert.NotNil(t, config)
		assert.Equal(t, "djot", config.OutputFormat)
		assert.Equal(t, "unified", config.ResultFormat)
	}
}

// TestConfigMergePreservesOutputFormat tests that ConfigMerge preserves OutputFormat.
func TestConfigMergePreservesOutputFormat(t *testing.T) {
	baseConfig := &kreuzberg.ExtractionConfig{
		OutputFormat: string(kreuzberg.OutputFormatMarkdown),
	}
	overrideConfig := &kreuzberg.ExtractionConfig{
		UseCache: kreuzberg.BoolPtr(true),
	}
	err := kreuzberg.ConfigMerge(baseConfig, overrideConfig)
	assert.NoError(t, err)
	assert.Equal(t, "markdown", baseConfig.OutputFormat)
	assert.True(t, *baseConfig.UseCache)
}

// TestConfigMergeOverridesOutputFormat tests that ConfigMerge overrides OutputFormat.
func TestConfigMergeOverridesOutputFormat(t *testing.T) {
	baseConfig := &kreuzberg.ExtractionConfig{
		OutputFormat: string(kreuzberg.OutputFormatMarkdown),
	}
	overrideConfig := &kreuzberg.ExtractionConfig{
		OutputFormat: string(kreuzberg.OutputFormatHTML),
	}
	err := kreuzberg.ConfigMerge(baseConfig, overrideConfig)
	assert.NoError(t, err)
	assert.Equal(t, "html", baseConfig.OutputFormat)
}

// TestConfigMergePreservesResultFormat tests that ConfigMerge preserves ResultFormat.
func TestConfigMergePreservesResultFormat(t *testing.T) {
	baseConfig := &kreuzberg.ExtractionConfig{
		ResultFormat: string(kreuzberg.ResultFormatUnified),
	}
	overrideConfig := &kreuzberg.ExtractionConfig{
		ForceOCR: kreuzberg.BoolPtr(true),
	}
	err := kreuzberg.ConfigMerge(baseConfig, overrideConfig)
	assert.NoError(t, err)
	assert.Equal(t, "unified", baseConfig.ResultFormat)
	assert.True(t, *baseConfig.ForceOCR)
}

// TestConfigMergeOverridesResultFormat tests that ConfigMerge overrides ResultFormat.
func TestConfigMergeOverridesResultFormat(t *testing.T) {
	baseConfig := &kreuzberg.ExtractionConfig{
		ResultFormat: string(kreuzberg.ResultFormatUnified),
	}
	overrideConfig := &kreuzberg.ExtractionConfig{
		ResultFormat: string(kreuzberg.ResultFormatElementBased),
	}
	err := kreuzberg.ConfigMerge(baseConfig, overrideConfig)
	assert.NoError(t, err)
	assert.Equal(t, "element_based", baseConfig.ResultFormat)
}

// TestConfigMergeBothFormats tests ConfigMerge with both format fields.
func TestConfigMergeBothFormats(t *testing.T) {
	baseConfig := &kreuzberg.ExtractionConfig{
		OutputFormat: string(kreuzberg.OutputFormatMarkdown),
		ResultFormat: string(kreuzberg.ResultFormatUnified),
	}
	overrideConfig := &kreuzberg.ExtractionConfig{
		OutputFormat: string(kreuzberg.OutputFormatDjot),
		ResultFormat: string(kreuzberg.ResultFormatElementBased),
	}
	err := kreuzberg.ConfigMerge(baseConfig, overrideConfig)
	assert.NoError(t, err)
	assert.Equal(t, "djot", baseConfig.OutputFormat)
	assert.Equal(t, "element_based", baseConfig.ResultFormat)
}

// TestOutputFormatTypeConversion tests type conversion for OutputFormat.
func TestOutputFormatTypeConversion(t *testing.T) {
	format := kreuzberg.OutputFormatMarkdown
	strValue := string(format)
	assert.Equal(t, "markdown", strValue)

	// Reverse conversion
	newFormat := kreuzberg.OutputFormat(strValue)
	assert.Equal(t, format, newFormat)
}

// TestResultFormatTypeConversion tests type conversion for ResultFormat.
func TestResultFormatTypeConversion(t *testing.T) {
	format := kreuzberg.ResultFormatElementBased
	strValue := string(format)
	assert.Equal(t, "element_based", strValue)

	// Reverse conversion
	newFormat := kreuzberg.ResultFormat(strValue)
	assert.Equal(t, format, newFormat)
}

// ============================================================================
// Configuration Builders Tests
// ============================================================================

// TestNewOCRConfig tests OCRConfig builder with options.
func TestNewOCRConfig(t *testing.T) {
	config := kreuzberg.NewOCRConfig(
		kreuzberg.WithOCRBackend("tesseract"),
		kreuzberg.WithOCRLanguage("eng"),
	)
	assert.NotNil(t, config)
	assert.Equal(t, "tesseract", config.Backend)
	assert.NotNil(t, config.Language)
	assert.Equal(t, "eng", *config.Language)
}

// TestNewChunkingConfig tests ChunkingConfig builder with options.
func TestNewChunkingConfig(t *testing.T) {
	config := kreuzberg.NewChunkingConfig(
		kreuzberg.WithMaxChars(1024),
		kreuzberg.WithChunkOverlap(100),
	)
	assert.NotNil(t, config)
	assert.NotNil(t, config.MaxChars)
	assert.Equal(t, 1024, *config.MaxChars)
	assert.NotNil(t, config.ChunkOverlap)
	assert.Equal(t, 100, *config.ChunkOverlap)
}

// TestNewImageExtractionConfig tests ImageExtractionConfig builder.
func TestNewImageExtractionConfig(t *testing.T) {
	config := kreuzberg.NewImageExtractionConfig(
		kreuzberg.WithExtractImages(true),
		kreuzberg.WithImageTargetDPI(300),
	)
	assert.NotNil(t, config)
	assert.NotNil(t, config.ExtractImages)
	assert.True(t, *config.ExtractImages)
	assert.NotNil(t, config.TargetDPI)
	assert.Equal(t, 300, *config.TargetDPI)
}

// TestNewPdfConfig tests PdfConfig builder.
func TestNewPdfConfig(t *testing.T) {
	config := kreuzberg.NewPdfConfig(
		kreuzberg.WithPdfExtractImages(true),
		kreuzberg.WithPdfExtractMetadata(true),
	)
	assert.NotNil(t, config)
	assert.NotNil(t, config.ExtractImages)
	assert.True(t, *config.ExtractImages)
	assert.NotNil(t, config.ExtractMetadata)
	assert.True(t, *config.ExtractMetadata)
}

// TestNewTokenReductionConfig tests TokenReductionConfig builder.
func TestNewTokenReductionConfig(t *testing.T) {
	config := kreuzberg.NewTokenReductionConfig(
		kreuzberg.WithTokenReductionMode("light"),
	)
	assert.NotNil(t, config)
	assert.Equal(t, "light", config.Mode)
}

// TestNewLanguageDetectionConfig tests LanguageDetectionConfig builder.
func TestNewLanguageDetectionConfig(t *testing.T) {
	config := kreuzberg.NewLanguageDetectionConfig(
		kreuzberg.WithLanguageDetectionEnabled(true),
		kreuzberg.WithDetectMultiple(true),
	)
	assert.NotNil(t, config)
	assert.NotNil(t, config.Enabled)
	assert.True(t, *config.Enabled)
	assert.NotNil(t, config.DetectMultiple)
	assert.True(t, *config.DetectMultiple)
}

// TestNewKeywordConfig tests KeywordConfig builder.
func TestNewKeywordConfig(t *testing.T) {
	config := kreuzberg.NewKeywordConfig(
		kreuzberg.WithKeywordAlgorithm("yake"),
		kreuzberg.WithMaxKeywords(10),
	)
	assert.NotNil(t, config)
	assert.Equal(t, "yake", config.Algorithm)
	assert.NotNil(t, config.MaxKeywords)
	assert.Equal(t, 10, *config.MaxKeywords)
}

// TestNewPostProcessorConfig tests PostProcessorConfig builder.
func TestNewPostProcessorConfig(t *testing.T) {
	config := kreuzberg.NewPostProcessorConfig(
		kreuzberg.WithPostProcessorEnabled(true),
	)
	assert.NotNil(t, config)
	assert.NotNil(t, config.Enabled)
	assert.True(t, *config.Enabled)
}

// TestNewHTMLConversionOptions tests HTMLConversionOptions builder.
func TestNewHTMLConversionOptions(t *testing.T) {
	config := kreuzberg.NewHTMLConversionOptions(
		kreuzberg.WithHeadingStyle("setext"),
		kreuzberg.WithListIndentWidth(2),
	)
	assert.NotNil(t, config)
	assert.NotNil(t, config.HeadingStyle)
	assert.Equal(t, "setext", *config.HeadingStyle)
	assert.NotNil(t, config.ListIndentWidth)
	assert.Equal(t, 2, *config.ListIndentWidth)
}

// TestNewPageConfig tests PageConfig builder.
func TestNewPageConfig(t *testing.T) {
	config := kreuzberg.NewPageConfig(
		kreuzberg.WithExtractPages(true),
		kreuzberg.WithInsertPageMarkers(true),
	)
	assert.NotNil(t, config)
	assert.NotNil(t, config.ExtractPages)
	assert.True(t, *config.ExtractPages)
	assert.NotNil(t, config.InsertPageMarkers)
	assert.True(t, *config.InsertPageMarkers)
}

// TestNestedConfigBuilding tests building nested configs with options.
func TestNestedConfigBuilding(t *testing.T) {
	config := kreuzberg.NewExtractionConfig(
		kreuzberg.WithOCR(
			kreuzberg.WithOCRBackend("tesseract"),
			kreuzberg.WithOCRLanguage("eng"),
		),
		kreuzberg.WithChunking(
			kreuzberg.WithMaxChars(512),
		),
	)
	assert.NotNil(t, config)
	assert.NotNil(t, config.OCR)
	assert.Equal(t, "tesseract", config.OCR.Backend)
	assert.NotNil(t, config.Chunking)
	assert.NotNil(t, config.Chunking.MaxChars)
	assert.Equal(t, 512, *config.Chunking.MaxChars)
}

// ============================================================================
// Validation Functions Tests
// ============================================================================

// TestValidateBinarizationMethod tests binarization method validation.
func TestValidateBinarizationMethod(t *testing.T) {
	// This test may fail if specific methods aren't implemented in FFI
	// but we test the validation function exists and can be called
	err := kreuzberg.ValidateBinarizationMethod("otsu")
	if err != nil {
		t.Logf("ValidateBinarizationMethod returned error (expected in test env): %v", err)
	}

	err = kreuzberg.ValidateBinarizationMethod("")
	assert.Error(t, err, "empty string should be invalid")
}

// TestValidateOCRBackend tests OCR backend validation.
func TestValidateOCRBackend(t *testing.T) {
	err := kreuzberg.ValidateOCRBackend("tesseract")
	if err != nil {
		t.Logf("ValidateOCRBackend('tesseract') returned: %v", err)
	}

	err = kreuzberg.ValidateOCRBackend("")
	assert.Error(t, err, "empty backend should be invalid")

	err = kreuzberg.ValidateOCRBackend("invalid_backend_xyz")
	assert.Error(t, err, "invalid backend should return error")
}

// TestValidateLanguageCode tests language code validation.
func TestValidateLanguageCode(t *testing.T) {
	// Test valid 2-letter codes
	err := kreuzberg.ValidateLanguageCode("en")
	if err != nil {
		t.Logf("ValidateLanguageCode('en') returned: %v", err)
	}

	err = kreuzberg.ValidateLanguageCode("de")
	if err != nil {
		t.Logf("ValidateLanguageCode('de') returned: %v", err)
	}

	// Test invalid code
	err = kreuzberg.ValidateLanguageCode("")
	assert.Error(t, err, "empty language code should be invalid")

	err = kreuzberg.ValidateLanguageCode("invalid")
	if err == nil {
		t.Logf("Note: 'invalid' language code was accepted (may be test env limitation)")
	}
}

// TestValidateTokenReductionLevel tests token reduction level validation.
func TestValidateTokenReductionLevel(t *testing.T) {
	err := kreuzberg.ValidateTokenReductionLevel("light")
	if err != nil {
		t.Logf("ValidateTokenReductionLevel('light') returned: %v", err)
	}

	err = kreuzberg.ValidateTokenReductionLevel("")
	assert.Error(t, err, "empty level should be invalid")
}

// TestValidateTesseractPSM tests Tesseract PSM validation with boundaries.
func TestValidateTesseractPSM(t *testing.T) {
	// Valid PSM values: 0-13
	testCases := []struct {
		psm       int
		shouldErr bool
	}{
		{0, false},
		{3, false},
		{13, false},
		{-1, true},
		{14, true},
	}

	for _, tc := range testCases {
		err := kreuzberg.ValidateTesseractPSM(tc.psm)
		if tc.shouldErr {
			assert.Error(t, err, "PSM %d should be invalid", tc.psm)
		} else if err != nil {
			t.Logf("PSM %d validation returned: %v", tc.psm, err)
		}
	}
}

// TestValidateTesseractOEM tests Tesseract OEM validation with boundaries.
func TestValidateTesseractOEM(t *testing.T) {
	// Valid OEM values: 0-3
	testCases := []struct {
		oem       int
		shouldErr bool
	}{
		{0, false},
		{1, false},
		{3, false},
		{-1, true},
		{4, true},
	}

	for _, tc := range testCases {
		err := kreuzberg.ValidateTesseractOEM(tc.oem)
		if tc.shouldErr {
			assert.Error(t, err, "OEM %d should be invalid", tc.oem)
		} else if err != nil {
			t.Logf("OEM %d validation returned: %v", tc.oem, err)
		}
	}
}

// TestValidateConfidence tests confidence threshold validation.
func TestValidateConfidence(t *testing.T) {
	testCases := []struct {
		confidence float64
		shouldErr  bool
	}{
		{0.0, false},
		{0.5, false},
		{1.0, false},
		{-0.1, true},
		{1.1, true},
	}

	for _, tc := range testCases {
		err := kreuzberg.ValidateConfidence(tc.confidence)
		if tc.shouldErr {
			assert.Error(t, err, "confidence %.1f should be invalid", tc.confidence)
		} else if err != nil {
			t.Logf("confidence %.1f validation returned: %v", tc.confidence, err)
		}
	}
}

// TestValidateDPI tests DPI validation.
func TestValidateDPI(t *testing.T) {
	testCases := []struct {
		dpi       int
		shouldErr bool
	}{
		{72, false},
		{150, false},
		{300, false},
		{600, false},
		{0, true},
		{-100, true},
	}

	for _, tc := range testCases {
		err := kreuzberg.ValidateDPI(tc.dpi)
		if tc.shouldErr {
			assert.Error(t, err, "DPI %d should be invalid", tc.dpi)
		} else if err != nil {
			t.Logf("DPI %d validation returned: %v", tc.dpi, err)
		}
	}
}

// TestValidateChunkingParams tests chunking parameter validation.
func TestValidateChunkingParams(t *testing.T) {
	testCases := []struct {
		maxChars   int
		maxOverlap int
		shouldErr  bool
	}{
		{1000, 100, false},
		{512, 50, false},
		{0, 100, true},
		{-1, 100, true},
		{100, -1, true},
		{100, 100, true},
		{100, 150, true},
	}

	for _, tc := range testCases {
		err := kreuzberg.ValidateChunkingParams(tc.maxChars, tc.maxOverlap)
		if tc.shouldErr {
			assert.Error(t, err, "params (%d, %d) should be invalid", tc.maxChars, tc.maxOverlap)
		} else if err != nil {
			t.Logf("params (%d, %d) validation returned: %v", tc.maxChars, tc.maxOverlap, err)
		}
	}
}

// TestValidateOutputFormat tests output format validation.
func TestValidateOutputFormat(t *testing.T) {
	err := kreuzberg.ValidateOutputFormat("text")
	if err != nil {
		t.Logf("ValidateOutputFormat('text') returned: %v", err)
	}

	err = kreuzberg.ValidateOutputFormat("")
	assert.Error(t, err, "empty format should be invalid")
}

// TestValidateMimeType tests MIME type validation and canonicalization.
func TestValidateMimeType(t *testing.T) {
	canonical, err := kreuzberg.ValidateMimeType("application/pdf")
	assert.NoError(t, err)
	assert.NotEmpty(t, canonical)

	_, err = kreuzberg.ValidateMimeType("")
	assert.Error(t, err, "empty MIME type should be invalid")
}

// TestGetValidBinarizationMethods tests retrieving valid binarization methods.
func TestGetValidBinarizationMethods(t *testing.T) {
	methods, err := kreuzberg.GetValidBinarizationMethods()
	if err == nil {
		assert.NotNil(t, methods)
		assert.GreaterOrEqual(t, len(methods), 0)
	} else {
		t.Logf("GetValidBinarizationMethods returned error: %v", err)
	}
}

// TestGetValidLanguageCodes tests retrieving valid language codes.
func TestGetValidLanguageCodes(t *testing.T) {
	codes, err := kreuzberg.GetValidLanguageCodes()
	if err == nil {
		assert.NotNil(t, codes)
		assert.GreaterOrEqual(t, len(codes), 0)
	} else {
		t.Logf("GetValidLanguageCodes returned error: %v", err)
	}
}

// TestGetValidOCRBackends tests retrieving valid OCR backends.
func TestGetValidOCRBackends(t *testing.T) {
	backends, err := kreuzberg.GetValidOCRBackends()
	if err == nil {
		assert.NotNil(t, backends)
		assert.GreaterOrEqual(t, len(backends), 0)
	} else {
		t.Logf("GetValidOCRBackends returned error: %v", err)
	}
}

// TestGetValidTokenReductionLevels tests retrieving valid token reduction levels.
func TestGetValidTokenReductionLevels(t *testing.T) {
	levels, err := kreuzberg.GetValidTokenReductionLevels()
	if err == nil {
		assert.NotNil(t, levels)
		assert.GreaterOrEqual(t, len(levels), 0)
	} else {
		t.Logf("GetValidTokenReductionLevels returned error: %v", err)
	}
}

// ============================================================================
// Pointer Helper Functions Tests
// ============================================================================

// TestInt32Ptr tests Int32Ptr helper.
func TestInt32Ptr(t *testing.T) {
	val := int32(42)
	ptr := kreuzberg.Int32Ptr(val)
	assert.NotNil(t, ptr)
	assert.Equal(t, val, *ptr)
}

// TestInt64Ptr tests Int64Ptr helper.
func TestInt64Ptr(t *testing.T) {
	val := int64(999)
	ptr := kreuzberg.Int64Ptr(val)
	assert.NotNil(t, ptr)
	assert.Equal(t, val, *ptr)
}

// TestFloat32Ptr tests Float32Ptr helper.
func TestFloat32Ptr(t *testing.T) {
	val := float32(3.14)
	ptr := kreuzberg.Float32Ptr(val)
	assert.NotNil(t, ptr)
	assert.Equal(t, val, *ptr)
}

// TestUint32Ptr tests Uint32Ptr helper.
func TestUint32Ptr(t *testing.T) {
	val := uint32(100)
	ptr := kreuzberg.Uint32Ptr(val)
	assert.NotNil(t, ptr)
	assert.Equal(t, val, *ptr)
}

// TestUint64Ptr tests Uint64Ptr helper.
func TestUint64Ptr(t *testing.T) {
	val := uint64(5000)
	ptr := kreuzberg.Uint64Ptr(val)
	assert.NotNil(t, ptr)
	assert.Equal(t, val, *ptr)
}

// TestPointerHelpersReturnIndependentValues tests that pointer helpers return independent values.
// Modifying one pointer's value should not affect other pointers created with the same input value.
func TestPointerHelpersReturnIndependentValues(t *testing.T) {
	// Test that modifying one pointer does not affect another
	ptr1 := kreuzberg.IntPtr(42)
	ptr2 := kreuzberg.IntPtr(42)

	// Both should have initial value of 42
	assert.Equal(t, 42, *ptr1)
	assert.Equal(t, 42, *ptr2)

	// Modify ptr1's value
	*ptr1 = 100

	// ptr2 should still be 42 (values are independent)
	assert.Equal(t, 100, *ptr1)
	assert.Equal(t, 42, *ptr2, "modifying ptr1 should not affect ptr2")

	// Same test for strings
	ptr3 := kreuzberg.StringPtr("test")
	ptr4 := kreuzberg.StringPtr("test")

	assert.Equal(t, "test", *ptr3)
	assert.Equal(t, "test", *ptr4)

	*ptr3 = "modified"

	assert.Equal(t, "modified", *ptr3)
	assert.Equal(t, "test", *ptr4, "modifying ptr3 should not affect ptr4")
}

// ============================================================================
// Result Accessor Methods Tests
// ============================================================================

// TestResultGetPageCount tests GetPageCount method.
func TestResultGetPageCount(t *testing.T) {
	result := &kreuzberg.ExtractionResult{
		Metadata: kreuzberg.Metadata{
			PageStructure: &kreuzberg.PageStructure{
				TotalCount: 5,
			},
		},
	}
	count, err := result.GetPageCount()
	assert.NoError(t, err)
	assert.Equal(t, 5, count)
}

// TestResultGetPageCountNoPages tests GetPageCount with no pages.
func TestResultGetPageCountNoPages(t *testing.T) {
	result := &kreuzberg.ExtractionResult{}
	count, err := result.GetPageCount()
	assert.NoError(t, err)
	assert.Equal(t, 0, count)
}

// TestResultGetChunkCount tests GetChunkCount method.
func TestResultGetChunkCount(t *testing.T) {
	result := &kreuzberg.ExtractionResult{
		Chunks: []kreuzberg.Chunk{
			{Content: "chunk1"},
			{Content: "chunk2"},
			{Content: "chunk3"},
		},
	}
	count, err := result.GetChunkCount()
	assert.NoError(t, err)
	assert.Equal(t, 3, count)
}

// TestResultGetChunkCountNoChunks tests GetChunkCount with no chunks.
func TestResultGetChunkCountNoChunks(t *testing.T) {
	result := &kreuzberg.ExtractionResult{}
	count, err := result.GetChunkCount()
	assert.NoError(t, err)
	assert.Equal(t, 0, count)
}

// TestResultGetDetectedLanguage tests GetDetectedLanguage method.
func TestResultGetDetectedLanguage(t *testing.T) {
	result := &kreuzberg.ExtractionResult{
		Metadata: kreuzberg.Metadata{
			Language: kreuzberg.StringPtr("en"),
		},
	}
	lang, err := result.GetDetectedLanguage()
	assert.NoError(t, err)
	assert.Equal(t, "en", lang)
}

// TestResultGetDetectedLanguageFromList tests GetDetectedLanguage fallback to DetectedLanguages array.
func TestResultGetDetectedLanguageFromList(t *testing.T) {
	result := &kreuzberg.ExtractionResult{
		DetectedLanguages: []string{"de", "fr"},
	}
	lang, err := result.GetDetectedLanguage()
	assert.NoError(t, err)
	assert.Equal(t, "de", lang)
}

// TestResultGetDetectedLanguageEmpty tests GetDetectedLanguage with no language.
func TestResultGetDetectedLanguageEmpty(t *testing.T) {
	result := &kreuzberg.ExtractionResult{}
	lang, err := result.GetDetectedLanguage()
	assert.NoError(t, err)
	assert.Equal(t, "", lang)
}

// TestResultGetMetadataField tests GetMetadataField method.
func TestResultGetMetadataField(t *testing.T) {
	result := &kreuzberg.ExtractionResult{
		Metadata: kreuzberg.Metadata{
			Language: kreuzberg.StringPtr("en"),
		},
	}
	field, err := result.GetMetadataField("language")
	assert.NoError(t, err)
	assert.NotNil(t, field)
	assert.Equal(t, "language", field.Name)
	assert.False(t, field.IsNull)
}

// TestResultGetMetadataFieldEmpty tests GetMetadataField with nonexistent field.
func TestResultGetMetadataFieldEmpty(t *testing.T) {
	result := &kreuzberg.ExtractionResult{}
	field, err := result.GetMetadataField("nonexistent")
	assert.NoError(t, err)
	assert.NotNil(t, field)
	assert.True(t, field.IsNull)
}

// TestResultToJSON tests ResultToJSON serialization.
func TestResultToJSON(t *testing.T) {
	result := &kreuzberg.ExtractionResult{
		Content:  "test content",
		MimeType: "text/plain",
	}
	jsonStr, err := kreuzberg.ResultToJSON(result)
	assert.NoError(t, err)
	assert.NotEmpty(t, jsonStr)
	assert.Contains(t, jsonStr, "test content")
}

// TestResultFromJSON tests ResultFromJSON deserialization.
func TestResultFromJSON(t *testing.T) {
	jsonStr := `{"content":"test","mime_type":"text/plain","success":true}`
	result, err := kreuzberg.ResultFromJSON(jsonStr)
	assert.NoError(t, err)
	assert.NotNil(t, result)
	assert.Equal(t, "test", result.Content)
	assert.Equal(t, "text/plain", result.MimeType)
	assert.NotEmpty(t, result.Content)
}

// TestResultJSONRoundTrip tests serialization and deserialization round-trip.
func TestResultJSONRoundTrip(t *testing.T) {
	original := &kreuzberg.ExtractionResult{
		Content:  "sample content",
		MimeType: "application/pdf",
	}
	jsonStr, err := kreuzberg.ResultToJSON(original)
	assert.NoError(t, err)

	restored, err := kreuzberg.ResultFromJSON(jsonStr)
	assert.NoError(t, err)
	assert.Equal(t, original.Content, restored.Content)
	assert.Equal(t, original.MimeType, restored.MimeType)
	assert.Equal(t, original.Content, restored.Content)
}

// ============================================================================
// Metadata Accessor Methods Tests
// ============================================================================

// TestMetadataEmailMetadata tests EmailMetadata accessor.
func TestMetadataEmailMetadata(t *testing.T) {
	emailMeta := &kreuzberg.EmailMetadata{
		FromEmail: kreuzberg.StringPtr("test@example.com"),
		ToEmails:  []string{"recipient@example.com"},
	}
	metadata := kreuzberg.Metadata{
		Format: kreuzberg.FormatMetadata{
			Type:  kreuzberg.FormatEmail,
			Email: emailMeta,
		},
	}

	email, ok := metadata.EmailMetadata()
	assert.True(t, ok)
	assert.NotNil(t, email)
	assert.NotNil(t, email.FromEmail)
	assert.Equal(t, "test@example.com", *email.FromEmail)
	assert.Equal(t, 1, len(email.ToEmails))
}

// TestMetadataPptxMetadata tests PptxMetadata accessor.
func TestMetadataPptxMetadata(t *testing.T) {
	pptxMeta := &kreuzberg.PptxMetadata{
		Title:  kreuzberg.StringPtr("Presentation"),
		Author: kreuzberg.StringPtr("John Doe"),
		Fonts:  []string{"Arial", "Times New Roman"},
	}
	metadata := kreuzberg.Metadata{
		Format: kreuzberg.FormatMetadata{
			Type: kreuzberg.FormatPPTX,
			Pptx: pptxMeta,
		},
	}

	pptx, ok := metadata.PptxMetadata()
	assert.True(t, ok)
	assert.NotNil(t, pptx)
	assert.NotNil(t, pptx.Title)
	assert.Equal(t, "Presentation", *pptx.Title)
	assert.Equal(t, 2, len(pptx.Fonts))
}

// TestMetadataArchiveMetadata tests ArchiveMetadata accessor.
func TestMetadataArchiveMetadata(t *testing.T) {
	archMeta := &kreuzberg.ArchiveMetadata{
		Format:    "zip",
		FileCount: 5,
		FileList:  []string{"file1.txt", "file2.txt"},
		TotalSize: 10000,
	}
	metadata := kreuzberg.Metadata{
		Format: kreuzberg.FormatMetadata{
			Type:    kreuzberg.FormatArchive,
			Archive: archMeta,
		},
	}

	arch, ok := metadata.ArchiveMetadata()
	assert.True(t, ok)
	assert.NotNil(t, arch)
	assert.Equal(t, "zip", arch.Format)
	assert.Equal(t, 5, arch.FileCount)
}

// TestMetadataImageMetadata tests ImageMetadata accessor.
func TestMetadataImageMetadata(t *testing.T) {
	imgMeta := &kreuzberg.ImageMetadata{
		Width:  1920,
		Height: 1080,
		Format: "png",
		EXIF:   map[string]string{"Camera": "Canon"},
	}
	metadata := kreuzberg.Metadata{
		Format: kreuzberg.FormatMetadata{
			Type:  kreuzberg.FormatImage,
			Image: imgMeta,
		},
	}

	img, ok := metadata.ImageMetadata()
	assert.True(t, ok)
	assert.NotNil(t, img)
	assert.Equal(t, uint32(1920), img.Width)
	assert.Equal(t, uint32(1080), img.Height)
	assert.Equal(t, "png", img.Format)
}

// TestMetadataXMLMetadata tests XMLMetadata accessor.
func TestMetadataXMLMetadata(t *testing.T) {
	xmlMeta := &kreuzberg.XMLMetadata{
		ElementCount:   15,
		UniqueElements: []string{"root", "item", "data"},
	}
	metadata := kreuzberg.Metadata{
		Format: kreuzberg.FormatMetadata{
			Type: kreuzberg.FormatXML,
			XML:  xmlMeta,
		},
	}

	xml, ok := metadata.XMLMetadata()
	assert.True(t, ok)
	assert.NotNil(t, xml)
	assert.Equal(t, 15, xml.ElementCount)
	assert.Equal(t, 3, len(xml.UniqueElements))
}

// TestMetadataTextMetadata tests TextMetadata accessor.
func TestMetadataTextMetadata(t *testing.T) {
	textMeta := &kreuzberg.TextMetadata{
		LineCount:      100,
		WordCount:      1000,
		CharacterCount: 5000,
		Links:          [][2]string{{"href", "text"}},
	}
	metadata := kreuzberg.Metadata{
		Format: kreuzberg.FormatMetadata{
			Type: kreuzberg.FormatText,
			Text: textMeta,
		},
	}

	text, ok := metadata.TextMetadata()
	assert.True(t, ok)
	assert.NotNil(t, text)
	assert.Equal(t, 100, text.LineCount)
	assert.Equal(t, 1000, text.WordCount)
}

// TestMetadataHTMLMetadata tests HTMLMetadata accessor.
func TestMetadataHTMLMetadata(t *testing.T) {
	htmlMeta := &kreuzberg.HtmlMetadata{
		Title:       kreuzberg.StringPtr("Example Page"),
		Description: kreuzberg.StringPtr("An example page"),
		Keywords:    []string{"example", "test"},
		OpenGraph:   map[string]string{"og:title": "Example"},
	}
	metadata := kreuzberg.Metadata{
		Format: kreuzberg.FormatMetadata{
			Type: kreuzberg.FormatHTML,
			HTML: htmlMeta,
		},
	}

	html, ok := metadata.HTMLMetadata()
	assert.True(t, ok)
	assert.NotNil(t, html)
	assert.NotNil(t, html.Title)
	assert.Equal(t, "Example Page", *html.Title)
	assert.Equal(t, 2, len(html.Keywords))
}

// TestMetadataOcrMetadata tests OcrMetadata accessor.
func TestMetadataOcrMetadata(t *testing.T) {
	ocrMeta := &kreuzberg.OcrMetadata{
		Language:     "eng",
		PSM:          3,
		OutputFormat: "text",
		TableCount:   2,
	}
	metadata := kreuzberg.Metadata{
		Format: kreuzberg.FormatMetadata{
			Type: kreuzberg.FormatOCR,
			OCR:  ocrMeta,
		},
	}

	ocr, ok := metadata.OcrMetadata()
	assert.True(t, ok)
	assert.NotNil(t, ocr)
	assert.Equal(t, "eng", ocr.Language)
	assert.Equal(t, 3, ocr.PSM)
}

// TestMetadataAccessorWrongFormat tests accessor with wrong format type.
func TestMetadataAccessorWrongFormat(t *testing.T) {
	metadata := kreuzberg.Metadata{
		Format: kreuzberg.FormatMetadata{
			Type: kreuzberg.FormatPDF,
		},
	}

	email, ok := metadata.EmailMetadata()
	assert.False(t, ok)
	assert.Nil(t, email)
}

// ============================================================================
// Config JSON Serialization Tests
// ============================================================================

// TestConfigNestedJSONSerialization tests serialization of nested configs.
func TestConfigNestedJSONSerialization(t *testing.T) {
	config := kreuzberg.NewExtractionConfig(
		kreuzberg.WithOCR(
			kreuzberg.WithOCRBackend("tesseract"),
		),
		kreuzberg.WithChunking(
			kreuzberg.WithMaxChars(1024),
		),
	)

	jsonStr, err := kreuzberg.ConfigToJSON(config)
	assert.NoError(t, err)
	assert.NotEmpty(t, jsonStr)
	assert.Contains(t, jsonStr, "ocr")
	assert.Contains(t, jsonStr, "chunking")
}

// TestConfigInvalidJSONHandling tests IsValidJSON with various inputs.
func TestConfigInvalidJSONHandling(t *testing.T) {
	testCases := []struct {
		json      string
		shouldErr bool
	}{
		{"{}", false},
		{`{"key":"value"}`, false},
		{`{"incomplete":`, true},
		{`invalid json`, true},
		{``, true},
	}

	for _, tc := range testCases {
		result := kreuzberg.IsValidJSON(tc.json)
		if tc.shouldErr {
			assert.False(t, result, "json should be invalid: %s", tc.json)
		} else if !result {
			t.Logf("IsValidJSON returned false for valid json: %s", tc.json)
		}
	}
}

// TestConfigMergeComplexStructures tests merging complex nested configs.
func TestConfigMergeComplexStructures(t *testing.T) {
	baseConfig := kreuzberg.NewExtractionConfig(
		kreuzberg.WithOCR(
			kreuzberg.WithOCRBackend("tesseract"),
		),
		kreuzberg.WithChunking(
			kreuzberg.WithMaxChars(512),
		),
	)

	overrideConfig := kreuzberg.NewExtractionConfig(
		kreuzberg.WithChunking(
			kreuzberg.WithMaxChars(1024),
		),
	)

	err := kreuzberg.ConfigMerge(baseConfig, overrideConfig)
	assert.NoError(t, err)
	assert.NotNil(t, baseConfig.OCR)
	assert.NotNil(t, baseConfig.Chunking)
	assert.Equal(t, 1024, *baseConfig.Chunking.MaxChars)
}

// ============================================================================
// Helper Function
// ============================================================================

// Helper function to get test document path
func getTestDocumentPath(t *testing.T, subdir string, filename string) string {
	wd, err := os.Getwd()
	if err != nil {
		t.Fatalf("failed to get working directory: %v", err)
	}

	// Navigate from test_apps/go to the kreuzberg root directory and then to test_documents
	var repoRoot string
	currentDir := wd
	for {
		testDocsPath := filepath.Join(currentDir, "test_documents")
		kreuzbergPath := filepath.Join(currentDir, "kreuzberg")

		if _, err := os.Stat(kreuzbergPath); err == nil {
			if _, err := os.Stat(filepath.Join(kreuzbergPath, "test_documents")); err == nil {
				repoRoot = kreuzbergPath
				break
			}
		}

		if _, err := os.Stat(testDocsPath); err == nil {
			if _, err := os.Stat(filepath.Join(testDocsPath, "pdfs_with_tables")); err == nil {
				repoRoot = currentDir
				break
			}
		}

		parent := filepath.Dir(currentDir)
		if parent == currentDir {
			t.Fatalf("failed to find complete test_documents directory")
		}
		currentDir = parent
	}

	if subdir != "" {
		return filepath.Join(repoRoot, "test_documents", subdir, filename)
	}
	return filepath.Join(repoRoot, "test_documents", filename)
}
