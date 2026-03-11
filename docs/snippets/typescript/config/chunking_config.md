```typescript title="TypeScript"
import { extractFile } from '@kreuzberg/node';

const config = {
	chunking: {
		maxChars: 1000,
		maxOverlap: 200,
	},
};

const result = await extractFile('document.pdf', null, config);
console.log(`Total chunks: ${result.chunks?.length ?? 0}`);
```

```typescript title="TypeScript - Markdown with Heading Context"
import { extractFile } from '@kreuzberg/node';

const config = {
	chunking: {
		chunkerType: 'markdown',
		maxChars: 500,
		maxOverlap: 50,
		sizingType: 'tokenizer',
		sizingModel: 'Xenova/gpt-4o',
	},
};

const result = await extractFile('document.md', null, config);
for (const chunk of result.chunks ?? []) {
	const headings = chunk.metadata?.headingContext?.headings ?? [];
	for (const heading of headings) {
		console.log(`Heading L${heading.level}: ${heading.text}`);
	}
	console.log(`Content: ${chunk.content.slice(0, 100)}...`);
}
```
