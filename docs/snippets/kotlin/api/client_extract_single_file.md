```kotlin title="Kotlin"
import java.net.URI
import java.net.http.HttpClient
import java.net.http.HttpRequest
import java.net.http.HttpResponse
import java.nio.file.Files
import java.nio.file.Paths

fun main() {
    val client = HttpClient.newHttpClient()
    val path = Paths.get("document.pdf")
    val bytes = Files.readAllBytes(path)
    val fileName = path.fileName.toString()

    val boundary = "----KreuzbergBoundary${System.currentTimeMillis()}"
    val crlf = "\r\n"
    val header = (
        "--$boundary$crlf" +
        "Content-Disposition: form-data; name=\"file\"; filename=\"$fileName\"$crlf" +
        "Content-Type: application/pdf$crlf$crlf"
    ).toByteArray()
    val footer = "$crlf--$boundary--$crlf".toByteArray()

    val body = ByteArray(header.size + bytes.size + footer.size)
    System.arraycopy(header, 0, body, 0, header.size)
    System.arraycopy(bytes, 0, body, header.size, bytes.size)
    System.arraycopy(footer, 0, body, header.size + bytes.size, footer.size)

    val request = HttpRequest.newBuilder()
        .uri(URI.create("http://localhost:8000/extract"))
        .header("Content-Type", "multipart/form-data; boundary=$boundary")
        .POST(HttpRequest.BodyPublishers.ofByteArray(body))
        .build()

    val response = client.send(request, HttpResponse.BodyHandlers.ofString())
    println(response.body())
}
```
