```java title="Java"
import dev.kreuzberg.ExtractionConfig;
import dev.kreuzberg.ChunkingConfig;

ExtractionConfig config = ExtractionConfig.builder()
    .chunking(ChunkingConfig.builder()
        .maxChars(1000)
        .maxOverlap(200)
        .build())
    .build();
```

```java title="Java - Markdown with Heading Context"
import dev.kreuzberg.ExtractionConfig;
import dev.kreuzberg.ChunkingConfig;
import dev.kreuzberg.HeadingContext;
import dev.kreuzberg.HeadingLevel;

ExtractionConfig config = ExtractionConfig.builder()
    .chunking(ChunkingConfig.builder()
        .chunkerType("markdown")
        .maxChars(500)
        .maxOverlap(50)
        .sizingTokenizer("Xenova/gpt-4o")
        .build())
    .build();

ExtractionResult result = KreuzbergClient.extractFile("document.md", config);

result.getChunks().forEach(chunk -> {
    var headingContext = chunk.getMetadata().getHeadingContext();
    if (headingContext.isPresent()) {
        System.out.println("Headings:");
        headingContext.get().getHeadings().forEach(heading ->
            System.out.println("  Level " + heading.getLevel() + ": " + heading.getText())
        );
    }
});
```

```java title="Java - Prepend Heading Context"
import dev.kreuzberg.ExtractionConfig;
import dev.kreuzberg.ChunkingConfig;

ExtractionConfig config = ExtractionConfig.builder()
    .chunking(ChunkingConfig.builder()
        .chunkerType("markdown")
        .maxChars(500)
        .maxOverlap(50)
        .prependHeadingContext(true)
        .build())
    .build();

ExtractionResult result = KreuzbergClient.extractFile("document.md", config);

result.getChunks().forEach(chunk -> {
    // Each chunk's content is prefixed with its heading breadcrumb
    System.out.println(chunk.getContent().substring(0, Math.min(100, chunk.getContent().length())));
});
```
