```kotlin title="Kotlin"
import java.net.URI
import java.net.http.HttpClient
import java.net.http.HttpRequest
import java.net.http.HttpResponse

fun main() {
    val client = HttpClient.newHttpClient()
    val json = """
        {
          "text": "Your long text here...",
          "chunker_type": "text",
          "config": {
            "max_characters": 1000,
            "overlap": 50,
            "trim": true
          }
        }
    """.trimIndent()

    val request = HttpRequest.newBuilder()
        .uri(URI.create("http://localhost:8000/chunk"))
        .header("Content-Type", "application/json")
        .POST(HttpRequest.BodyPublishers.ofString(json))
        .build()

    val response = client.send(request, HttpResponse.BodyHandlers.ofString())
    println("Status: ${response.statusCode()}")
    println(response.body())
}
```
