```go title="Go"
package main

import (
	"log"

	"github.com/xberg-io/xberg/packages/go"
)

func main() {
	maxChars := uint(1000)
	batchSize := uint(16)
	normalize := true
	modelName := "all-mpnet-base-v2"

	cfg := xberg.ExtractionConfig{
		Chunking: &xberg.ChunkingConfig{
			MaxCharacters: &maxChars,
			Embedding: &xberg.EmbeddingConfig{
				Model: xberg.EmbeddingModelType{
					Type: "preset",
					Name: &modelName,
				},
				BatchSize:            &batchSize,
				Normalize:            &normalize,
				ShowDownloadProgress: true,
			},
		},
	}

	result, err := xberg.ExtractSync("document.pdf", nil, cfg)
	if err != nil {
		log.Fatalf("extract failed: %v", err)
	}
	log.Println("content length:", len(result.Content))
}
```
