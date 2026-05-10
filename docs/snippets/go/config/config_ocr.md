```go title="Go"
package main

import "github.com/kreuzberg-dev/kreuzberg/packages/go/v5"

func main() {
	psm := int32(3)

	_ = kreuzberg.ExtractionConfig{
		Ocr: &kreuzberg.OcrConfig{
			Backend:  "tesseract",
			Language: "eng+fra",
			TesseractConfig: &kreuzberg.TesseractConfig{
				Psm: &psm,
			},
		},
	}
}
```
