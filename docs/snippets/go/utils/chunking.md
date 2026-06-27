```go title="Go"
package main

import (
	"fmt"
	"log"

	"github.com/xberg-io/xberg/packages/go"
)

func main() {
	maxChars := 1000
	maxOverlap := 200
	config := &xberg.ExtractionConfig{
		Chunking: &xberg.ChunkingConfig{
			MaxChars:   &maxChars,
			MaxOverlap: &maxOverlap,
		},
	}

	result, err := xberg.ExtractSync("document.pdf", config)
	if err != nil {
		log.Fatalf("extract failed: %v", err)
	}

	for i, chunk := range result.Chunks {
		fmt.Printf("Chunk %d/%d (%d-%d)\n", i+1, chunk.Metadata.TotalChunks, chunk.Metadata.CharStart, chunk.Metadata.CharEnd)
		fmt.Printf("%s...\n", chunk.Content[:min(len(chunk.Content), 100)])
	}
}

func min(a, b int) int {
	if a < b {
		return a
	}
	return b
}
```
