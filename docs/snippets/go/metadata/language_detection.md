```go title="Go"
package main

import (
	"log"

	"github.com/xberg-io/xberg/packages/go"
)

func main() {
	enabled := true
	minConfidence := 0.9
	result, err := xberg.ExtractSync("document.pdf", nil, xberg.ExtractionConfig{
		LanguageDetection: &xberg.LanguageDetectionConfig{
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
