```go title="Go"
package main

import (
	"fmt"
	"log"

	"github.com/xberg-io/xberg/packages/go"
)

func main() {
	config := &xberg.ExtractionConfig{
		Keywords: &xberg.KeywordConfig{
			Algorithm:   "YAKE",
			MaxKeywords: 10,
			MinScore:    0.3,
		},
	}

	result, err := xberg.ExtractSync("research_paper.pdf", config)
	if err != nil {
		log.Fatalf("extract failed: %v", err)
	}

	if keywords, ok := result.Metadata.Additional["keywords"]; ok {
		fmt.Printf("Keywords: %v\n", keywords)
	}
}
```
