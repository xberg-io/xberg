```go title="Go"
package main

import (
	"fmt"

	"github.com/xberg-io/xberg"
)

func main() {
	maxKeywords := uint(10)
	language := "en"

	config := xberg.ExtractionConfig{
		Keywords: &xberg.KeywordConfig{
			Algorithm:   xberg.KeywordAlgorithmYake,
			MaxKeywords: &maxKeywords,
			MinScore:    0.3,
			Language:    &language,
		},
	}

	fmt.Printf("Keywords config: Algorithm=%s, MaxKeywords=%d, MinScore=%f\n",
		config.Keywords.Algorithm,
		*config.Keywords.MaxKeywords,
		config.Keywords.MinScore)
}
```
