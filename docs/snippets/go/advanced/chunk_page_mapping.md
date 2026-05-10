```go title="Go"
package main

import (
	"fmt"
	"log"

	"github.com/kreuzberg-dev/kreuzberg/v5"
)

func main() {
	maxChars := uint(500)
	overlap := uint(50)
	config := &kreuzberg.ExtractionConfig{
		Chunking: &kreuzberg.ChunkingConfig{
			MaxCharacters: &maxChars,
			Overlap:       &overlap,
		},
	}

	result, err := kreuzberg.ExtractFileSync("document.pdf", config)
	if err != nil {
		log.Fatal(err)
	}

	for _, chunk := range result.Chunks {
		first := chunk.Metadata.FirstPage
		last := chunk.Metadata.LastPage
		if first == nil {
			continue
		}
		pageRange := fmt.Sprintf("Page %d", *first)
		if last != nil && *first != *last {
			pageRange = fmt.Sprintf("Pages %d-%d", *first, *last)
		}

		preview := chunk.Content
		if len(preview) > 50 {
			preview = preview[:50]
		}
		fmt.Printf("Chunk: %s... (%s)\n", preview, pageRange)
	}
}
```
