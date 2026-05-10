```go title="Go"
package main

import (
	"log"

	"github.com/kreuzberg-dev/kreuzberg/packages/go/v5"
)

func main() {
	cfg := kreuzberg.ExtractionConfig{
		Ocr: &kreuzberg.OcrConfig{
			Backend:  "paddle-ocr",
			Language: "en",
		},
	}

	result, err := kreuzberg.ExtractFileSync("scanned.pdf", nil, cfg)
	if err != nil {
		log.Fatalf("extract failed: %v", err)
	}
	log.Println(len(result.Content))
}
```
