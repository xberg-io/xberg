```go title="Go"
package main

import (
	"log"

	"github.com/kreuzberg-dev/kreuzberg/v5"
)

func main() {
	items := []kreuzberg.BatchFileItem{
		{Path: "doc1.pdf"},
		{Path: "doc2.docx"},
		{Path: "doc3.pptx"},
	}

	results, err := kreuzberg.BatchExtractFilesSync(items, kreuzberg.ExtractionConfig{})
	if err != nil {
		log.Fatalf("batch extraction failed: %v", err)
	}

	for i, result := range results {
		println("Doc", i, "content length:", len(result.Content))
	}
}
```
