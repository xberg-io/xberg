```go title="Go"
package main

import (
	"errors"
	"log"

	"github.com/xberg-io/xberg"
)

func main() {
	result, err := xberg.ExtractSync("missing.pdf", nil, xberg.ExtractionConfig{})
	if err != nil {
		if errors.Is(err, xberg.ErrIo) {
			log.Printf("file not found: %v", err)
		} else if errors.Is(err, xberg.ErrUnsupportedFormat) {
			log.Printf("unsupported format: %v", err)
		} else {
			log.Printf("extraction error: %v", err)
		}
		return
	}

	println("Content:", result.Content)
}
```
