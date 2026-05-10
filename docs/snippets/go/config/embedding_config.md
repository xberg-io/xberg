```go title="Go"
package main

import (
	"log"

	"github.com/kreuzberg-dev/kreuzberg/packages/go/v5"
)

func main() {
	maxChars := uint(1000)
	batchSize := uint(16)
	normalize := true
	modelName := "all-mpnet-base-v2"

	cfg := kreuzberg.ExtractionConfig{
		Chunking: &kreuzberg.ChunkingConfig{
			MaxCharacters: &maxChars,
			Embedding: &kreuzberg.EmbeddingConfig{
				Model: kreuzberg.EmbeddingModelType{
					Type: "preset",
					Name: &modelName,
				},
				BatchSize:            &batchSize,
				Normalize:            &normalize,
				ShowDownloadProgress: true,
			},
		},
	}

	result, err := kreuzberg.ExtractFileSync("document.pdf", nil, cfg)
	if err != nil {
		log.Fatalf("extract failed: %v", err)
	}
	log.Println("content length:", len(result.Content))
}
```
