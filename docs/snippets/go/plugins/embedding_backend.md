```go title="Go"
package main

import (
	"log"

	"github.com/kreuzberg-dev/kreuzberg/packages/go/v5"
)

// MyEmbedder wraps an already-loaded embedder so kreuzberg can call back into
// it during chunking and standalone embed requests. Implement the
// kreuzberg.EmbeddingBackend interface.
type MyEmbedder struct{}

func (e *MyEmbedder) Name() string    { return "my-embedder" }
func (e *MyEmbedder) Version() string { return "1.0.0" }
func (e *MyEmbedder) Initialize() error {
	// Optional warm-up; runs once at registration before Dimensions() is cached.
	return nil
}
func (e *MyEmbedder) Shutdown() error { return nil }

// Captured once at registration; the dispatcher uses this for shape validation.
func (e *MyEmbedder) Dimensions() uint { return 768 }

func (e *MyEmbedder) Embed(texts []string) ([][]float32, error) {
	// Delegate to the already-loaded host model.
	out := make([][]float32, len(texts))
	for i := range texts {
		out[i] = make([]float32, 768)
	}
	return out, nil
}

func main() {
	// Register once at startup.
	if err := kreuzberg.RegisterEmbeddingBackend(&MyEmbedder{}); err != nil {
		log.Fatalf("failed to register embedding backend: %v", err)
	}
	defer func() {
		if err := kreuzberg.UnregisterEmbeddingBackend("my-embedder"); err != nil {
			log.Printf("warning: failed to unregister embedding backend: %v", err)
		}
	}()

	maxDuration := uint64(30)
	config := kreuzberg.EmbeddingConfig{
		Model: kreuzberg.EmbeddingModelType{
			Variant: "plugin",
			Type:    "plugin",
			Name:    func() *string { s := "my-embedder"; return &s }(),
		},
		// Optional: bound the wait on a hung backend (default 60s; nil disables).
		MaxEmbedDurationSecs: &maxDuration,
	}

	vectors, err := kreuzberg.EmbedTexts([]string{"Hello, world!", "Second text"}, config)
	if err != nil {
		log.Fatalf("embed failed: %v", err)
	}
	log.Printf("Generated %d vectors", len(vectors))
}
```
