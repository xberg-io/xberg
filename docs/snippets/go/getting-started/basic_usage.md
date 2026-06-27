```go title="Go"
package main

import (
	"fmt"
	"log"

	"github.com/xberg-io/xberg/packages/go"
)

func main() {
	result, err := xberg.ExtractSync("document.pdf", nil)
	if err != nil {
		log.Fatalf("extract failed: %v", err)
	}

	fmt.Println("Content:")
	fmt.Println(result.Content)

	fmt.Println("\nMetadata:")
	if result.Metadata != nil {
		fmt.Printf("Title: %v\n", result.Metadata["title"])
		fmt.Printf("Author: %v\n", result.Metadata["author"])
	}

	fmt.Printf("\nTables found: %d\n", len(result.Tables))
	fmt.Printf("Images found: %d\n", len(result.Images))
}
```
