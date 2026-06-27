```go title="Go"
package main

import (
	"github.com/xberg-io/xberg"
)

func main() {
	maxKeywords := uint(10)
	minScore := float32(0.3)
	language := "en"

	config := xberg.ExtractionConfig{
		Keywords: &xberg.KeywordConfig{
			Algorithm:   xberg.KeywordAlgorithmYake,
			MaxKeywords: &maxKeywords,
			MinScore:    minScore,
			Language:    &language,
		},
	}
	_ = config
}
```
