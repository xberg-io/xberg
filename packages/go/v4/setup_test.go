package kreuzberg

import (
	"os"
	"sync"
	"testing"
)

// pdfiumOnce ensures Pdfium is initialized only once across all tests
var pdfiumOnce sync.Once

// init() runs before any tests in this package and initializes Pdfium
func init() {
	// Initialize Pdfium early to avoid "already initialized" errors
	_ = initializePdfium()
}

// TestMain is called before all tests run.
// We use it to initialize Pdfium once more (it's idempotent due to sync.Once)
func TestMain(m *testing.M) {
	// Initialize Pdfium by doing a dummy extraction that will trigger initialization
	// This ensures Pdfium is ready for all subsequent tests
	_ = initializePdfium()

	// Run all tests
	code := m.Run()
	os.Exit(code)
}

// initializePdfium triggers Pdfium initialization by performing a simple extraction
// This function is protected by sync.Once to ensure it's only called once
func initializePdfium() error {
	var err error
	pdfiumOnce.Do(func() {
		// Extract a simple text to initialize the library
		_, err = ExtractBytesSync([]byte("test"), "text/plain", nil)
	})
	return err
}
