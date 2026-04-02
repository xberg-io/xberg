```go title="Go"
package main

import (
	"context"
	"fmt"

	"github.com/kreuzberg-dev/kreuzberg/packages/go/v4"
)

func main() {
	normalize := true
	config := &kreuzberg.EmbeddingConfig{
		Model:     &kreuzberg.EmbeddingModelType{Type_: "preset", Preset: kreuzberg.String("balanced")},
		Normalize: normalize,
	}

	// Synchronous
	embeddings, err := kreuzberg.EmbedTexts([]string{"Hello, world!", "Kreuzberg is fast"}, config)
	if err != nil {
		panic(err)
	}
	fmt.Println(len(embeddings))    // 2
	fmt.Println(len(embeddings[0])) // 768

	// Asynchronous (context-aware)
	embeddings, err = kreuzberg.EmbedTextsAsync(context.Background(), []string{"Hello, world!"}, config)
	if err != nil {
		panic(err)
	}
	fmt.Println(len(embeddings[0])) // 768
}
```
