```go title="Go"
package main

import (
	"log"

	"github.com/xberg-io/xberg/packages/go"
)

func main() {
	psm := int32(6)
	oem := int32(1)
	enableTableDetection := true
	whitelist := "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789 .,!?"

	config := xberg.ExtractionConfig{
		Ocr: &xberg.OcrConfig{
			Backend:  "tesseract",
			Language: "eng+fra+deu",
			TesseractConfig: &xberg.TesseractConfig{
				Psm:                   &psm,
				Oem:                   &oem,
				MinConfidence:         0.8,
				EnableTableDetection:  &enableTableDetection,
				TesseditCharWhitelist: whitelist,
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
