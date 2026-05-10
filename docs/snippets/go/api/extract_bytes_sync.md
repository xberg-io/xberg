```go title="Go"
package main

import (
	"log"
	"os"

	"github.com/kreuzberg-dev/kreuzberg/v5"
)

func main() {
	content, err := os.ReadFile("document.pdf")
	if err != nil {
		log.Fatalf("failed to read file: %v", err)
	}

	result, err := kreuzberg.ExtractBytesSync(content, "application/pdf", kreuzberg.ExtractionConfig{})
	if err != nil {
		log.Fatalf("extraction failed: %v", err)
	}

	println("Content:", result.Content)
}
```
