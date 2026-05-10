```go title="Go"
package main

import (
	"log"

	"github.com/kreuzberg-dev/kreuzberg/packages/go/v5"
)

func main() {
	enabled := true
	minConfidence := 0.9
	result, err := kreuzberg.ExtractFileSync("document.pdf", nil, kreuzberg.ExtractionConfig{
		LanguageDetection: &kreuzberg.LanguageDetectionConfig{
			Enabled:        &enabled,
			MinConfidence:  &minConfidence,
			DetectMultiple: true,
		},
	})
	if err != nil {
		log.Fatalf("extract failed: %v", err)
	}

	log.Println("content length:", len(result.Content))
}
```
