// Test setup to configure the working directory and library paths for e2e tests.
package e2e_test

import (
	"os"
	"path/filepath"
	"testing"
)

func TestMain(m *testing.M) {
	// Change to test_documents directory so that fixture file paths like
	// "pdf/fake_memo.pdf" resolve correctly.
	testDocsPath := filepath.Join("..", "..", "test_documents")
	if err := os.Chdir(testDocsPath); err != nil {
		panic(err)
	}

	// Set DYLD_LIBRARY_PATH for macOS to find libpdfium.dylib and libkreuzberg_ffi.dylib
	// from the Cargo release build directory.
	repoRoot := filepath.Join("..", "..")
	libPath := filepath.Join(repoRoot, "target", "release")
	existingPath := os.Getenv("DYLD_LIBRARY_PATH")
	if existingPath != "" {
		os.Setenv("DYLD_LIBRARY_PATH", libPath+":"+existingPath)
	} else {
		os.Setenv("DYLD_LIBRARY_PATH", libPath)
	}

	os.Exit(m.Run())
}
