```go title="Go"
package main

import (
	"log"

	"github.com/kreuzberg-dev/kreuzberg/packages/go/v5"
)

// The Go binding does not currently expose plugin OCR backend registration.
// Use one of the built-in backends ("tesseract", "paddle-ocr", or VLM via "vlm").
func main() {
	result, err := kreuzberg.ExtractFileSync("scanned.pdf", nil, kreuzberg.ExtractionConfig{
		Ocr: &kreuzberg.OcrConfig{
			Backend:  "tesseract",
			Language: "eng",
		},
	})
	if err != nil {
		log.Fatalf("extract failed: %v", err)
	}

	log.Println("content length:", len(result.Content))
}
```
