```go title="Go"
package main

import (
	"fmt"
	"log"

	"github.com/xberg-io/xberg/packages/go"
)

func main() {
	fmt.Println("Xberg CGO bindings loaded successfully")

	result, err := xberg.ExtractSync("sample.pdf", nil)
	if err != nil {
		log.Fatalf("extract failed: %v", err)
	}

	fmt.Println("Installation verified!")
	fmt.Printf("Extracted %d characters\n", len(result.Content))
}
```
