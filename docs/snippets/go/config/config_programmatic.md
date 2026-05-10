```go title="Go"
package main

import (
	"log"

	"github.com/kreuzberg-dev/kreuzberg/packages/go/v5"
)

func main() {
	psm := int32(6)
	maxChars := uint(1000)
	overlap := uint(200)
	useCache := true

	config := kreuzberg.ExtractionConfig{
		UseCache: &useCache,
		Ocr: &kreuzberg.OcrConfig{
			Backend: "tesseract",
			TesseractConfig: &kreuzberg.TesseractConfig{
				Psm: &psm,
			},
		},
		Chunking: &kreuzberg.ChunkingConfig{
			MaxCharacters: &maxChars,
			Overlap:       &overlap,
		},
	}

	result, err := kreuzberg.ExtractFileSync("document.pdf", nil, config)
	if err != nil {
		log.Fatalf("extract failed: %v", err)
	}

	log.Printf("Content length: %d", len(result.Content))
}
```
