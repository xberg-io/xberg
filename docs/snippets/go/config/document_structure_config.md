```go title="Document Structure Config (Go)"
package main

import (
    "fmt"
    xberg "github.com/xberg-io/xberg/packages/go"
)

func main() {
    config := xberg.NewExtractionConfig(
        xberg.WithIncludeDocumentStructure(true),
    )

    result, err := xberg.ExtractSync("document.pdf", config)
    if err != nil {
        panic(err)
    }

    if result.Document != nil {
        for _, node := range result.Document.Nodes {
            fmt.Printf("[%s]\n", node.Content.NodeType)
        }
    }
}
```
