package kreuzberg_test

import (
	"fmt"
	"os"
	"testing"
)

// createTestPDF creates a minimal valid PDF file for testing.
// Returns the path to the created file.
func createTestPDF(t *testing.T) string {
	pdfContent := `%PDF-1.4
1 0 obj<</Type/Catalog/Pages 2 0 R>>endobj
2 0 obj<</Type/Pages/Kids[3 0 R]/Count 1>>endobj
3 0 obj<</Type/Page/Parent 2 0 R/MediaBox[0 0 612 792]/Contents 4 0 R/Resources<<>>>>endobj
4 0 obj<</Length 44>>stream
BT /F1 12 Tf 100 700 Td (Hello World) Tj ET
endstream endobj
xref
0 5
0000000000 65535 f
0000000009 00000 n
0000000058 00000 n
0000000115 00000 n
0000000203 00000 n
trailer<</Size 5/Root 1 0 R>>
startxref
297
%%EOF
`

	tmpFile, err := os.CreateTemp("", "test-*.pdf")
	if err != nil {
		t.Fatalf("failed to create temp PDF file: %v", err)
	}
	defer tmpFile.Close()

	if _, err := tmpFile.WriteString(pdfContent); err != nil {
		t.Fatalf("failed to write PDF content: %v", err)
	}

	return tmpFile.Name()
}

// generateTestPDFBytes generates minimal valid PDF bytes for testing.
func generateTestPDFBytes(t *testing.T) []byte {
	pdfContent := `%PDF-1.4
1 0 obj<</Type/Catalog/Pages 2 0 R>>endobj
2 0 obj<</Type/Pages/Kids[3 0 R]/Count 1>>endobj
3 0 obj<</Type/Page/Parent 2 0 R/MediaBox[0 0 612 792]/Contents 4 0 R/Resources<<>>>>endobj
4 0 obj<</Length 44>>stream
BT /F1 12 Tf 100 700 Td (Hello World) Tj ET
endstream endobj
xref
0 5
0000000000 65535 f
0000000009 00000 n
0000000058 00000 n
0000000115 00000 n
0000000203 00000 n
trailer<</Size 5/Root 1 0 R>>
startxref
297
%%EOF
`
	return []byte(pdfContent)
}

// cleanup removes the temporary PDF file created by createTestPDF.
func cleanup(path string) {
	if err := os.Remove(path); err != nil {
		fmt.Printf("warning: failed to cleanup temp file %s: %v\n", path, err)
	}
}
