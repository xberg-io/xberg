```typescript title="WASM"
// HTTP client approach for file extraction via multipart upload
// Use this when uploading files from a browser form

const fileInput = document.getElementById("file") as HTMLInputElement;
const file = fileInput.files?.[0];

if (file) {
  const formData = new FormData();
  formData.append("file", file);
  formData.append("mime_type", file.type || "application/octet-stream");

  const response = await fetch("http://localhost:8000/extract/file", {
    method: "POST",
    body: formData
  });

  const result = await response.json();
  console.log(`Extracted ${result.content.length} characters`);
  console.log(`Title: ${result.metadata?.title ?? "Unknown"}`);
}
```
