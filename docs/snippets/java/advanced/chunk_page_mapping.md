```java title="Java"
import dev.kreuzberg.Kreuzberg;
import dev.kreuzberg.ExtractionConfig;
import dev.kreuzberg.ChunkingConfig;
import dev.kreuzberg.PageConfig;
import java.nio.file.Path;
import java.util.Optional;

ExtractionConfig config = ExtractionConfig.builder()
    .withChunking(Optional.of(ChunkingConfig.builder()
        .withMaxCharacters(500L)
        .withOverlap(50L)
        .build()))
    .withPages(Optional.of(PageConfig.builder()
        .withExtractPages(true)
        .build()))
    .build();

var result = Kreuzberg.extractFileSync(Path.of("document.pdf"), config);

if (result.chunks() != null) {
    for (var chunk : result.chunks()) {
        Long firstPage = chunk.metadata().firstPage();
        Long lastPage = chunk.metadata().lastPage();
        if (firstPage != null && lastPage != null) {
            String pageRange = firstPage.equals(lastPage)
                ? "Page " + firstPage
                : "Pages " + firstPage + "-" + lastPage;

            String content = chunk.content();
            String preview = content.substring(0, Math.min(50, content.length()));
            System.out.println("Chunk: " + preview + "... (" + pageRange + ")");
        }
    }
}
```
