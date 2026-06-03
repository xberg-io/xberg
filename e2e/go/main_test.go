package e2e_test

import (
	"bufio"
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
	"runtime"
	"strings"
	"testing"
)

func TestMain(m *testing.M) {
	_, filename, _, _ := runtime.Caller(0)
	dir := filepath.Dir(filename)

	// Change to the configured test-documents directory (if it exists) so that fixture
	// file paths like "pdf/fake_memo.pdf" resolve correctly when running go test
	// from e2e/go/. Repos without document fixtures skip chdir and run from e2e/go/.
	testDocumentsDir := filepath.Join(dir, "..", "..", "test_documents")
	if info, err := os.Stat(testDocumentsDir); err == nil && info.IsDir() {
		if err := os.Chdir(testDocumentsDir); err != nil {
			panic(err)
		}
	}

	if os.Getenv("MOCK_SERVER_URL") != "" {
		os.Exit(m.Run())
	}

	mockBin := filepath.Join(dir, "..", "rust", "target", "release", "mock-server")
	mockManifest := filepath.Join(dir, "..", "rust", "Cargo.toml")
	if _, err := os.Stat(mockBin); os.IsNotExist(err) {
		fmt.Fprintln(os.Stderr, "Building mock-server...")
		build := exec.Command("cargo", "build", "--release", "--manifest-path", mockManifest, "--bin", "mock-server")
		build.Stdout = os.Stderr
		build.Stderr = os.Stderr
		if err := build.Run(); err != nil {
			panic(fmt.Sprintf("mock-server build failed: %v", err))
		}
	}

	fixturesDir := filepath.Join(dir, "..", "..", "fixtures")
	cmd := exec.Command(mockBin, fixturesDir)
	stdout, err := cmd.StdoutPipe()
	if err != nil { panic(err) }
	cmd.Stderr = os.Stderr
	if err := cmd.Start(); err != nil { panic(err) }
	defer func() { _ = cmd.Process.Kill() }()

	scanner := bufio.NewScanner(stdout)
	scanner.Buffer(make([]byte, 0, 64*1024), 1024*1024)
	for scanner.Scan() {
		line := scanner.Text()
		if strings.HasPrefix(line, "MOCK_SERVER_URL=") {
			_ = os.Setenv("MOCK_SERVER_URL", strings.TrimPrefix(line, "MOCK_SERVER_URL="))
			break
		}
	}
	if os.Getenv("MOCK_SERVER_URL") == "" {
		panic("mock-server did not emit MOCK_SERVER_URL")
	}
	// Drain remaining stdout asynchronously so the pipe doesn't fill.
	go func() { for scanner.Scan() { } }()

	os.Exit(m.Run())
}
