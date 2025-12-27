```java title="Java"
import dev.kreuzberg.Kreuzberg;
import dev.kreuzberg.ExtractionResult;
import dev.kreuzberg.KreuzbergException;
import java.io.IOException;
import java.util.Map;
import java.util.List;

public class Main {
    public static void main(String[] args) {
        try {
            ExtractionResult result = Kreuzberg.extractFileSync("document.pdf");

            // Access PDF metadata
            @SuppressWarnings("unchecked")
            Map<String, Object> pdfMeta = (Map<String, Object>) result.getMetadata().get("pdf");
            if (pdfMeta != null) {
                System.out.println("Pages: " + pdfMeta.get("page_count"));
                System.out.println("Author: " + pdfMeta.get("author"));
                System.out.println("Title: " + pdfMeta.get("title"));
            }

            // Access HTML metadata
            ExtractionResult htmlResult = Kreuzberg.extractFileSync("page.html");
            @SuppressWarnings("unchecked")
            Map<String, Object> htmlMeta = (Map<String, Object>) htmlResult.getMetadata().get("html");
            if (htmlMeta != null) {
                System.out.println("Title: " + htmlMeta.get("title"));
                System.out.println("Description: " + htmlMeta.get("description"));

                // Access keywords as array
                @SuppressWarnings("unchecked")
                List<String> keywords = (List<String>) htmlMeta.get("keywords");
                if (keywords != null) {
                    System.out.println("Keywords: " + keywords);
                }

                // Access canonical URL (renamed from canonical)
                String canonicalUrl = (String) htmlMeta.get("canonical_url");
                if (canonicalUrl != null) {
                    System.out.println("Canonical URL: " + canonicalUrl);
                }

                // Access Open Graph fields from map
                @SuppressWarnings("unchecked")
                Map<String, String> openGraph = (Map<String, String>) htmlMeta.get("open_graph");
                if (openGraph != null) {
                    System.out.println("Open Graph Image: " + openGraph.get("image"));
                    System.out.println("Open Graph Title: " + openGraph.get("title"));
                    System.out.println("Open Graph Type: " + openGraph.get("type"));
                }

                // Access Twitter Card fields from map
                @SuppressWarnings("unchecked")
                Map<String, String> twitterCard = (Map<String, String>) htmlMeta.get("twitter_card");
                if (twitterCard != null) {
                    System.out.println("Twitter Card Type: " + twitterCard.get("card"));
                    System.out.println("Twitter Creator: " + twitterCard.get("creator"));
                }

                // Access new fields
                String language = (String) htmlMeta.get("language");
                if (language != null) {
                    System.out.println("Language: " + language);
                }

                String textDirection = (String) htmlMeta.get("text_direction");
                if (textDirection != null) {
                    System.out.println("Text Direction: " + textDirection);
                }

                // Access headers
                @SuppressWarnings("unchecked")
                List<String> headers = (List<String>) htmlMeta.get("headers");
                if (headers != null) {
                    System.out.println("Headers: " + headers);
                }

                // Access links
                @SuppressWarnings("unchecked")
                List<Map<String, String>> links = (List<Map<String, String>>) htmlMeta.get("links");
                if (links != null) {
                    for (Map<String, String> link : links) {
                        System.out.println("Link: " + link.get("href") + " (" + link.get("text") + ")");
                    }
                }

                // Access images
                @SuppressWarnings("unchecked")
                List<Map<String, String>> images = (List<Map<String, String>>) htmlMeta.get("images");
                if (images != null) {
                    for (Map<String, String> image : images) {
                        System.out.println("Image: " + image.get("src"));
                    }
                }

                // Access structured data
                @SuppressWarnings("unchecked")
                List<Map<String, Object>> structuredData = (List<Map<String, Object>>) htmlMeta.get("structured_data");
                if (structuredData != null) {
                    System.out.println("Structured data items: " + structuredData.size());
                }
            }
        } catch (IOException | KreuzbergException e) {
            System.err.println("Extraction failed: " + e.getMessage());
        }
    }
}
```
