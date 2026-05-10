```go title="Go"
package main

import (
	"bytes"
	"io"
	"log"
	"mime/multipart"
	"net/http"
	"os"
)

func main() {
	file, err := os.Open("document.pdf")
	if err != nil {
		log.Fatalf("failed to open file: %v", err)
	}
	defer file.Close()

	body := &bytes.Buffer{}
	writer := multipart.NewWriter(body)
	part, _ := writer.CreateFormFile("files", "document.pdf")
	io.Copy(part, file)
	writer.Close()

	resp, err := http.Post("http://localhost:8000/extract", writer.FormDataContentType(), body)
	if err != nil {
		log.Fatalf("request failed: %v", err)
	}
	defer resp.Body.Close()

	io.Copy(os.Stdout, resp.Body)
}
```
