```go title="Go"
package main

import (
	"log"

	"github.com/kreuzberg-dev/kreuzberg/v5"
)

func main() {
	trueVal := true
	maxChars := uint(1000)
	overlap := uint(200)
	config := kreuzberg.ExtractionConfig{
		UseCache:                &trueVal,
		EnableQualityProcessing: &trueVal,
		Ocr: &kreuzberg.OcrConfig{
			Backend:   "tesseract",
			Language:  "eng",
		},
		Chunking: &kreuzberg.ChunkingConfig{
			MaxCharacters: &maxChars,
			Overlap:       &overlap,
		},
	}

	result, err := kreuzberg.ExtractFileSync("document.pdf", nil, config)
	if err != nil {
		log.Fatalf("extraction failed: %v", err)
	}

	println("Content length:", len(result.Content))
	println("Chunks:", len(result.Chunks))
}
```
