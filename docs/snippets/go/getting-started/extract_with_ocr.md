```go title="Go"
package main

import (
	"fmt"
	"log"

	"github.com/kreuzberg-dev/kreuzberg/packages/go/v5"
)

func main() {
	ocrConfig := &kreuzberg.OcrConfig{
		Backend:  "tesseract",
		Language: "eng",
	}

	config := kreuzberg.ExtractionConfig{
		Ocr: ocrConfig,
	}

	result, err := kreuzberg.ExtractFileSync("scanned.pdf", nil, config)
	if err != nil {
		log.Fatalf("extract failed: %v", err)
	}

	fmt.Println("Extracted text from scanned document:")
	fmt.Println(result.Content)
	fmt.Println("Used OCR backend: tesseract")
}
```
