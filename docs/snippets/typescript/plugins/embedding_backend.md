```typescript title="TypeScript"
import { registerEmbeddingBackend, embedTexts } from '@kreuzberg/node';

// Wrap an already-loaded embedder so kreuzberg can call back into it during
// chunking and standalone embed requests.
class MyEmbedder {
	// Plugin trait hooks
	name(): string {
		return 'my-embedder';
	}

	version(): string {
		return '1.0.0';
	}

	initialize(): void {
		// Optional warm-up; runs once at registration before dimensions() is cached.
	}

	shutdown(): void {}

	// EmbeddingBackend hooks
	dimensions(): number {
		// Captured once at registration; the dispatcher uses this for shape validation.
		return 768;
	}

	async embed(texts: string[]): Promise<number[][]> {
		// Delegate to the already-loaded host-language embedder.
		return texts.map(() => new Array(768).fill(0));
	}
}

// Register once at startup.
registerEmbeddingBackend(new MyEmbedder());

const vectors = await embedTexts(['Hello, world!', 'Second text'], {
	model: { type: 'plugin', name: 'my-embedder' },
	// Optional: bound the wait on a hung backend (default 60s; null disables).
	maxEmbedDurationSecs: 30,
});
```
