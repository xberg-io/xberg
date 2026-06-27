```kotlin title="Kotlin"
import io.xberg.*
import java.nio.file.Paths
import java.util.Optional

fun main() {
    val htmlOutput = HtmlOutputConfig.builder()
        .withTheme(HtmlTheme.GitHub)
        .build()

    val config = ExtractionConfig.builder()
        .withOutputFormat(OutputFormat.Html)
        .withHtmlOutput(Optional.of(htmlOutput))
        .build()

    val result = Xberg.extractSync(Paths.get("document.pdf"), null, config)
    println(result.content()) // HTML with kb-* classes
}
```
