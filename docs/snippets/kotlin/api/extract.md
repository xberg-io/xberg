```kotlin title="Kotlin"
import io.xberg.ExtractInput
import io.xberg.ExtractInputKind
import io.xberg.ExtractionConfig
import io.xberg.Xberg

val output = Xberg.extract(
    ExtractInput(kind = ExtractInputKind.URI, uri = "document.pdf"),
    ExtractionConfig(),
)

println(output.results.first().content)
```
