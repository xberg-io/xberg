```go title="disk_cache.go"
package main

import (
	"fmt"
	"log"

	"github.com/kreuzberg-dev/kreuzberg/packages/go/v5"
)

func main() {
	useCache := true
	namespace := "documents"
	ttl := uint64(7 * 86400)

	config := kreuzberg.ExtractionConfig{
		UseCache:       &useCache,
		CacheNamespace: &namespace,
		CacheTTLSecs:   &ttl,
	}

	fmt.Println("First extraction (will be cached)...")
	result1, err := kreuzberg.ExtractFileSync("document.pdf", nil, config)
	if err != nil {
		log.Fatalf("extract failed: %v", err)
	}
	fmt.Printf("  - Content length: %d\n", len(result1.Content))

	fmt.Println("\nSecond extraction (from cache)...")
	result2, err := kreuzberg.ExtractFileSync("document.pdf", nil, config)
	if err != nil {
		log.Fatalf("extract failed: %v", err)
	}
	fmt.Printf("  - Content length: %d\n", len(result2.Content))

	fmt.Printf("\nResults are identical: %v\n", result1.Content == result2.Content)
}
```
