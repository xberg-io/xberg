```go title="Go"
package main

import (
	"fmt"
	"log"

	"github.com/xberg-io/xberg/packages/go"
)

func main() {
	maxChars := uint(1000)
	maxOverlap := uint(100)
	useCache := true
	enableQuality := true
	languageDetectionEnabled := true

	config := xberg.ExtractionConfig{
		Ocr: &xberg.OcrConfig{
			Backend:  "tesseract",
			Language: "eng+deu",
		},
		Chunking: &xberg.ChunkingConfig{
			MaxCharacters: &maxChars,
			Overlap:       &maxOverlap,
		},
		LanguageDetection: &xberg.LanguageDetectionConfig{
			Enabled:        &languageDetectionEnabled,
			DetectMultiple: true,
		},
		UseCache:                &useCache,
		EnableQualityProcessing: &enableQuality,
	}

	result, err := xberg.ExtractSync("document.pdf", nil, config)
	if err != nil {
		log.Fatalf("extract failed: %v", err)
	}

	// Access chunks
	if len(result.Chunks) > 0 {
		snippet := result.Chunks[0].Content
		if len(snippet) > 100 {
			snippet = snippet[:100]
		}
		fmt.Printf("First chunk: %s...\n", snippet)
	}

	// Access detected languages
	if len(result.DetectedLanguages) > 0 {
		fmt.Printf("Languages: %v\n", result.DetectedLanguages)
	}
}
```
