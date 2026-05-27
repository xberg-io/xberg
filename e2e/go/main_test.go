package e2e_test

import (
	"bufio"
	"encoding/json"
	"io"
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
	// from e2e/go/. Repos without document fixtures (web crawler, network clients) do
	// not ship this directory — skip chdir and run from e2e/go/.
	testDocumentsDir := filepath.Join(dir, "..", "..", "test_documents")
	if info, err := os.Stat(testDocumentsDir); err == nil && info.IsDir() {
		if err := os.Chdir(testDocumentsDir); err != nil {
			panic(err)
		}
	}

	// If MOCK_SERVER_URL is already set, a parent process (e.g. `alef test-apps run`)
	// started a shared mock-server and exported its URL (plus any MOCK_SERVERS /
	// MOCK_SERVER_<FIXTURE_ID> vars). Use it as-is and do NOT spawn our own server.
	if os.Getenv("MOCK_SERVER_URL") != "" {
		os.Exit(m.Run())
	}

	// Start the mock HTTP server if it exists.
	mockServerBin := filepath.Join(dir, "..", "rust", "target", "release", "mock-server")
	if _, err := os.Stat(mockServerBin); err == nil {
		fixturesDir := filepath.Join(dir, "..", "..", "fixtures")
		cmd := exec.Command(mockServerBin, fixturesDir)
		cmd.Stderr = os.Stderr
		stdout, err := cmd.StdoutPipe()
		if err != nil {
			panic(err)
		}
		// Keep a writable pipe to the mock-server's stdin so the
		// server does not see EOF and exit immediately. The mock-server
		// blocks reading stdin until the parent closes the pipe.
		stdin, err := cmd.StdinPipe()
		if err != nil {
			panic(err)
		}
		if err := cmd.Start(); err != nil {
			panic(err)
		}
		scanner := bufio.NewScanner(stdout)
		for scanner.Scan() {
			line := scanner.Text()
			if strings.HasPrefix(line, "MOCK_SERVER_URL=") {
				_ = os.Setenv("MOCK_SERVER_URL", strings.TrimPrefix(line, "MOCK_SERVER_URL="))
			} else if strings.HasPrefix(line, "MOCK_SERVERS=") {
				_jsonVal := strings.TrimPrefix(line, "MOCK_SERVERS=")
				_ = os.Setenv("MOCK_SERVERS", _jsonVal)
				// Parse the JSON map and set per-fixture env vars (MOCK_SERVER_<FIXTURE_ID>).
				var _perFixture map[string]string
				if err := json.Unmarshal([]byte(_jsonVal), &_perFixture); err == nil {
					for _fid, _furl := range _perFixture {
						_ = os.Setenv("MOCK_SERVER_"+strings.ToUpper(_fid), _furl)
					}
				}
				break
			} else if os.Getenv("MOCK_SERVER_URL") != "" {
				break
			}
		}
		go func() { _, _ = io.Copy(io.Discard, stdout) }()
		code := m.Run()
		_ = stdin.Close()
		_ = cmd.Process.Signal(os.Interrupt)
		_ = cmd.Wait()
		os.Exit(code)
	} else {
		code := m.Run()
		os.Exit(code)
	}
}
