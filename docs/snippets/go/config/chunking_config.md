```go title="Go"
package main

import (
	"fmt"

	"github.com/kreuzberg-dev/kreuzberg/packages/go/v5"
)

func main() {
	maxChars := uint(1000)
	overlap := uint(200)
	config := kreuzberg.ExtractionConfig{
		Chunking: &kreuzberg.ChunkingConfig{
			MaxCharacters: &maxChars,
			Overlap:       &overlap,
		},
	}

	fmt.Printf("Config: MaxCharacters=%d, Overlap=%d\n",
		*config.Chunking.MaxCharacters, *config.Chunking.Overlap)
}
```

```go title="Go - Markdown with Heading Context"
package main

import (
	"fmt"
	"log"

	"github.com/kreuzberg-dev/kreuzberg/packages/go/v5"
)

func main() {
	maxChars := uint(500)
	overlap := uint(50)
	model := "Xenova/gpt-4o"
	chunkerType := kreuzberg.ChunkerTypeMarkdown

	config := kreuzberg.ExtractionConfig{
		Chunking: &kreuzberg.ChunkingConfig{
			MaxCharacters: &maxChars,
			Overlap:       &overlap,
			ChunkerType:   &chunkerType,
			Sizing: kreuzberg.ChunkSizing{
				Type:  "tokenizer",
				Model: &model,
			},
		},
	}

	result, err := kreuzberg.ExtractFile("document.md", nil, config)
	if err != nil {
		log.Fatalf("extract failed: %v", err)
	}

	for _, chunk := range result.Chunks {
		if chunk.Metadata.HeadingContext != nil {
			for _, heading := range chunk.Metadata.HeadingContext.Headings {
				fmt.Printf("Heading L%d: %s\n", heading.Level, heading.Text)
			}
		}
		fmt.Printf("Content: %.100s...\n", chunk.Content)
	}
}
```

```go title="Go - Prepend Heading Context"
package main

import (
	"fmt"
	"log"

	"github.com/kreuzberg-dev/kreuzberg/packages/go/v5"
)

func main() {
	maxChars := uint(500)
	overlap := uint(50)
	chunkerType := kreuzberg.ChunkerTypeMarkdown

	config := kreuzberg.ExtractionConfig{
		Chunking: &kreuzberg.ChunkingConfig{
			MaxCharacters:         &maxChars,
			Overlap:               &overlap,
			ChunkerType:           &chunkerType,
			PrependHeadingContext: true,
		},
	}

	result, err := kreuzberg.ExtractFile("document.md", nil, config)
	if err != nil {
		log.Fatalf("extract failed: %v", err)
	}

	for _, chunk := range result.Chunks {
		// Each chunk's content is prefixed with its heading breadcrumb
		fmt.Printf("Content: %.100s...\n", chunk.Content)
	}
}
```
