```python title="Python"
import asyncio
from xberg import embed, EmbeddingConfig

async def main() -> None:
    config = EmbeddingConfig(
        # Pass the LLM model as a dict. The typed
        # EmbeddingModelType.llm(LlmConfig(...)) constructor currently rejects the
        # public LlmConfig class — see https://github.com/xberg-io/xberg/issues/1165.
        model={"type": "llm", "llm": {"model": "openai/text-embedding-3-small"}},
        normalize=True,
    )
    embeddings = await embed(["Hello world"], config=config)
    print(len(embeddings[0]))  # 1536

asyncio.run(main())
```
