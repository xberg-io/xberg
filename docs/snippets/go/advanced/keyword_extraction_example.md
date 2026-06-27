```go title="Go"
package main

import (
	"fmt"
	"log"

	"github.com/xberg-io/xberg/packages/go"
)

func main() {
	maxKeywords := int32(10)
	minScore := 0.3

	config := &xberg.ExtractionConfig{
		Keywords: &xberg.KeywordConfig{
			Algorithm:   xberg.KeywordAlgorithm_YAKE,
			MaxKeywords: &maxKeywords,
			MinScore:    &minScore,
		},
	}

	result, err := xberg.ExtractSync("research_paper.pdf", config)
	if err != nil {
		log.Fatalf("extraction failed: %v", err)
	}

	if keywords, ok := result.Metadata["keywords"]; ok {
		keywordList := keywords.([]map[string]interface{})
		for _, kw := range keywordList {
			text := kw["text"].(string)
			score := kw["score"].(float64)
			fmt.Printf("%s: %.3f\n", text, score)
		}
	}
}
```
