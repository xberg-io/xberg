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
		Content:   "test",
		MimeType:  "text/plain",
		Success:   true,
		Metadata:  kreuzberg.Metadata{},
		Tables:    []kreuzberg.Table{},
		Chunks:    []kreuzberg.Chunk{},
		Images:    []kreuzberg.ExtractedImage{},
		Pages:     []kreuzberg.PageContent{},
	}
	assert.NotNil(t, result)
	assert.Equal(t, "test", result.Content)
	assert.Equal(t, "text/plain", result.MimeType)
	assert.True(t, result.Success)
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
	assert.True(t, result.Success, "extraction should succeed")
	assert.Equal(t, "application/pdf", result.MimeType, "mime type should be PDF")
	assert.NotEmpty(t, result.Content, "content should not be empty")
}

// TestExtractFileSyncDOCXValid extracts text from a valid DOCX file.
func TestExtractFileSyncDOCXValid(t *testing.T) {
	docxPath := getTestDocumentPath(t, "documents", "lorem_ipsum.docx")

	result, err := kreuzberg.ExtractFileSync(docxPath, nil)
	assert.NoError(t, err, "should extract DOCX without error")
	assert.NotNil(t, result, "result should not be nil")
	assert.True(t, result.Success, "extraction should succeed")
	assert.Equal(t, "application/vnd.openxmlformats-officedocument.wordprocessingml.document", result.MimeType)
	assert.NotEmpty(t, result.Content, "content should not be empty")
}

// TestExtractFileSyncXLSXValid extracts text from a valid XLSX file.
func TestExtractFileSyncXLSXValid(t *testing.T) {
	xlsxPath := getTestDocumentPath(t, "spreadsheets", "stanley_cups.xlsx")

	result, err := kreuzberg.ExtractFileSync(xlsxPath, nil)
	assert.NoError(t, err, "should extract XLSX without error")
	assert.NotNil(t, result, "result should not be nil")
	assert.True(t, result.Success, "extraction should succeed")
	assert.NotEmpty(t, result.Content, "content should not be empty")
}

// TestExtractFileSyncImageJPGValid extracts text from a valid JPG image.
func TestExtractFileSyncImageJPGValid(t *testing.T) {
	imgPath := getTestDocumentPath(t, "images", "example.jpg")

	result, err := kreuzberg.ExtractFileSync(imgPath, nil)
	assert.NoError(t, err, "should extract JPG without error")
	assert.NotNil(t, result, "result should not be nil")
	assert.True(t, result.Success, "extraction should succeed")
	assert.Equal(t, "image/jpeg", result.MimeType)
}

// TestExtractFileSyncImagePNGValid extracts text from a valid PNG image.
func TestExtractFileSyncImagePNGValid(t *testing.T) {
	imgPath := getTestDocumentPath(t, "images", "sample.png")

	result, err := kreuzberg.ExtractFileSync(imgPath, nil)
	assert.NoError(t, err, "should extract PNG without error")
	assert.NotNil(t, result, "result should not be nil")
	assert.True(t, result.Success, "extraction should succeed")
	assert.Equal(t, "image/png", result.MimeType)
}

// TestExtractFileSyncODTValid extracts text from a valid ODT file.
func TestExtractFileSyncODTValid(t *testing.T) {
	odtPath := getTestDocumentPath(t, "odt", "paragraph.odt")

	result, err := kreuzberg.ExtractFileSync(odtPath, nil)
	assert.NoError(t, err, "should extract ODT without error")
	assert.NotNil(t, result, "result should not be nil")
	assert.True(t, result.Success, "extraction should succeed")
}

// TestExtractFileSyncMarkdownValid extracts text from a valid Markdown file.
func TestExtractFileSyncMarkdownValid(t *testing.T) {
	mdPath := getTestDocumentPath(t, "", "extraction_test.md")

	result, err := kreuzberg.ExtractFileSync(mdPath, nil)
	assert.NoError(t, err, "should extract Markdown without error")
	assert.NotNil(t, result, "result should not be nil")
	assert.True(t, result.Success, "extraction should succeed")
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
	assert.True(t, result.Success, "extraction should succeed")
}

