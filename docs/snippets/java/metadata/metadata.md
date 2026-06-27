```java title="Java"
import io.xberg.Xberg;
import io.xberg.ExtractionResult;
import io.xberg.Metadata;
import io.xberg.XbergException;
import java.io.IOException;
import java.util.Map;
import java.util.List;

public class Main {
    public static void main(String[] args) {
        try {
            ExtractionResult result = Xberg.extractSync("document.pdf");

            // Metadata is flat — format-specific fields are at the top level
            Metadata metadata = result.getMetadata();
            metadata.getTitle().ifPresent(t -> System.out.println("Title: " + t));
            metadata.getAuthors().ifPresent(a -> System.out.println("Authors: " + String.join(", ", a)));

            // Format-specific fields are in the additional map
            Map<String, Object> extra = metadata.getAdditional();
            if (extra.get("page_count") != null) {
                System.out.println("Pages: " + extra.get("page_count"));
            }

            // Access HTML metadata
            ExtractionResult htmlResult = Xberg.extractSync("page.html");
            Metadata htmlMeta = htmlResult.getMetadata();
            htmlMeta.getTitle().ifPresent(t -> System.out.println("Title: " + t));

            Map<String, Object> htmlExtra = htmlMeta.getAdditional();
            String description = (String) htmlExtra.get("description");
            if (description != null) {
                System.out.println("Description: " + description);
            }

            // Access keywords as array
            htmlMeta.getKeywords().ifPresent(keywords ->
                System.out.println("Keywords: " + keywords));

            // Access canonical URL (renamed from canonical)
            String canonicalUrl = (String) htmlExtra.get("canonical_url");
            if (canonicalUrl != null) {
                System.out.println("Canonical URL: " + canonicalUrl);
            }

            // Access Open Graph fields from map
            @SuppressWarnings("unchecked")
            Map<String, String> openGraph = (Map<String, String>) htmlExtra.get("open_graph");
            if (openGraph != null) {
                System.out.println("Open Graph Image: " + openGraph.get("image"));
                System.out.println("Open Graph Title: " + openGraph.get("title"));
                System.out.println("Open Graph Type: " + openGraph.get("type"));
            }

            // Access Twitter Card fields from map
            @SuppressWarnings("unchecked")
            Map<String, String> twitterCard = (Map<String, String>) htmlExtra.get("twitter_card");
            if (twitterCard != null) {
                System.out.println("Twitter Card Type: " + twitterCard.get("card"));
                System.out.println("Twitter Creator: " + twitterCard.get("creator"));
            }

            // Access new fields
            htmlMeta.getLanguage().ifPresent(l -> System.out.println("Language: " + l));

            String textDirection = (String) htmlExtra.get("text_direction");
            if (textDirection != null) {
                System.out.println("Text Direction: " + textDirection);
            }

            // Access headers
            @SuppressWarnings("unchecked")
            List<Map<String, Object>> headers = (List<Map<String, Object>>) htmlExtra.get("headers");
            if (headers != null) {
                headers.stream()
                    .map(h -> h.get("text"))
                    .forEach(text -> System.out.print(text + ", "));
                System.out.println();
            }

            // Access links
            @SuppressWarnings("unchecked")
            List<Map<String, Object>> links = (List<Map<String, Object>>) htmlExtra.get("links");
            if (links != null) {
                for (Map<String, Object> link : links) {
                    System.out.println("Link: " + link.get("href") + " (" + link.get("text") + ")");
                }
            }

            // Access images
            @SuppressWarnings("unchecked")
            List<Map<String, Object>> images = (List<Map<String, Object>>) htmlExtra.get("images");
            if (images != null) {
                for (Map<String, Object> image : images) {
                    System.out.println("Image: " + image.get("src"));
                }
            }

            // Access structured data
            @SuppressWarnings("unchecked")
            List<Map<String, Object>> structuredData = (List<Map<String, Object>>) htmlExtra.get("structured_data");
            if (structuredData != null) {
                System.out.println("Structured data items: " + structuredData.size());
            }
        } catch (IOException | XbergException e) {
            System.err.println("Extraction failed: " + e.getMessage());
        }
    }
}
```
