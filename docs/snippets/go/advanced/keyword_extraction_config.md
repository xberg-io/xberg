```go title="Go"
package main

import (
	"github.com/kreuzberg-dev/kreuzberg/packages/go/v5"
)

func main() {
	maxKeywords := int32(10)
	minScore := 0.3
	language := "en"

	config := &kreuzberg.ExtractionConfig{
		Keywords: &kreuzberg.KeywordConfig{
			Algorithm:   kreuzberg.KeywordAlgorithm_YAKE,
			MaxKeywords: &maxKeywords,
			MinScore:    &minScore,
			Language:    &language,
		},
	}
	_ = config
}
```
