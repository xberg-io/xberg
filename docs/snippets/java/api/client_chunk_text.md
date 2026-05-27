<!-- snippet:skip -->
```java title="Java"
import java.net.http.HttpClient;
import java.net.http.HttpRequest;
import java.net.http.HttpResponse;
import java.net.URI;
import com.fasterxml.jackson.databind.ObjectMapper;
import com.fasterxml.jackson.annotation.JsonProperty;
import java.util.List;

record ChunkRequest(String text, @JsonProperty("chunker_type") String chunkerType, ChunkConfig config) {}
record ChunkConfig(@JsonProperty("max_characters") int maxCharacters, int overlap, boolean trim) {}
record ChunkItem(String content, @JsonProperty("byte_start") int byteStart, @JsonProperty("chunk_index") int chunkIndex) {}

HttpClient client = HttpClient.newHttpClient();
ObjectMapper mapper = new ObjectMapper();

ChunkRequest req = new ChunkRequest("Your long text here...", "text", new ChunkConfig(1000, 50, true));
String json = mapper.writeValueAsString(req);

var request = HttpRequest.newBuilder()
    .uri(URI.create("http://localhost:8000/chunk"))
    .header("Content-Type", "application/json")
    .POST(HttpRequest.BodyPublishers.ofString(json))
    .build();

HttpResponse<String> response = client.send(request, HttpResponse.BodyHandlers.ofString());
var result = mapper.readTree(response.body());
System.out.println("Created " + result.get("chunk_count").asInt() + " chunks");
```
