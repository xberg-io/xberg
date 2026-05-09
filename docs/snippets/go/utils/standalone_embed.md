```go title="Go"
package main

import (
	"fmt"

	"github.com/kreuzberg-dev/kreuzberg/packages/go/v5"
)

func main() {
	preset := "balanced"
	normalize := true
	config := kreuzberg.EmbeddingConfig{
		Model: kreuzberg.EmbeddingModelType{
			Type: "preset",
			Name: &preset,
		},
		Normalize: &normalize,
	}

	// Synchronous
	embeddings, err := kreuzberg.EmbedTexts([]string{"Hello, world!", "Kreuzberg is fast"}, config)
	if err != nil {
		panic(err)
	}
	fmt.Println(len(embeddings))    // 2
	fmt.Println(len(embeddings[0])) // 768

	// Asynchronous
	embeddings, err = kreuzberg.EmbedTextsAsync([]string{"Hello, world!"}, config)
	if err != nil {
		panic(err)
	}
	fmt.Println(len(embeddings[0])) // 768
}
```
