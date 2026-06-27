```go title="Go"
package main

import (
	"log"

	"github.com/xberg-io/xberg/packages/go"
)

// The Go binding does not currently expose plugin OCR backend registration.
// Use one of the built-in backends ("tesseract", "paddle-ocr", or VLM via "vlm").
func main() {
	result, err := xberg.ExtractSync("scanned.pdf", nil, xberg.ExtractionConfig{
		Ocr: &xberg.OcrConfig{
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
