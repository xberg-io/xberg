package main

import (
	"bufio"
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"
	"time"

	kz "github.com/kreuzberg-dev/kreuzberg/packages/go/v4"
)

var debugEnabled = os.Getenv("KREUZBERG_BENCHMARK_DEBUG") != ""

func debug(msg string, args ...interface{}) {
	if debugEnabled {
		fmt.Fprintf(os.Stderr, "[DEBUG] "+msg+"\n", args...)
	}
}

type payload struct {
	Content          string         `json:"content"`
	Metadata         map[string]any `json:"metadata"`
	ExtractionTimeMs float64        `json:"_extraction_time_ms"`
	BatchTotalTimeMs float64        `json:"_batch_total_ms,omitempty"`
}

func main() {
	debug("Kreuzberg Go extraction script started")
	debug("Command-line args: %v", os.Args)
	debug("Working directory: %s", getWorkingDir())
	debug("LD_LIBRARY_PATH: %s", os.Getenv("LD_LIBRARY_PATH"))
	debug("DYLD_LIBRARY_PATH: %s", os.Getenv("DYLD_LIBRARY_PATH"))

	ocrEnabled := false
	var args []string

	// Parse OCR flags
	for _, arg := range os.Args[1:] {
		if arg == "--ocr" {
			ocrEnabled = true
		} else if arg == "--no-ocr" {
			ocrEnabled = false
		} else {
			args = append(args, arg)
		}
	}

	if len(args) < 1 {
		fmt.Fprintln(os.Stderr, "Usage: kreuzberg_extract_go [--ocr|--no-ocr] <mode> <file_path> [additional_files...]")
		fmt.Fprintln(os.Stderr, "Modes: sync, batch, server")
		os.Exit(1)
	}

	mode := args[0]
	files := args[1:]

	debug("Mode: %s, OCR enabled: %v, Files: %v", mode, ocrEnabled, files)

	switch mode {
	case "server":
		debug("Starting server mode")
		runServer(ocrEnabled)
	case "sync":
		if len(files) != 1 {
			fatal(fmt.Errorf("sync mode requires exactly one file"))
		}
		debug("Starting sync extraction for: %s", files[0])
		result, err := extractSync(files[0], ocrEnabled)
		if err != nil {
			fatal(err)
		}
		debug("Sync extraction completed successfully")
		mustEncode(result)
	case "batch":
		if len(files) == 0 {
			fatal(fmt.Errorf("batch mode requires at least one file"))
		}
		debug("Starting batch extraction for %d files", len(files))
		results, err := extractBatch(files, ocrEnabled)
		if err != nil {
			fatal(err)
		}
		debug("Batch extraction completed successfully")
		mustEncode(results)
	default:
		fatal(fmt.Errorf("unknown mode %q", mode))
	}
}

func getWorkingDir() string {
	pwd, err := os.Getwd()
	if err != nil {
		return fmt.Sprintf("<error: %v>", err)
	}
	return pwd
}

func boolPtr(v bool) *bool { return &v }

func createConfig(ocrEnabled bool) *kz.ExtractionConfig {
	config := &kz.ExtractionConfig{
		UseCache: boolPtr(false),
	}
	if ocrEnabled {
		config.OCR = &kz.OCRConfig{}
	}
	return config
}

func runServer(ocrEnabled bool) {
	debug("Server mode: reading paths from stdin")
	config := createConfig(ocrEnabled)
	scanner := bufio.NewScanner(os.Stdin)

	for scanner.Scan() {
		filePath := scanner.Text()
		if filePath = filepath.Clean(filePath); filePath == "" {
			continue
		}

		absPath, err := filepath.Abs(filePath)
		if err != nil {
			debug("Failed to resolve path %s: %v", filePath, err)
			mustEncodeError(err)
			continue
		}

		start := time.Now()
		result, err := kz.ExtractFileSync(absPath, config)
		if err != nil {
			debug("Extraction failed for %s: %v", absPath, err)
			mustEncodeError(err)
			continue
		}

		elapsed := time.Since(start).Seconds() * 1000.0
		meta, err := metadataMap(result.Metadata)
		if err != nil {
			debug("metadataMap failed: %v", err)
			mustEncodeError(err)
			continue
		}

		p := &payload{
			Content:          result.Content,
			Metadata:         meta,
			ExtractionTimeMs: elapsed,
		}
		mustEncodeNoNewline(p)
		fmt.Println()
		os.Stdout.Sync()
	}

	if err := scanner.Err(); err != nil {
		debug("Scanner error: %v", err)
	}
}

