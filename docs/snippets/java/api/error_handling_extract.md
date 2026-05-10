```java title="Java"
import java.net.http.HttpClient;
import java.net.http.HttpRequest;
import java.net.http.HttpResponse;
import java.net.URI;
import java.nio.file.Files;
import java.nio.file.Paths;
import com.fasterxml.jackson.databind.ObjectMapper;

HttpClient client = HttpClient.newHttpClient();
byte[] fileBytes = Files.readAllBytes(Paths.get("document.pdf"));

var request = HttpRequest.newBuilder()
    .uri(URI.create("http://localhost:8000/extract"))
    .header("Content-Type", "application/octet-stream")
    .POST(HttpRequest.BodyPublishers.ofByteArray(fileBytes))
    .build();

HttpResponse<String> response = client.send(request, HttpResponse.BodyHandlers.ofString());

if (response.statusCode() != 200) {
    ObjectMapper mapper = new ObjectMapper();
    var error = mapper.readTree(response.body());
    System.err.println("Error: " + error.get("error_type").asText() + " - " + error.get("message").asText());
} else {
    System.out.println("Success: " + response.body());
}
```
