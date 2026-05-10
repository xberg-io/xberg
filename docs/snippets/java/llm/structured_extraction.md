```java title="Java"
import dev.kreuzberg.ExtractionConfig;
import dev.kreuzberg.ExtractionResult;
import dev.kreuzberg.Kreuzberg;
import dev.kreuzberg.LlmConfig;
import dev.kreuzberg.StructuredExtractionConfig;

import java.nio.file.Path;
import java.util.List;
import java.util.Map;

public class StructuredExtractionExample {
    public static void main(String[] args) throws Exception {
        Map<String, Object> schema = Map.of(
            "type", "object",
            "properties", Map.of(
                "title", Map.of("type", "string"),
                "authors", Map.of("type", "array", "items", Map.of("type", "string")),
                "date", Map.of("type", "string")
            ),
            "required", List.of("title", "authors", "date"),
            "additionalProperties", false
        );

        LlmConfig llm = LlmConfig.builder()
            .withModel("openai/gpt-4o-mini")
            .build();

        StructuredExtractionConfig structured = new StructuredExtractionConfig(
            schema,
            "PaperMetadata",
            null,
            true,
            null,
            llm
        );

        ExtractionConfig config = ExtractionConfig.builder()
            .withStructuredExtraction(java.util.Optional.of(structured))
            .build();

        ExtractionResult result = Kreuzberg.extractFile(Path.of("paper.pdf"), config);
        System.out.println(result.structuredOutput());
    }
}
```