func extractSync(path string, ocrEnabled bool) (*payload, error) {
	start := time.Now()
	debug("ExtractFileSync called with path: %s", path)

	absPath, err := filepath.Abs(path)
	if err != nil {
		debug("filepath.Abs failed for %s: %v", path, err)
		return nil, fmt.Errorf("failed to resolve path: %w", err)
	}
	debug("Resolved absolute path: %s", absPath)

	config := createConfig(ocrEnabled)
	result, err := kz.ExtractFileSync(absPath, config)
	if err != nil {
		debug("ExtractFileSync failed: %v", err)
		return nil, err
	}
	elapsed := time.Since(start).Seconds() * 1000.0
	debug("ExtractFileSync succeeded, elapsed: %.2f ms", elapsed)
	meta, err := metadataMap(result.Metadata)
	if err != nil {
		debug("metadataMap failed: %v", err)
		return nil, err
	}
	return &payload{
		Content:          result.Content,
		Metadata:         meta,
		ExtractionTimeMs: elapsed,
	}, nil
}

func extractBatch(paths []string, ocrEnabled bool) (any, error) {
	start := time.Now()
	debug("BatchExtractFilesSync called with %d files", len(paths))

	absPaths := make([]string, len(paths))
	for i, path := range paths {
		absPath, err := filepath.Abs(path)
		if err != nil {
			debug("filepath.Abs failed for %s: %v", path, err)
			return nil, fmt.Errorf("failed to resolve path %s: %w", path, err)
		}
		absPaths[i] = absPath
		debug("Resolved path %d: %s -> %s", i, path, absPath)
	}

	config := createConfig(ocrEnabled)
	results, err := kz.BatchExtractFilesSync(absPaths, config)
	if err != nil {
		debug("BatchExtractFilesSync failed: %v", err)
		return nil, err
	}
	totalMs := time.Since(start).Seconds() * 1000.0
	debug("BatchExtractFilesSync succeeded, %d results, total elapsed: %.2f ms", len(results), totalMs)
	if len(paths) == 1 && len(results) == 1 {
		meta, err := metadataMap(results[0].Metadata)
		if err != nil {
			return nil, err
		}
		return &payload{
			Content:          results[0].Content,
			Metadata:         meta,
			ExtractionTimeMs: totalMs,
			BatchTotalTimeMs: totalMs,
		}, nil
	}

	out := make([]*payload, 0, len(results))
	perMs := totalMs / float64(max(len(results), 1))
	for _, item := range results {
		if item == nil {
			continue
		}
		meta, err := metadataMap(item.Metadata)
		if err != nil {
			return nil, err
		}
		out = append(out, &payload{
			Content:          item.Content,
			Metadata:         meta,
			ExtractionTimeMs: perMs,
			BatchTotalTimeMs: totalMs,
		})
	}
	return out, nil
}

func metadataMap(meta kz.Metadata) (map[string]any, error) {
	bytes, err := json.Marshal(meta)
	if err != nil {
		return nil, err
	}
	var out map[string]any
	if err := json.Unmarshal(bytes, &out); err != nil {
		return nil, err
	}
	return out, nil
}

func mustEncode(value any) {
	debug("Encoding result to JSON")
	data, err := json.Marshal(value)
	if err != nil {
		debug("JSON encoding failed: %v", err)
		fatal(err)
	}
	_, err = os.Stdout.Write(data)
	if err != nil {
		debug("JSON write failed: %v", err)
		fatal(err)
	}
	debug("JSON output complete")
}

func mustEncodeNoNewline(value any) {
	debug("Encoding result to JSON (no newline)")
	data, err := json.Marshal(value)
	if err != nil {
		debug("JSON encoding failed: %v", err)
		fatal(err)
	}
	_, err = os.Stdout.Write(data)
	if err != nil {
		debug("JSON write failed: %v", err)
		fatal(err)
	}
}

func mustEncodeError(err error) {
	errorMap := map[string]interface{}{
		"error":                 err.Error(),
		"_extraction_time_ms": 0,
	}
	data, marshalErr := json.Marshal(errorMap)
	if marshalErr != nil {
		debug("JSON encoding failed: %v", marshalErr)
		fmt.Fprintf(os.Stdout, "{\"error\":\"encoding failed\",\"_extraction_time_ms\":0}\n")
		return
	}
	fmt.Println(string(data))
}

func fatal(err error) {
	fmt.Fprintf(os.Stderr, "Error extracting with Go binding: %v\n", err)
	debug("Exiting with error: %v", err)
	os.Exit(1)
}

func max(a, b int) int {
	if a > b {
		return a
	}
	return b
}
