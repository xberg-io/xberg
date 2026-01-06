package kreuzberg

import (
	"os"
	"testing"
)

// TestMain is called before all tests run.
func TestMain(m *testing.M) {
	// Run all tests
	code := m.Run()
	os.Exit(code)
}
