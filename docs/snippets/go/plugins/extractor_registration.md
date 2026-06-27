```go title="Go"
package main

import (
	"log"

	"github.com/xberg-io/xberg/packages/go"
)

func main() {
	// Register custom extractor with priority 50
	if err := xberg.RegisterDocumentExtractor("custom-json-extractor", 50); err != nil {
		log.Fatalf("register extractor failed: %v", err)
	}

	result, err := xberg.ExtractSync("document.json", nil)
	if err != nil {
		log.Fatalf("extract failed: %v", err)
	}
	log.Printf("Extracted content length: %d", len(result.Content))
}
```
