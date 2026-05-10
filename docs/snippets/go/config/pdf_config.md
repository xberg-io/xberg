```go title="Go"
package main

import (
	"log"

	"github.com/kreuzberg-dev/kreuzberg/packages/go/v5"
)

func main() {
	extractMetadata := true
	result, err := kreuzberg.ExtractFileSync("document.pdf", nil, kreuzberg.ExtractionConfig{
		PdfOptions: &kreuzberg.PdfConfig{
			ExtractImages:   true,
			ExtractMetadata: &extractMetadata,
			Passwords:       []string{"password1", "password2"},
			Hierarchy:       &kreuzberg.HierarchyConfig{},
		},
	})
	if err != nil {
		log.Fatalf("extract failed: %v", err)
	}

	log.Println("content length:", len(result.Content))
}
```
