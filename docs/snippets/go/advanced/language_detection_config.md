```go title="Go"
package main

import (
	"github.com/kreuzberg-dev/kreuzberg/packages/go/v5"
)

func main() {
	enabled := true
	detectMultiple := false
	minConfidence := 0.8

	config := &kreuzberg.ExtractionConfig{
		LanguageDetection: &kreuzberg.LanguageDetectionConfig{
			Enabled:        &enabled,
			MinConfidence:  &minConfidence,
			DetectMultiple: &detectMultiple,
		},
	}
	_ = config
}
```
