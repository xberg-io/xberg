```go title="Go"
package main

import (
	"fmt"
	"log"
	"strings"

	"github.com/kreuzberg-dev/kreuzberg/packages/go/v4"
)

func main() {
	result, err := kreuzberg.ExtractFileSync("document.pdf", nil)
	if err != nil {
		log.Fatalf("extract pdf: %v", err)
	}

	// Access PDF metadata
	if pdf, ok := result.Metadata.PdfMetadata(); ok {
		if pdf.PageCount != nil {
			fmt.Printf("Pages: %d\n", *pdf.PageCount)
		}
		if pdf.Author != nil {
			fmt.Printf("Author: %s\n", *pdf.Author)
		}
		if pdf.Title != nil {
			fmt.Printf("Title: %s\n", *pdf.Title)
		}
	}

	// Access HTML metadata
	htmlResult, err := kreuzberg.ExtractFileSync("page.html", nil)
	if err != nil {
		log.Fatalf("extract html: %v", err)
	}
	if html, ok := htmlResult.Metadata.HTMLMetadata(); ok {
		if html.Title != nil {
			fmt.Printf("Title: %s\n", *html.Title)
		}
		if html.Description != nil {
			fmt.Printf("Description: %s\n", *html.Description)
		}

		// Access keywords as array
		if len(html.Keywords) > 0 {
			fmt.Printf("Keywords: %s\n", strings.Join(html.Keywords, ", "))
		}

		// Access canonical URL (renamed from canonical)
		if html.CanonicalURL != nil {
			fmt.Printf("Canonical URL: %s\n", *html.CanonicalURL)
		}

		// Access Open Graph fields from map
		if len(html.OpenGraph) > 0 {
			if image, ok := html.OpenGraph["image"]; ok {
				fmt.Printf("Open Graph Image: %s\n", image)
			}
			if ogTitle, ok := html.OpenGraph["title"]; ok {
				fmt.Printf("Open Graph Title: %s\n", ogTitle)
			}
			if ogType, ok := html.OpenGraph["type"]; ok {
				fmt.Printf("Open Graph Type: %s\n", ogType)
			}
		}

		// Access Twitter Card fields from map
		if len(html.TwitterCard) > 0 {
			if card, ok := html.TwitterCard["card"]; ok {
				fmt.Printf("Twitter Card Type: %s\n", card)
			}
			if creator, ok := html.TwitterCard["creator"]; ok {
				fmt.Printf("Twitter Creator: %s\n", creator)
			}
		}

		// Access new fields
		if html.Language != nil {
			fmt.Printf("Language: %s\n", *html.Language)
		}

		if html.TextDirection != nil {
			fmt.Printf("Text Direction: %s\n", *html.TextDirection)
		}

		// Access headers
		if len(html.Headers) > 0 {
			fmt.Printf("Headers: %s\n", strings.Join(html.Headers, ", "))
		}

		// Access links
		if len(html.Links) > 0 {
			for _, link := range html.Links {
				fmt.Printf("Link: %s (%s)\n", link.Href, link.Text)
			}
		}

		// Access images
		if len(html.Images) > 0 {
			for _, image := range html.Images {
				fmt.Printf("Image: %s\n", image.Src)
			}
		}

		// Access structured data
		if len(html.StructuredData) > 0 {
			fmt.Printf("Structured data items: %d\n", len(html.StructuredData))
		}
	}
}
```
