```go title="Go"
package main

import (
	"fmt"
	"log"
	"strings"

	"github.com/xberg-io/xberg/packages/go"
)

func main() {
	enabled := true
	detectMultiple := true
	minConfidence := 0.8

	config := &xberg.ExtractionConfig{
		LanguageDetection: &xberg.LanguageDetectionConfig{
			Enabled:        &enabled,
			MinConfidence:  &minConfidence,
			DetectMultiple: &detectMultiple,
		},
	}

	result, err := xberg.ExtractSync("multilingual_document.pdf", config)
	if err != nil {
		log.Fatalf("Processing failed: %v", err)
	}

	languages := result.DetectedLanguages
	if len(languages) > 0 {
		fmt.Printf("Detected %d language(s): %s\n", len(languages), strings.Join(languages, ", "))
	} else {
		fmt.Println("No languages detected")
	}

	fmt.Printf("Total content: %d characters\n", len(result.Content))
	fmt.Printf("MIME type: %s\n", result.MimeType)
}
```
