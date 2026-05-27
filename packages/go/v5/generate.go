//go:generate go run ./cmd/download_ffi
//go:build ignore

// This file's sole purpose is to hold the go:generate directive that downloads
// the platform-specific FFI library from GitHub releases. This file is not compiled
// but its directives are processed by `go generate`.
package main
