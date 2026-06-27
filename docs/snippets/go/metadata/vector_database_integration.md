```go title="Go"
package main

import (
	"fmt"
	"log"

	"github.com/xberg-io/xberg/packages/go"
)

func main() {
	maxChars := 512
	maxOverlap := 50
	config := &xberg.ExtractionConfig{
		Chunking: &xberg.ChunkingConfig{
			MaxChars:   &maxChars,
			MaxOverlap: &maxOverlap,
			Embedding: &xberg.EmbeddingConfig{
				Model:     "balanced",
				Normalize: true,
			},
		},
	}

	result, err := xberg.ExtractSync("document.pdf", config)
	if err != nil {
		log.Fatalf("extract failed: %v", err)
	}

	if result.Chunks != nil {
		for i, chunk := range result.Chunks {
			if chunk.Embedding != nil {
				fmt.Printf("Chunk %d: %d dimensions\n", i, len(chunk.Embedding))
				// Store in vector database
			}
		}
	}
}
```
