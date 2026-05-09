```go title="Go"
package main

import (
	"fmt"
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
