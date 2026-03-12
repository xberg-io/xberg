```go title="Go"
package main

import (
	"fmt"

	"github.com/kreuzberg-dev/kreuzberg/packages/go/v4"
)

func main() {
	maxChars := 1000
	maxOverlap := 200
	config := &kreuzberg.ExtractionConfig{
		Chunking: &kreuzberg.ChunkingConfig{
			MaxChars:   &maxChars,
			MaxOverlap: &maxOverlap,
		},
	}

	fmt.Printf("Config: MaxChars=%d, MaxOverlap=%d\n", *config.Chunking.MaxChars, *config.Chunking.MaxOverlap)
}
```

```go title="Go - Markdown with Heading Context"
package main

import (
	"fmt"

	"github.com/kreuzberg-dev/kreuzberg/packages/go/v4"
)

func main() {
	maxChars := 500
	maxOverlap := 50

	config := &kreuzberg.ExtractionConfig{
		Chunking: &kreuzberg.ChunkingConfig{
			MaxChars:   &maxChars,
			MaxOverlap: &maxOverlap,
			Sizing: &kreuzberg.ChunkSizingConfig{
				Type:  "tokenizer",
				Model: "Xenova/gpt-4o",
			},
		},
	}

	result, err := kreuzberg.ExtractFile("document.md", nil, config)
	if err != nil {
		panic(err)
	}

	for _, chunk := range result.Chunks {
		if chunk.Metadata != nil && chunk.Metadata.HeadingContext != nil {
			for _, heading := range chunk.Metadata.HeadingContext.Headings {
				fmt.Printf("Heading L%d: %s\n", heading.Level, heading.Text)
			}
		}
		fmt.Printf("Content: %.100s...\n", chunk.Content)
	}
}
```
