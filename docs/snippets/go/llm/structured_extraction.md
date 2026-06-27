```go title="Go"
package main

import (
	"encoding/json"
	"fmt"
	"log"

	"github.com/xberg-io/xberg/packages/go"
)

func main() {
	schema, err := json.Marshal(map[string]any{
		"type": "object",
		"properties": map[string]any{
			"title":   map[string]string{"type": "string"},
			"authors": map[string]any{"type": "array", "items": map[string]string{"type": "string"}},
			"date":    map[string]string{"type": "string"},
		},
		"required":             []string{"title", "authors", "date"},
		"additionalProperties": false,
	})
	if err != nil {
		log.Fatalf("marshal schema: %v", err)
	}

	config := xberg.ExtractionConfig{
		StructuredExtraction: &xberg.StructuredExtractionConfig{
			Schema:     schema,
			SchemaName: "PaperMetadata",
			Strict:     true,
			Llm: xberg.LlmConfig{
				Model: "openai/gpt-4o-mini",
			},
		},
	}

	result, err := xberg.Extract("paper.pdf", nil, config)
	if err != nil {
		log.Fatalf("extract: %v", err)
	}

	if result.StructuredOutput != nil {
		fmt.Println(string(*result.StructuredOutput))
	}
}
```
