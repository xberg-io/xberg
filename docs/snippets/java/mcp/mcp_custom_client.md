```java title="Java"
import com.fasterxml.jackson.databind.ObjectMapper;
import java.io.BufferedReader;
import java.io.BufferedWriter;
import java.io.IOException;
import java.io.InputStreamReader;
import java.io.OutputStreamWriter;
import java.util.Map;

public class McpCustomClient {
    public static void main(String[] args) throws IOException, InterruptedException {
        ProcessBuilder pb = new ProcessBuilder("xberg", "mcp");
        Process mcp = pb.start();

        ObjectMapper mapper = new ObjectMapper();
        try (BufferedWriter stdin = new BufferedWriter(new OutputStreamWriter(mcp.getOutputStream()));
             BufferedReader stdout = new BufferedReader(new InputStreamReader(mcp.getInputStream()))) {

            Map<String, Object> request = Map.of(
                "method", "tools/call",
                "params", Map.of(
                    "name", "extract",
                    "arguments", Map.of("path", "document.pdf", "async", true)
                )
            );

            stdin.write(mapper.writeValueAsString(request));
            stdin.newLine();
            stdin.flush();

            String line = stdout.readLine();
            if (line != null) {
                System.out.println(line);
            }
        }

        mcp.waitFor();
    }
}
```
