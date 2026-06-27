```go title="Go"
package main

import (
	"fmt"
	"log"

	"github.com/xberg-io/xberg/packages/go"
)

func main() {
	config := &xberg.ExtractionConfig{
		TokenReduction: &xberg.TokenReductionConfig{
			Mode:             "moderate",
			PreserveMarkdown: true,
		},
	}

	result, err := xberg.ExtractSync("verbose_document.pdf", config)
	if err != nil {
		log.Fatalf("extract failed: %v", err)
	}

	fmt.Printf("Original tokens: %v\n", result.Metadata.Additional["original_token_count"])
	fmt.Printf("Reduced tokens: %v\n", result.Metadata.Additional["token_count"])
	fmt.Printf("Reduction ratio: %v\n", result.Metadata.Additional["token_reduction_ratio"])
}
```
