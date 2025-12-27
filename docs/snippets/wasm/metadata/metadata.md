```typescript
import { extractFromFile, initWasm } from '@kreuzberg/wasm';

await initWasm();

const fileInput = document.getElementById('file') as HTMLInputElement;
const file = fileInput.files?.[0];

if (file) {
	const result = await extractFromFile(file);
	console.log(`Metadata: ${JSON.stringify(result.metadata)}`);

	// Access common metadata fields
	if (result.metadata.title) {
		console.log(`Title: ${result.metadata.title}`);
	}

	// Access format-specific metadata
	const metadata = result.metadata;

	// For HTML files
	if (metadata.html) {
		const htmlMeta = metadata.html;
		console.log(`HTML Title: ${htmlMeta.title}`);
		console.log(`Description: ${htmlMeta.description}`);

		// Access keywords as array
		if (htmlMeta.keywords && htmlMeta.keywords.length > 0) {
			console.log(`Keywords: ${htmlMeta.keywords.join(', ')}`);
		}

		// Access canonical URL
		if (htmlMeta.canonical_url) {
			console.log(`Canonical URL: ${htmlMeta.canonical_url}`);
		}

		// Access Open Graph fields
		if (htmlMeta.open_graph) {
			if (htmlMeta.open_graph['title']) {
				console.log(`OG Title: ${htmlMeta.open_graph['title']}`);
			}
			if (htmlMeta.open_graph['image']) {
				console.log(`OG Image: ${htmlMeta.open_graph['image']}`);
			}
		}

		// Access Twitter Card fields
		if (htmlMeta.twitter_card && htmlMeta.twitter_card['card']) {
			console.log(`Twitter Card Type: ${htmlMeta.twitter_card['card']}`);
		}

		// Access headers
		if (htmlMeta.headers && htmlMeta.headers.length > 0) {
			console.log(`Headers: ${htmlMeta.headers.map((h: any) => h.text).join(', ')}`);
		}

		// Access links
		if (htmlMeta.links && htmlMeta.links.length > 0) {
			htmlMeta.links.forEach((link: any) => {
				console.log(`Link: ${link.href} (${link.text})`);
			});
		}

		// Access images
		if (htmlMeta.images && htmlMeta.images.length > 0) {
			htmlMeta.images.forEach((image: any) => {
				console.log(`Image: ${image.src}`);
			});
		}

		// Access structured data
		if (htmlMeta.structured_data && htmlMeta.structured_data.length > 0) {
			console.log(`Structured data items: ${htmlMeta.structured_data.length}`);
		}
	}

	// For PDF files
	if (metadata.pdf) {
		const pdfMeta = metadata.pdf;
		if (pdfMeta.page_count) {
			console.log(`Pages: ${pdfMeta.page_count}`);
		}
		if (pdfMeta.author) {
			console.log(`Author: ${pdfMeta.author}`);
		}
	}
}
```
