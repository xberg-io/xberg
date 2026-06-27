```go title="Go"
package main

import (
	"fmt"
	"log"

	"github.com/xberg-io/xberg/packages/go"
)

func main() {
	outputFormat := xberg.OutputFormatHTML
	theme := xberg.HTMLThemeGitHub
	embedCSS := true

	config := &xberg.ExtractionConfig{
		OutputFormat: &outputFormat,
		HTMLOutput: &xberg.HTMLOutputConfig{
			Theme:    &theme,
			EmbedCSS: &embedCSS,
		},
	}

	result, err := xberg.ExtractSync("document.pdf", config)
	if err != nil {
		log.Fatalf("extract failed: %v", err)
	}

	fmt.Println(result.Content) // HTML with kb-* classes
}
```
