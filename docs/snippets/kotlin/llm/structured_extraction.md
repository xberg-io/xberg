```kotlin title="Kotlin"
import io.xberg.*
import java.nio.file.Paths
import java.util.Optional

fun main() {
    val schema = mapOf(
        "type" to "object",
        "properties" to mapOf(
            "title" to mapOf("type" to "string"),
            "authors" to mapOf("type" to "array", "items" to mapOf("type" to "string")),
            "date" to mapOf("type" to "string")
        ),
        "required" to listOf("title", "authors", "date"),
        "additionalProperties" to false
    )

    val llm = LlmConfig.builder()
        .withModel("openai/gpt-4o-mini")
        .build()

    val structured = StructuredExtractionConfig(
        schema,
        "document",
        null,
        true,
        null,
        llm
    )

    val config = ExtractionConfig.builder()
        .withStructuredExtraction(Optional.of(structured))
        .build()

    val result = Xberg.extractSync(Paths.get("paper.pdf"), null, config)
    result.structuredOutput()?.let { println(it) }
}
```
