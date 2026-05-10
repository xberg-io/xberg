```go title="Go"
package main

import (
	"log"

	"github.com/kreuzberg-dev/kreuzberg/packages/go/v5"
)

func main() {
	targetDpi := int32(300)
	result, err := kreuzberg.ExtractFileSync("scanned.pdf", nil, kreuzberg.ExtractionConfig{
		Ocr: &kreuzberg.OcrConfig{
			Backend: "tesseract",
			TesseractConfig: &kreuzberg.TesseractConfig{
				Preprocessing: &kreuzberg.ImagePreprocessingConfig{
					TargetDpi: &targetDpi,
				},
			},
		},
	})
	if err != nil {
		log.Fatalf("extract failed: %v", err)
	}

	log.Println("content length:", len(result.Content))
}
```
