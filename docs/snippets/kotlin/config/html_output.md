```kotlin title="Kotlin"
import io.xberg.*
import java.util.Optional

fun main() {
    val htmlOutput = HtmlOutputConfig.builder()
        .withTheme(HtmlTheme.GitHub)
        .build()

    val config = ExtractionConfig.builder()
        .withOutputFormat(OutputFormat.Html)
        .withHtmlOutput(Optional.of(htmlOutput))
        .build()

    val resultOutput = Xberg.extract(
        ExtractInput(kind = ExtractInputKind.URI, uri = "document.pdf"),
        config,
    )
    val result = resultOutput.results().first()
    println(result.content()) // HTML with kb-* classes
}
```
