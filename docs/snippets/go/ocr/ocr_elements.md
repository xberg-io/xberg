```go title="Go"
package main

import (
	"fmt"
	"log"

	"github.com/xberg-io/xberg/packages/go"
)

func main() {
	cfg := xberg.ExtractionConfig{
		Ocr: &xberg.OcrConfig{
			Backend:  "paddle-ocr",
			Language: "en",
		},
	}

	result, err := xberg.ExtractSync("scanned.pdf", nil, cfg)
	if err != nil {
		log.Fatalf("extract failed: %v", err)
	}

	for _, element := range result.OcrElements {
		fmt.Printf("Text: %s\n", element.Text)
		fmt.Printf("Confidence: %.2f\n", element.Confidence.Recognition)
		fmt.Printf("Geometry: %+v\n", element.Geometry)
		if element.Rotation != nil {
			fmt.Printf("Rotation: %.1f°\n", element.Rotation.AngleDegrees)
		}
		fmt.Println()
	}
}
```
