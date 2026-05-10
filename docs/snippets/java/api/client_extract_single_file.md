```java title="Java"
import java.net.http.HttpClient;
import java.net.http.HttpRequest;
import java.net.http.HttpResponse;
import java.net.URI;
import java.nio.file.Files;
import java.nio.file.Paths;

HttpClient client = HttpClient.newHttpClient();

try (var fileStream = Files.newInputStream(Paths.get("document.pdf"))) {
    byte[] content = fileStream.readAllBytes();
    var request = HttpRequest.newBuilder()
        .uri(URI.create("http://localhost:8000/extract"))
        .header("Content-Type", "application/octet-stream")
        .POST(HttpRequest.BodyPublishers.ofByteArray(content))
        .build();

    HttpResponse<String> response = client.send(request, HttpResponse.BodyHandlers.ofString());
    System.out.println(response.body());
}
```
