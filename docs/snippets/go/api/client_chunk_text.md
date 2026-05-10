```go title="Go"
package main

import (
	"bytes"
	"encoding/json"
	"io"
	"log"
	"net/http"
)

func main() {
	client := &http.Client{}

	payload := map[string]interface{}{
		"text":          "Your long text content here...",
		"chunker_type":  "text",
		"config": map[string]interface{}{
			"max_characters": 1000,
			"overlap":        50,
			"trim":           true,
		},
	}

	data, _ := json.Marshal(payload)
	resp, err := client.Post("http://localhost:8000/chunk", "application/json", bytes.NewBuffer(data))
	if err != nil {
		log.Fatalf("request failed: %v", err)
	}
	defer resp.Body.Close()

	var result map[string]interface{}
	json.NewDecoder(resp.Body).Decode(&result)

	chunks := result["chunks"].([]interface{})
	log.Printf("Created %d chunks", len(chunks))
	for _, chunk := range chunks {
		c := chunk.(map[string]interface{})
		println("Chunk content:", c["content"].(string))
	}
}
```
