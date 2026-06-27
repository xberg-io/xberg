```go title="Go"
package main

import (
	"log"

	"github.com/xberg-io/xberg/packages/go"
)

func main() {
	extractMetadata := true
	result, err := xberg.ExtractSync("document.pdf", nil, xberg.ExtractionConfig{
		PdfOptions: &xberg.PdfConfig{
			ExtractImages:   true,
			ExtractMetadata: &extractMetadata,
			Passwords:       []string{"password1", "password2"},
			Hierarchy:       &xberg.HierarchyConfig{},
		},
	})
	if err != nil {
		log.Fatalf("extract failed: %v", err)
	}

	log.Println("content length:", len(result.Content))
}
```
