```go title="Go"
package main

import (
	"log"

	"github.com/kreuzberg-dev/kreuzberg/packages/go/v5"
)

func main() {
	targetDpi := int32(300)
	deskew := true
	binarization := "otsu"

	config := kreuzberg.ExtractionConfig{
		Ocr: &kreuzberg.OcrConfig{
			TesseractConfig: &kreuzberg.TesseractConfig{
				Preprocessing: &kreuzberg.ImagePreprocessingConfig{
					TargetDpi:          &targetDpi,
					Denoise:            true,
					Deskew:             &deskew,
					ContrastEnhance:    true,
					BinarizationMethod: &binarization,
				},
			},
		},
	}

	result, err := kreuzberg.ExtractFileSync("document.pdf", nil, config)
	if err != nil {
		log.Fatalf("extract failed: %v", err)
	}

	log.Println("content length:", len(result.Content))
}
```
