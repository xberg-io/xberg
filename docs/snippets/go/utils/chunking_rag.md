```go title="Go"
package main

import (
	"fmt"
	"log"

	"github.com/xberg-io/xberg/packages/go"
)

func main() {
	maxChars := 500
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

	result, err := xberg.ExtractSync("research_paper.pdf", config)
	if err != nil {
		log.Fatalf("extract failed: %v", err)
	}

	for i, chunk := range result.Chunks {
		fmt.Printf("Chunk %d/%d (%d-%d)\n", i+1, chunk.Metadata.TotalChunks, chunk.Metadata.CharStart, chunk.Metadata.CharEnd)
		fmt.Printf("Content: %s...\n", chunk.Content[:min(len(chunk.Content), 100)])
		if chunk.Embedding != nil {
			fmt.Printf("Embedding: %d dimensions\n", len(chunk.Embedding))
		}
	}
}

func min(a, b int) int {
	if a < b {
		return a
	}
	return b
}
```
