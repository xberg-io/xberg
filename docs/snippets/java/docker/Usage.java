```java title="Usage.java"
import java.io.*;
import java.net.HttpURLConnection;
import java.net.URL;
import java.nio.file.Files;
import java.nio.file.Paths;
import java.util.UUID;
import com.google.gson.JsonParser;

public final class Usage {
  private static final String BOUNDARY = "----WebKitFormBoundary" + UUID.randomUUID();
  private final String containerName;
  private final int apiPort;

  public Usage(String containerName, int apiPort) {
    this.containerName = containerName;
    this.apiPort = apiPort;
  }

  public void startContainer(String image) throws IOException, InterruptedException {
    System.out.println("Starting Xberg Docker container...");
    ProcessBuilder pb = new ProcessBuilder("docker", "run", "-d",
      "--name", containerName,
      "-p", apiPort + ":8000",
      image);
    Process process = pb.start();
    if (process.waitFor() != 0) {
      throw new RuntimeException("Failed to start container");
    }
    System.out.println("Container started on http://localhost:" + apiPort);
  }

  public String extract(String filePath) throws IOException {
    byte[] fileBytes = Files.readAllBytes(Paths.get(filePath));
    String fileName = Paths.get(filePath).getFileName().toString();

    URL url = new URL("http://localhost:" + apiPort + "/api/extract");
    HttpURLConnection conn = (HttpURLConnection) url.openConnection();
    conn.setRequestMethod("POST");
    conn.setRequestProperty("Content-Type", "multipart/form-data; boundary=" + BOUNDARY);
    conn.setDoOutput(true);

    try (OutputStream os = conn.getOutputStream()) {
      os.write(("--" + BOUNDARY + "\r\n").getBytes());
      os.write(("Content-Disposition: form-data; name=\"file\"; filename=\"" + fileName + "\"\r\n").getBytes());
      os.write("Content-Type: application/octet-stream\r\n\r\n".getBytes());
      os.write(fileBytes);
      os.write(("\r\n--" + BOUNDARY + "--\r\n").getBytes());
    }

    StringBuilder response = new StringBuilder();
    try (BufferedReader reader = new BufferedReader(new InputStreamReader(conn.getInputStream()))) {
      String line;
      while ((line = reader.readLine()) != null) {
        response.append(line);
      }
    }

    return JsonParser.parseString(response.toString())
      .getAsJsonObject()
      .get("content")
      .getAsString();
  }

  public void stopContainer() throws IOException, InterruptedException {
    System.out.println("Stopping Xberg Docker container...");
    new ProcessBuilder("docker", "stop", containerName).start().waitFor();
    new ProcessBuilder("docker", "rm", containerName).start().waitFor();
    System.out.println("Container stopped and removed");
  }

  public static void main(String[] args) throws Exception {
    Usage docker = new Usage("xberg-api", 8000);

    try {
      docker.startContainer("xberg:latest");
      Thread.sleep(2000);

      String content = docker.extract("document.pdf");
      System.out.println("Extracted content:\n" + content);
    } finally {
      docker.stopContainer();
    }
  }
}
```
