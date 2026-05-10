```go title="Go"
package main

import (
	"fmt"
	"log"

	"github.com/kreuzberg-dev/kreuzberg/packages/go/v5"
)

func main() {
	outputFormat := kreuzberg.OutputFormatHTML
	theme := kreuzberg.HTMLThemeGitHub
	embedCSS := true

	config := &kreuzberg.ExtractionConfig{
		OutputFormat: &outputFormat,
		HTMLOutput: &kreuzberg.HTMLOutputConfig{
			Theme:    &theme,
			EmbedCSS: &embedCSS,
		},
	}

	result, err := kreuzberg.ExtractFileSync("document.pdf", config)
	if err != nil {
		log.Fatalf("extract failed: %v", err)
	}

	fmt.Println(result.Content) // HTML with kb-* classes
}
```
