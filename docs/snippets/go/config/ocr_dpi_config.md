```go title="Go"
package main

import (
	"log"

	"github.com/xberg-io/xberg/packages/go"
)

func main() {
	targetDpi := int32(300)
	result, err := xberg.ExtractSync("scanned.pdf", nil, xberg.ExtractionConfig{
		Ocr: &xberg.OcrConfig{
			Backend: "tesseract",
			TesseractConfig: &xberg.TesseractConfig{
				Preprocessing: &xberg.ImagePreprocessingConfig{
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
