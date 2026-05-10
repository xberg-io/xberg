```go title="Go"
package main

import (
	"log"

	"github.com/kreuzberg-dev/kreuzberg/packages/go/v5"
)

func main() {
	result, err := kreuzberg.ExtractFileSync("multilingual.pdf", nil, kreuzberg.ExtractionConfig{
		Ocr: &kreuzberg.OcrConfig{
			Backend:  "tesseract",
			Language: "eng+deu+fra",
		},
	})
	if err != nil {
		log.Fatalf("extract failed: %v", err)
	}

	log.Println(result.Content)
}
```
