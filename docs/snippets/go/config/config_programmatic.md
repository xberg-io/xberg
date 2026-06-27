```go title="Go"
package main

import (
	"log"

	"github.com/xberg-io/xberg/packages/go"
)

func main() {
	psm := int32(6)
	maxChars := uint(1000)
	overlap := uint(200)
	useCache := true

	config := xberg.ExtractionConfig{
		UseCache: &useCache,
		Ocr: &xberg.OcrConfig{
			Backend: "tesseract",
			TesseractConfig: &xberg.TesseractConfig{
				Psm: &psm,
			},
		},
		Chunking: &xberg.ChunkingConfig{
			MaxCharacters: &maxChars,
			Overlap:       &overlap,
		},
	}

	result, err := xberg.ExtractSync("document.pdf", nil, config)
	if err != nil {
		log.Fatalf("extract failed: %v", err)
	}

	log.Printf("Content length: %d", len(result.Content))
}
```
