```go title="Go"
package main

import (
	"log"

	"github.com/xberg-io/xberg/packages/go"
)

func main() {
	cfg := xberg.ExtractionConfig{
		Ocr: &xberg.OcrConfig{
			Backend:  "tesseract",
			Language: "eng",
		},
	}

	result, err := xberg.ExtractSync("scanned.pdf", nil, cfg)
	if err != nil {
		log.Fatalf("extract failed: %v", err)
	}
	log.Println(len(result.Content))
}
```