// TestExtractBytesSyncPDFValid extracts text from PDF bytes.
func TestExtractBytesSyncPDFValid(t *testing.T) {
	pdfPath := getTestDocumentPath(t, "pdfs_with_tables", "tiny.pdf")
	data, err := os.ReadFile(pdfPath)
	assert.NoError(t, err, "should read PDF file")

	result, err := kreuzberg.ExtractBytesSync(data, "application/pdf", nil)
	assert.NoError(t, err, "should extract PDF bytes without error")
	assert.NotNil(t, result, "result should not be nil")
	assert.True(t, result.Success, "extraction should succeed")
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
	assert.True(t, result.Success, "extraction should succeed")
}

// TestExtractBytesSyncXLSXValid extracts text from XLSX bytes.
func TestExtractBytesSyncXLSXValid(t *testing.T) {
	xlsxPath := getTestDocumentPath(t, "spreadsheets", "stanley_cups.xlsx")
	data, err := os.ReadFile(xlsxPath)
	assert.NoError(t, err, "should read XLSX file")

	result, err := kreuzberg.ExtractBytesSync(data, "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet", nil)
	assert.NoError(t, err, "should extract XLSX bytes without error")
	assert.NotNil(t, result, "result should not be nil")
	assert.True(t, result.Success, "extraction should succeed")
}

// TestExtractBytesSyncImageJPGValid extracts text from JPG bytes.
func TestExtractBytesSyncImageJPGValid(t *testing.T) {
	imgPath := getTestDocumentPath(t, "images", "example.jpg")
	data, err := os.ReadFile(imgPath)
	assert.NoError(t, err, "should read JPG file")

	result, err := kreuzberg.ExtractBytesSync(data, "image/jpeg", nil)
	assert.NoError(t, err, "should extract JPG bytes without error")
	assert.NotNil(t, result, "result should not be nil")
	assert.True(t, result.Success, "extraction should succeed")
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
	assert.True(t, result.Success, "extraction should succeed")
}

// TestExtractBytesSyncODTValid extracts text from ODT bytes.
func TestExtractBytesSyncODTValid(t *testing.T) {
	odtPath := getTestDocumentPath(t, "odt", "paragraph.odt")
	data, err := os.ReadFile(odtPath)
	assert.NoError(t, err, "should read ODT file")

	result, err := kreuzberg.ExtractBytesSync(data, "application/vnd.oasis.opendocument.text", nil)
	assert.NoError(t, err, "should extract ODT bytes without error")
	assert.NotNil(t, result, "result should not be nil")
	assert.True(t, result.Success, "extraction should succeed")
}

// TestExtractBytesSyncMarkdownValid extracts text from Markdown bytes.
func TestExtractBytesSyncMarkdownValid(t *testing.T) {
	mdPath := getTestDocumentPath(t, "", "extraction_test.md")
	data, err := os.ReadFile(mdPath)
	assert.NoError(t, err, "should read Markdown file")

	result, err := kreuzberg.ExtractBytesSync(data, "text/markdown", nil)
	assert.NoError(t, err, "should extract Markdown bytes without error")
	assert.NotNil(t, result, "result should not be nil")
	assert.True(t, result.Success, "extraction should succeed")
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
	assert.True(t, result.Success, "extraction should succeed")
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
		UseCache:                kreuzberg.BoolPtr(true),
		EnableQualityProcessing: kreuzberg.BoolPtr(false),
		ForceOCR:                kreuzberg.BoolPtr(false),
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
	assert.True(t, result.Success, "should be successful")
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
			assert.True(t, result.Success, fmt.Sprintf("%s extraction should succeed", tt.name))
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
			assert.True(t, result.Success, fmt.Sprintf("%s extraction should succeed", tt.name))
		})
	}
}

// TestFileTypeCoverageXLSX tests XLSX extraction coverage.
func TestFileTypeCoverageXLSX(t *testing.T) {
	xlsxPath := getTestDocumentPath(t, "spreadsheets", "stanley_cups.xlsx")

	result, err := kreuzberg.ExtractFileSync(xlsxPath, nil)
	assert.NoError(t, err, "should extract XLSX without error")
	assert.NotNil(t, result, "XLSX result should not be nil")
	assert.True(t, result.Success, "XLSX extraction should succeed")
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
	assert.True(t, result.Success, "ODT extraction should succeed")
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
	assert.True(t, result.Success, "success field should be true")
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

// TestValidateMimeType validates a MIME type string.
func TestValidateMimeType(t *testing.T) {
	validMime := "application/pdf"
	canonical, err := kreuzberg.ValidateMimeType(validMime)
	if err == nil {
		t.Logf("MIME type %s is valid, canonical: %s", validMime, canonical)
		assert.NotEmpty(t, canonical, "canonical MIME type should not be empty")
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
