```go title="Go"
package main

import (
	"fmt"
	"log"

	"github.com/xberg-io/xberg"
)

func main() {
	result, err := xberg.ExtractSync("document.pdf", nil)
	if err != nil {
		log.Fatal(err)
	}

	if result.Metadata.Pages == nil || result.Metadata.Pages.Boundaries == nil {
		return
	}

	contentBytes := []byte(result.Content)
	for i, boundary := range result.Metadata.Pages.Boundaries {
		if i >= 3 {
			break
		}
		pageText := string(contentBytes[boundary.ByteStart:boundary.ByteEnd])
		preview := pageText
		if len(preview) > 100 {
			preview = preview[:100]
		}

		fmt.Printf("Page %d:\n", boundary.PageNumber)
		fmt.Printf("  Byte range: %d-%d\n", boundary.ByteStart, boundary.ByteEnd)
		fmt.Printf("  Preview: %s...\n", preview)
	}
}
```
