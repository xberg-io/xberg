```go title="Go"
package main

import (
	"fmt"
	"log"

	"github.com/xberg-io/xberg"
)

func main() {
	maxChars := uint(500)
	overlap := uint(50)
	config := &xberg.ExtractionConfig{
		Chunking: &xberg.ChunkingConfig{
			MaxCharacters: &maxChars,
			Overlap:       &overlap,
		},
	}

	result, err := xberg.ExtractSync("document.pdf", config)
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
