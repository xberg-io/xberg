// Tool to download platform-specific FFI libraries from GitHub releases.
// Invoked via `go generate` before compilation.
//go:build ignore
// +build ignore

package main

import (
	"archive/tar"
	"compress/gzip"
	"fmt"
	"io"
	"net/http"
	"os"
	"path/filepath"
	"runtime"
	"strings"
)

const (
	moduleVersion = "5.0.0-rc.2"
	repoURL       = "https://github.com/kreuzberg-dev/kreuzberg"
	assetPrefix   = "kreuzberg"
)

func main() {
	if err := run(); err != nil {
		fmt.Fprintf(os.Stderr, "Error: %v\n", err)
		os.Exit(1)
	}
}

func run() error {
	// Determine cache and library paths
	cacheBase, libPath, err := determinePaths()
	if err != nil {
		return err
	}

	// Check if library already exists
	if _, err := os.Stat(libPath); err == nil {
		// Library already cached, nothing to do
		return nil
	}

	// Download the FFI library tarball
	if err := downloadAndExtractLibrary(cacheBase); err != nil {
		return fmt.Errorf("failed to download FFI library: %w", err)
	}

	return nil
}

func determinePaths() (string, string, error) {
	goos := runtime.GOOS
	goarch := runtime.GOARCH

	// Map Go platform names to asset names
	osName := goos
	if goos == "darwin" {
		osName = "macos"
	}

	libName := "kreuzberg_ffi"
	// Use the current working directory as the module root (where this script is run from)
	// and place libraries in .lib/{os}-{arch}/
	moduleRoot, err := os.Getwd()
	if err != nil {
		return "", "", fmt.Errorf("cannot determine module root: %w", err)
	}

	libDir := filepath.Join(moduleRoot, ".lib", fmt.Sprintf("%s-%s", osName, goarch))
	libPath := filepath.Join(libDir, libFilename(libName, goos))

	return libDir, libPath, nil
}

func libFilename(libName, goos string) string {
	switch goos {
	case "windows":
		return libName + ".dll"
	case "darwin":
		return "lib" + libName + ".dylib"
	default:
		return "lib" + libName + ".so"
	}
}

func downloadAndExtractLibrary(cacheDir string) error {
	goos := runtime.GOOS
	goarch := runtime.GOARCH

	osName := goos
	if goos == "darwin" {
		osName = "macos"
	}

	// Map Go arch names to the alef platform names used in release asset filenames.
	// The local .lib/<os>-<goarch>/ directories use Go arch names (matching cgo LDFLAGS),
	// but alef's packager emits tarballs with its own arch names: x86_64, aarch64.
	archName := goarch
	switch goarch {
	case "amd64":
		archName = "x86_64"
	case "arm64":
		// macOS arm64 stays "arm64" (alef go_java_platform special-cases it);
		// all other platforms use "aarch64".
		if goos != "darwin" {
			archName = "aarch64"
		}
	}

	// Clean version for asset name
	version := strings.TrimPrefix(moduleVersion, "v")
	assetName := fmt.Sprintf("%s-go-v%s-%s-%s.tar.gz", assetPrefix, version, osName, archName)
	downloadURL := fmt.Sprintf("%s/releases/download/v%s/%s", repoURL, version, assetName)

	// Create cache directory
	if err := os.MkdirAll(cacheDir, 0755); err != nil {
		return fmt.Errorf("mkdir cache: %w", err)
	}

	// Download tarball
	resp, err := http.Get(downloadURL)
	if err != nil {
		return fmt.Errorf("download %s: %w", downloadURL, err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != 200 {
		body, _ := io.ReadAll(resp.Body)
		return fmt.Errorf("HTTP %d: %s", resp.StatusCode, string(body))
	}

	// Extract tarball to cache directory
	if err := extractTarGz(resp.Body, cacheDir); err != nil {
		return fmt.Errorf("extract tarball: %w", err)
	}

	return nil
}

func extractTarGz(src io.Reader, dstDir string) error {
	gzr, err := gzip.NewReader(src)
	if err != nil {
		return err
	}
	defer gzr.Close()

	tr := tar.NewReader(gzr)
	for {
		header, err := tr.Next()
		if err == io.EOF {
			break
		}
		if err != nil {
			return err
		}

		// Construct destination path, stripping any leading directory
		// from the tarball (e.g., "staging/lib..." -> "lib...")
		targetPath := filepath.Join(dstDir, filepath.Base(header.Name))

		switch header.Typeflag {
		case tar.TypeReg:
			f, err := os.OpenFile(targetPath, os.O_CREATE|os.O_WRONLY|os.O_TRUNC, os.FileMode(header.Mode))
			if err != nil {
				return err
			}
			if _, err := io.Copy(f, tr); err != nil {
				f.Close()
				return err
			}
			f.Close()

		case tar.TypeDir:
			if err := os.MkdirAll(targetPath, os.FileMode(header.Mode)); err != nil {
				return err
			}
		}
	}

	return nil
}
