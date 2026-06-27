```go title="Go"
package main

import (
	"log"

	"github.com/xberg-io/xberg/packages/go"
)

func main() {
	targetDpi := int32(300)
	deskew := true
	binarization := "otsu"

	config := xberg.ExtractionConfig{
		Ocr: &xberg.OcrConfig{
			TesseractConfig: &xberg.TesseractConfig{
				Preprocessing: &xberg.ImagePreprocessingConfig{
					TargetDpi:          &targetDpi,
					Denoise:            true,
					Deskew:             &deskew,
					ContrastEnhance:    true,
					BinarizationMethod: &binarization,
				},
			},
		},
	}

	result, err := xberg.ExtractSync("document.pdf", nil, config)
	if err != nil {
		log.Fatalf("extract failed: %v", err)
	}

	log.Println("content length:", len(result.Content))
}
```
