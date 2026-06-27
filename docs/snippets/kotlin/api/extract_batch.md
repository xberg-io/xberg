```kotlin title="Kotlin"
import io.xberg.ExtractInput
import io.xberg.ExtractInputKind
import io.xberg.ExtractionConfig
import io.xberg.Xberg

val output = Xberg.extractBatch(
    listOf(
        ExtractInput(kind = ExtractInputKind.URI, uri = "document.pdf"),
        ExtractInput(
            kind = ExtractInputKind.BYTES,
            bytes = "Hello from memory".toByteArray(),
            mimeType = "text/plain",
            filename = "note.txt",
        ),
    ),
    ExtractionConfig(),
)

output.results.forEach { result ->
    println(result.content)
}
```
