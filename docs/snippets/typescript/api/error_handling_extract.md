```typescript title="TypeScript"
import { readFileSync } from "node:fs";

async function extractDocument(): Promise<void> {
  const formData = new FormData();
  const fileData = readFileSync("document.pdf");
  formData.append("files", new Blob([fileData]), "document.pdf");

  try {
    const response = await fetch("http://localhost:8000/extract", {
      method: "POST",
      body: formData,
    });

    if (!response.ok) {
      const error = await response.json();
      console.error(`Error: ${error.error_type}: ${error.message}`);
      return;
    }

    const results = await response.json();
    console.log(`Extracted ${results.length} documents`);
  } catch (error: unknown) {
    if (error instanceof Error) {
      console.error(`Request failed: ${error.message}`);
    }
  }
}

extractDocument();
```
