```typescript title="TypeScript"
import { extractFileSync } from '@kreuzberg/node';

const result = extractFileSync('document.pdf');
console.log(`Metadata: ${JSON.stringify(result.metadata)}`);
if (result.metadata.pageCount) {
	console.log(`Pages: ${result.metadata.pageCount}`);
}

const htmlResult = extractFileSync('page.html');
console.log(`HTML Metadata: ${JSON.stringify(htmlResult.metadata)}`);

const htmlMeta = htmlResult.metadata;
if (htmlMeta.title) {
	console.log(`Title: ${htmlMeta.title}`);
}

// Access keywords as array
if (htmlMeta.keywords && htmlMeta.keywords.length > 0) {
	console.log(`Keywords: ${htmlMeta.keywords.join(', ')}`);
}

// Access canonical URL (renamed from canonical)
if (htmlMeta.canonicalUrl) {
	console.log(`Canonical URL: ${htmlMeta.canonicalUrl}`);
}

// Access Open Graph fields from map
if (htmlMeta.openGraph) {
	if (htmlMeta.openGraph['image']) {
		console.log(`Open Graph Image: ${htmlMeta.openGraph['image']}`);
	}
	if (htmlMeta.openGraph['title']) {
		console.log(`Open Graph Title: ${htmlMeta.openGraph['title']}`);
	}
	if (htmlMeta.openGraph['type']) {
		console.log(`Open Graph Type: ${htmlMeta.openGraph['type']}`);
	}
}

// Access Twitter Card fields from map
if (htmlMeta.twitterCard) {
	if (htmlMeta.twitterCard['card']) {
		console.log(`Twitter Card Type: ${htmlMeta.twitterCard['card']}`);
	}
	if (htmlMeta.twitterCard['creator']) {
		console.log(`Twitter Creator: ${htmlMeta.twitterCard['creator']}`);
	}
}

// Access new fields
if (htmlMeta.language) {
	console.log(`Language: ${htmlMeta.language}`);
}

if (htmlMeta.textDirection) {
	console.log(`Text Direction: ${htmlMeta.textDirection}`);
}

// Access headers
if (htmlMeta.headers && htmlMeta.headers.length > 0) {
	console.log(`Headers: ${htmlMeta.headers.join(', ')}`);
}

// Access links
if (htmlMeta.links && htmlMeta.links.length > 0) {
	htmlMeta.links.forEach((link) => {
		console.log(`Link: ${link.href} (${link.text})`);
	});
}

// Access images
if (htmlMeta.images && htmlMeta.images.length > 0) {
	htmlMeta.images.forEach((image) => {
		console.log(`Image: ${image.src}`);
	});
}

// Access structured data
if (htmlMeta.structuredData && htmlMeta.structuredData.length > 0) {
	console.log(`Structured data items: ${htmlMeta.structuredData.length}`);
}
```
