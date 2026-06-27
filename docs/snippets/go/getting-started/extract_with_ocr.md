```go title="Go"
package main

import (
	"fmt"
	"log"

	"github.com/xberg-io/xberg/packages/go"
)

func main() {
	ocrConfig := &xberg.OcrConfig{
		Backend:  "tesseract",
		Language: "eng",
	}

	config := xberg.ExtractionConfig{
		Ocr: ocrConfig,
	}

	result, err := xberg.ExtractSync("scanned.pdf", nil, config)
	if err != nil {
		log.Fatalf("extract failed: %v", err)
	}

	fmt.Println("Extracted text from scanned document:")
	fmt.Println(result.Content)
	fmt.Println("Used OCR backend: tesseract")
}
```
