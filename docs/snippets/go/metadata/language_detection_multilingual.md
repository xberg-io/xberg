```go title="Go"
package main

import (
	"fmt"
	"log"

	"github.com/xberg-io/xberg/packages/go"
)

func main() {
	minConfidence := 0.8
	config := &xberg.ExtractionConfig{
		LanguageDetection: &xberg.LanguageDetectionConfig{
			Enabled:        true,
			MinConfidence:  &minConfidence,
			DetectMultiple: true,
		},
	}

	result, err := xberg.ExtractSync("multilingual_document.pdf", config)
	if err != nil {
		log.Fatalf("extract failed: %v", err)
	}

	fmt.Printf("Detected languages: %v\n", result.DetectedLanguages)
	// Output: [eng fra deu]
}
```
