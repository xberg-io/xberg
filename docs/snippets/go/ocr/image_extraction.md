```go title="Go"
package main

import (
	"log"

	"github.com/kreuzberg-dev/kreuzberg/packages/go/v5"
)

func main() {
	extractImages := true
	injectPlaceholders := true
	autoAdjustDpi := true
	targetDpi := int32(200)
	maxDim := int32(2048)

	result, err := kreuzberg.ExtractFileSync("document.pdf", nil, kreuzberg.ExtractionConfig{
		Images: &kreuzberg.ImageExtractionConfig{
			ExtractImages:      &extractImages,
			TargetDpi:          &targetDpi,
			MaxImageDimension:  &maxDim,
			InjectPlaceholders: &injectPlaceholders, // set to false to extract images without markdown references
			AutoAdjustDpi:      &autoAdjustDpi,
		},
	})
	if err != nil {
		log.Fatalf("extract failed: %v", err)
	}

	log.Println("content length:", len(result.Content))
}
```
