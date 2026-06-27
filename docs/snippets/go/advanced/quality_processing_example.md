```go title="Go"
package main

import (
	"fmt"
	"log"

	"github.com/xberg-io/xberg/packages/go"
)

func main() {
	enableQualityProcessing := true

	config := &xberg.ExtractionConfig{
		EnableQualityProcessing: &enableQualityProcessing,
	}

	result, err := xberg.ExtractSync("scanned_document.pdf", config)
	if err != nil {
		log.Fatalf("extraction failed: %v", err)
	}

	qualityScore := 0.0
	if result.QualityScore != nil {
		qualityScore = *result.QualityScore
	}

	if qualityScore < 0.5 {
		fmt.Printf("Warning: Low quality extraction (%.2f)\n", qualityScore)
		fmt.Println("Consider re-scanning with higher DPI or adjusting OCR settings")
	} else {
		fmt.Printf("Quality score: %.2f\n", qualityScore)
	}
}
```
