```go title="Go"
package main

import (
	"log"

	"github.com/kreuzberg-dev/kreuzberg/v5"
)

func main() {
	result, err := kreuzberg.ExtractFileSync("document.pdf", nil, kreuzberg.ExtractionConfig{})
	if err != nil {
		log.Fatalf("extraction failed: %v", err)
	}

	println("Content:", result.Content)
}
```
