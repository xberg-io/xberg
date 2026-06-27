```go title="Go"
package main

import (
	"fmt"
	"log"

	"github.com/xberg-io/xberg/packages/go"
)

func main() {
	result, err := xberg.Extract("document.pdf", nil, xberg.ExtractionConfig{
		Ocr: &xberg.OcrConfig{
			Backend: "tesseract",
		},
		ForceOcr: true,
	})
	if err != nil {
		log.Fatalf("extract failed: %v", err)
	}

	fmt.Println(result.Content)
}
```
