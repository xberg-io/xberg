```java title="Java"
import io.xberg.Xberg;
import io.xberg.ExtractionResult;
import io.xberg.ExtractedDocument;
import io.xberg.ExtractionConfig;
import io.xberg.ExtractInput;
import io.xberg.ChunkingConfig;
import io.xberg.EmbeddingConfig;
import io.xberg.EmbeddingModelType;
import java.util.List;

ExtractionConfig config = ExtractionConfig.builder()
    .chunking(ChunkingConfig.builder()
        .maxChars(512)
        .maxOverlap(50)
        .embedding(EmbeddingConfig.builder()
            .model(EmbeddingModelType.preset("balanced"))
            .normalize(true)
            .batchSize(32)
            .showDownloadProgress(false)
            .build())
        .build())
    .build();

ExtractionResult output = Xberg.extract(
    ExtractInput.fromUri("document.pdf"),
    config
);

ExtractedDocument result = output.results().get(0);

List<Object> chunks = result.getChunks() != null ? result.getChunks() : List.of();
for (int index = 0; index < chunks.size(); index++) {
    Object chunk = chunks.get(index);
    String chunkId = "doc_chunk_" + index;
    System.out.println("Chunk " + chunkId + ": " + chunk.toString().substring(0, Math.min(50, chunk.toString().length())));

    if (chunk instanceof java.util.Map) {
        Object embedding = ((java.util.Map<String, Object>) chunk).get("embedding");
        if (embedding != null) {
            System.out.println("  Embedding dimensions: " + ((float[]) embedding).length);
        }
    }
}
```
