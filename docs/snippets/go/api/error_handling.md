```go title="Go"
package main

import (
	"errors"
	"log"

	"github.com/kreuzberg-dev/kreuzberg/v5"
)

func main() {
	result, err := kreuzberg.ExtractFileSync("missing.pdf", nil, kreuzberg.ExtractionConfig{})
	if err != nil {
		if errors.Is(err, kreuzberg.ErrIo) {
			log.Printf("file not found: %v", err)
		} else if errors.Is(err, kreuzberg.ErrUnsupportedFormat) {
			log.Printf("unsupported format: %v", err)
		} else {
			log.Printf("extraction error: %v", err)
		}
		return
	}

	println("Content:", result.Content)
}
```
