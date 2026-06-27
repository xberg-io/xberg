```java title="Java"
import io.xberg.Xberg;
import io.xberg.ExtractionResult;
import io.xberg.ExtractionConfig;
import io.xberg.ChunkingConfig;
import io.xberg.EmbeddingConfig;
import io.xberg.EmbeddingModelType;
import java.util.List;

ExtractionConfig config = ExtractionConfig.builder()
    .chunking(ChunkingConfig.builder()
        .maxChars(500)
        .maxOverlap(50)
        .embedding(EmbeddingConfig.builder()
            .model(EmbeddingModelType.preset("all-mpnet-base-v2"))
            .normalize(true)
            .batchSize(16)
            .build())
        .build())
    .build();

try {
    ExtractionResult result = Xberg.extract("research_paper.pdf", config);

    List<Object> chunks = result.getChunks() != null ? result.getChunks() : List.of();
    System.out.println("Found " + chunks.size() + " chunks for RAG pipeline");

    for (int i = 0; i < Math.min(3, chunks.size()); i++) {
        Object chunk = chunks.get(i);
        System.out.println("Chunk " + i + ": " + chunk.toString().substring(0, Math.min(80, chunk.toString().length())) + "...");
    }
} catch (Exception ex) {
    System.err.println("RAG extraction failed: " + ex.getMessage());
}
```
