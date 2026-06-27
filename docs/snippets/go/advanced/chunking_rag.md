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
	normalize := true
	batchSize := int32(16)

	config := &xberg.ExtractionConfig{
		Chunking: &xberg.ChunkingConfig{
			MaxChars:   &maxChars,
			MaxOverlap: &maxOverlap,
			Embedding: &xberg.EmbeddingConfig{
				Model:     xberg.EmbeddingModelType_Preset("all-mpnet-base-v2"),
				Normalize: &normalize,
				BatchSize: &batchSize,
			},
		},
	}

	result, err := xberg.ExtractSync("research_paper.pdf", config)
	if err != nil {
		log.Fatalf("RAG extraction failed: %v", err)
	}

	chunks := result.Chunks
	fmt.Printf("Found %d chunks for RAG pipeline\n", len(chunks))

	for i := 0; i < len(chunks) && i < 3; i++ {
		chunk := chunks[i]
		content := chunk.Content
		if len(content) > 80 {
			content = content[:80]
		}
		fmt.Printf("Chunk %d: %s...\n", i, content)
	}
}
```
