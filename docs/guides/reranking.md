# Reranking

Rerank candidate documents by joint relevance scoring. After vector retrieval returns top-K candidates, rerank to surface the most relevant documents for LLM context.

## Quick example

Use the `fast` preset to rerank three documents against a query.

=== "Python"

    ```python
    from xberg import rerank_sync, RerankerConfig, RerankerModelType

    query = "How to train a dog"
    documents = [
        "Dog training requires patience and consistency.",
        "Cats are independent animals that prefer to play alone.",
        "Bird care includes proper cage setup and regular cleaning.",
    ]

    config = RerankerConfig(
        model=RerankerModelType(type="preset", name="fast"),
        top_k=2,
    )

    results = rerank_sync(query, documents, config)
    for result in results:
        print(f"#{result.index}: {result.score:.3f} — {result.document}")
    ```

=== "TypeScript"

    ```typescript
    import { rerankSync, RerankerConfig } from "@xberg-io/xberg";

    const config: RerankerConfig = {
      model: { type: "preset", name: "fast" },
      top_k: 2,
    };

    const results = rerankSync(
      "How to train a dog",
      [
        "Dog training requires patience and consistency.",
        "Cats are independent animals that prefer to play alone.",
        "Bird care includes proper cage setup and regular cleaning.",
      ],
      config,
    );

    for (const r of results) {
      console.log(`#${r.index}: ${r.score.toFixed(3)} — ${r.document}`);
    }
    ```

=== "Rust"

    ```rust
    use xberg::{rerank, RerankerConfig, RerankerModelType};

    let query = "How to train a dog".to_string();
    let documents = vec![
        "Dog training requires patience and consistency.".to_string(),
        "Cats are independent animals that prefer to play alone.".to_string(),
        "Bird care includes proper cage setup and regular cleaning.".to_string(),
    ];

    let config = RerankerConfig {
        model: RerankerModelType::Preset { name: "fast".to_string() },
        top_k: Some(2),
        ..Default::default()
    };

    let results = rerank(query, documents, &config)?;
    for r in results {
        println!("#{}: {:.3} — {}", r.index, r.score, r.document);
    }
    # Ok::<(), xberg::XbergError>(())
    ```

=== "Go"

    ```go
    import "github.com/xberg-io/xberg"

    config := xberg.RerankerConfig{
        Model: &xberg.RerankerModelTypePreset{Name: "fast"},
        TopK:  xberg.Ptr(uint(2)),
    }
    results, err := xberg.Rerank(
        "How to train a dog",
        []string{
            "Dog training requires patience and consistency.",
            "Cats are independent animals that prefer to play alone.",
            "Bird care includes proper cage setup and regular cleaning.",
        },
        &config,
    )
    if err != nil {
        log.Fatal(err)
    }
    for _, r := range results {
        fmt.Printf("#%d: %.3f — %s\n", r.Index, r.Score, r.Document)
    }
    ```

=== "Java"

    ```java
    import io.xberg.*;

    RerankerConfig config = new RerankerConfig.Builder()
        .model(new RerankerModelType.Preset("fast"))
        .topK(2L)
        .build();

    var results = Xberg.rerank(
        "How to train a dog",
        java.util.List.of(
            "Dog training requires patience and consistency.",
            "Cats are independent animals that prefer to play alone.",
            "Bird care includes proper cage setup and regular cleaning."
        ),
        config
    );

    for (var r : results) {
        System.out.printf("#%d: %.3f — %s%n", r.getIndex(), r.getScore(), r.getDocument());
    }
    ```

## Picking a preset

| Preset | When to use |
|--------|-------------|
| `fast` | Latency-critical retrieval, English-only. 22M parameters, ~50ms on CPU for 10 docs. |
| `balanced` | Production English/Chinese RAG. 278M parameters, the recommended default. |
| `quality` | Complex queries where accuracy matters more than latency. 560M parameters. |
| `multilingual` | International documents or long context (up to 8192 tokens). 100+ languages. |

All four download lazily from HuggingFace on first use and cache under `~/.cache/xberg/rerankers/`.

## Custom HuggingFace cross-encoder

To use any ONNX cross-encoder from HuggingFace, point the `Custom` variant at its repository ID. The repo must contain an `onnx/model.onnx` file.

```python
from xberg import rerank_sync, RerankerConfig, RerankerModelType

