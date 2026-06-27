```go title="Go"
package main

import (
	"log"

	"github.com/xberg-io/xberg/packages/go"
)

func main() {
	useCache := true
	enableQP := true

	result, err := xberg.ExtractSync("document.pdf", &xberg.ExtractionConfig{
		UseCache:                &useCache,
		EnableQualityProcessing: &enableQP,
	})
	if err != nil {
		log.Fatalf("extract failed: %v", err)
	}

	log.Println("content length:", len(result.Content))
}
```
