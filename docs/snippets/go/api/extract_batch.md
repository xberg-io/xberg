```go title="Go"
package main

import (
	"fmt"
	"log"

	"github.com/xberg-io/xberg"
)

func main() {
	uriKind := xberg.ExtractInputKindURI
	bytesKind := xberg.ExtractInputKindBytes
	uri := "document.pdf"
	mimeType := "text/plain"
	filename := "note.txt"

	output, err := xberg.ExtractBatch(
		[]xberg.ExtractInput{
			{Kind: &uriKind, URI: &uri},
			{
				Kind:     &bytesKind,
				Bytes:    []byte("Hello from memory"),
				MimeType: &mimeType,
				Filename: &filename,
			},
		},
		xberg.ExtractionConfig{},
	)
	if err != nil {
		log.Fatal(err)
	}

	for _, result := range output.Results {
		fmt.Println(result.Content)
	}
}
```