config = RerankerConfig(
    model=RerankerModelType(
        type="custom",
        model_id="cross-encoder/ms-marco-MiniLM-L-12-v2",
        max_length=512,
    ),
)

results = rerank_sync("query text", ["doc1", "doc2"], config)
```

## LLM rerank via liter-llm

For provider-hosted rerankers, use the `Llm` variant with a liter-llm model identifier. The model string must include the provider prefix (`cohere/`, `jina/`, `voyage/`).

```python
import os
from xberg import rerank_sync, RerankerConfig, RerankerModelType, LlmConfig

config = RerankerConfig(
    model=RerankerModelType(
        type="llm",
        llm=LlmConfig(
            model="cohere/rerank-english-v3.0",
            api_key=os.environ["COHERE_API_KEY"],
        ),
    ),
    top_k=5,
)

results = rerank_sync("query text", documents, config)
```

Set `COHERE_API_KEY` (or `JINA_API_KEY`, `VOYAGE_API_KEY`) in the environment. The Llm variant requires the `liter-llm` Cargo feature.

## In-process plugin backend

To wrap a model you already load — `sentence-transformers`, `llama-cpp-python`, a tuned ONNX session — implement the `RerankerBackend` protocol and register it once at startup.

The protocol returns **raw scores in input order**. The dispatcher handles sorting and `top_k` truncation; the plugin must not sort.

```python
from xberg import register_reranker_backend, rerank_sync, RerankerConfig, RerankerModelType

class MyReranker:
    def __init__(self):
        from sentence_transformers import CrossEncoder
        self._model = CrossEncoder("cross-encoder/ms-marco-MiniLM-L-12-v2")

    def name(self) -> str:
        return "my-reranker"

    def version(self) -> str:
        return "1.0.0"

    def initialize(self) -> None:
        pass

    def shutdown(self) -> None:
        pass

    async def rerank(self, query: str, documents: list[str]) -> list[float]:
        scores = self._model.predict([(query, doc) for doc in documents])
        return scores.tolist()


register_reranker_backend(MyReranker())

config = RerankerConfig(model=RerankerModelType(type="plugin", name="my-reranker"))
results = rerank_sync("query text", ["doc1", "doc2"], config)
```

The Plugin variant works on every target (including WASM) because no native ONNX Runtime is loaded.

## HTTP `/rerank`

When the API server is running, `POST /rerank` accepts JSON requests:

```bash
curl -X POST http://localhost:8000/rerank \
  -H "Content-Type: application/json" \
  -d '{
    "query": "How to train a dog",
    "documents": [
      "Dog training requires patience.",
      "Cats are independent animals.",
      "Bird care includes proper cage setup."
    ],
    "config": {
      "model": {"type": "preset", "name": "fast"},
      "top_k": 2
    }
  }'
```

Response:

```json
{
  "results": [
    {"index": 0, "score": 0.987, "document": "Dog training requires patience."},
    {"index": 2, "score": 0.412, "document": "Bird care includes proper cage setup."}
  ]
}
```

The endpoint caps `documents.length` at 1024 entries — chunk larger candidate sets client-side.

## Performance notes

- **`batch_size`** controls how many `(query, document)` pairs share a forward pass. The default of 32 is a good fit for CPU; raise to 64-128 on GPU.
- **`top_k`** truncates the response after scoring — it does not reduce inference cost. Always score the full candidate set, then pick.
- **Sigmoid normalization** is applied automatically to local-model logits so scores fall in `[0, 1]`. LLM rerankers return provider-native scores unchanged.
- **First-call latency** is dominated by model download. Warm the cache during application startup, not on the first user request.
