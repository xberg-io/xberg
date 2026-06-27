```kotlin title="Kotlin"
import io.xberg.*
import java.nio.file.Paths
import java.util.Optional

fun main() {
    val config = ExtractionConfig.builder().build()
    val result = Xberg.extractSync(Paths.get("document.pdf"), null, config)

    val metadata = result.metadata()
    metadata.title()?.let { println("Title: $it") }
    metadata.authors()?.let { println("Authors: ${it.joinToString(", ")}") }

    // Format-specific metadata via discriminated union
    metadata.format()?.pdf()?.let { pdf ->
        pdf.pageCount()?.let { println("Pages: $it") }
        pdf.producer()?.let { println("Producer: $it") }
        pdf.pdfVersion()?.let { println("PDF Version: $it") }
    }

    // Access HTML metadata
    val htmlResult = Xberg.extractSync(Paths.get("page.html"), null, config)
    htmlResult.metadata().format()?.html()?.let { html ->
        html.title()?.let { println("Title: $it") }
        html.description()?.let { println("Description: $it") }
        html.canonicalUrl()?.let { println("Canonical URL: $it") }
        html.language()?.let { println("Language: $it") }

        // Access keywords list
        println("Keywords: ${html.keywords()}")

        // Open Graph fields are exposed as a Map<String, String>
        html.openGraph()["image"]?.let { println("Open Graph Image: $it") }
        html.openGraph()["title"]?.let { println("Open Graph Title: $it") }

        // Twitter Card fields as a Map<String, String>
        html.twitterCard()["card"]?.let { println("Twitter Card Type: $it") }

        // Headers
        for (header in html.headers()) {
            println("Header (level ${header.level()}): ${header.text()}")
        }

        // Links
        for (link in html.links()) {
            println("Link: ${link.href()} (${link.text()})")
        }

        // Images
        for (image in html.images()) {
            println("Image: ${image.src()}")
        }

        // Structured data
        if (html.structuredData().isNotEmpty()) {
            println("Structured data items: ${html.structuredData().size}")
        }
    }
}
```
