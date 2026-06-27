```go title="Element-Based Output (Go)"
package main

import (
    "fmt"
    "xberg"
)

func main() {
    // Configure element-based output
    config := &xberg.ExtractionConfig{
        OutputFormat: "element_based",
    }

    // Extract document
    result, err := xberg.ExtractSync("document.pdf", config)
    if err != nil {
        panic(err)
    }

    // Access elements
    for _, element := range result.Elements {
        fmt.Printf("Type: %s\n", element.ElementType)

        text := element.Text
        if len(text) > 100 {
            text = text[:100]
        }
        fmt.Printf("Text: %s\n", text)

        if element.Metadata.PageNumber != nil {
            fmt.Printf("Page: %d\n", *element.Metadata.PageNumber)
        }

        if element.Metadata.Coordinates != nil {
            coords := element.Metadata.Coordinates
            fmt.Printf("Coords: (%f, %f) - (%f, %f)\n",
                coords.Left, coords.Top, coords.Right, coords.Bottom)
        }

        fmt.Println("---")
    }

    // Filter by element type
    var titles []xberg.Element
    for _, element := range result.Elements {
        if element.ElementType == "title" {
            titles = append(titles, element)
        }
    }

    for _, title := range titles {
        level, ok := title.Metadata.Additional["level"].(string)
        if !ok {
            level = "unknown"
        }
        fmt.Printf("[%s] %s\n", level, title.Text)
    }
}
```
