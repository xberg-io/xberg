```go title="Go"
package main

import (
	"fmt"
	"log"

	"github.com/xberg-io/xberg"
)

func main() {
	maxKeywords := uint(10)
	minScore := float32(0.3)
	kind := xberg.ExtractInputKindURI
	uri := "research_paper.pdf"

	config := xberg.ExtractionConfig{
		Keywords: &xberg.KeywordConfig{
			Algorithm:   xberg.KeywordAlgorithmYake,
			MaxKeywords: &maxKeywords,
			MinScore:    minScore,
		},
	}

	output, err := xberg.Extract(
		xberg.ExtractInput{Kind: &kind, URI: &uri},
		config,
	)
	if err != nil {
		log.Fatalf("extract failed: %v", err)
	}

	fmt.Printf("Keywords: %v\n", output.Results[0].ExtractedKeywords)
}
```
