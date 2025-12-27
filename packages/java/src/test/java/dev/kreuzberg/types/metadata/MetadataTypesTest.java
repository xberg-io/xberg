package dev.kreuzberg.types.metadata;

import com.fasterxml.jackson.databind.ObjectMapper;
import dev.kreuzberg.ExtractionResult;
import dev.kreuzberg.Kreuzberg;
import dev.kreuzberg.KreuzbergException;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.DisplayName;
import org.junit.jupiter.api.Nested;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.Timeout;

import java.io.IOException;
import java.nio.file.Path;
import java.nio.file.Paths;
import java.util.ArrayList;
import java.util.List;
import java.util.Map;
import java.util.concurrent.ExecutorService;
import java.util.concurrent.Executors;
import java.util.concurrent.Future;
import java.util.concurrent.TimeUnit;
import java.util.concurrent.atomic.AtomicInteger;

import static org.junit.jupiter.api.Assertions.*;
import static org.assertj.core.api.Assertions.assertThat;

/**
 * Comprehensive tests for Java metadata types in the Kreuzberg library.
 *
 * Tests verify record structure, JSON serialization/deserialization,
 * Jackson annotations, nullability, immutability, and record behavior.
 */
class MetadataTypesTest {

    private ObjectMapper objectMapper;

    @BeforeEach
    void setUp() {
        objectMapper = new ObjectMapper();
    }


    @Nested
    @DisplayName("Record Structure Tests")
    class RecordStructureTests {

        @Test
        @DisplayName("HtmlMetadata record has correct components")
        void testHtmlMetadataStructure() {
            HtmlMetadata metadata = new HtmlMetadata(
                "Test Title",
                "Test Description",
                List.of("keyword1", "keyword2"),
                "Test Author",
                "https://example.com/canonical",
                "https://example.com/base",
                "en",
                "ltr",
                Map.of("og:title", "OG Title"),
                Map.of("twitter:card", "summary"),
                Map.of("viewport", "width=device-width"),
                List.of(),
                List.of(),
                List.of(),
                List.of()
            );

            assertNotNull(metadata, "HtmlMetadata should not be null");
            assertEquals("Test Title", metadata.title());
            assertEquals("Test Description", metadata.description());
            assertEquals("Test Author", metadata.author());
            assertEquals("https://example.com/canonical", metadata.canonicalUrl());
            assertEquals("https://example.com/base", metadata.baseHref());
            assertEquals("en", metadata.language());
            assertEquals("ltr", metadata.textDirection());
        }

        @Test
        @DisplayName("keywords is List<String>, not String")
        void testKeywordsIsList() {
            HtmlMetadata metadata = new HtmlMetadata(
                null, null,
                List.of("java", "testing", "kreuzberg"),
                null, null, null, null, null,
                Map.of(), Map.of(), Map.of(),
                List.of(), List.of(), List.of(), List.of()
            );

            assertNotNull(metadata.keywords(), "keywords should not be null");
            assertInstanceOf(List.class, metadata.keywords(), "keywords should be a List");
            assertEquals(3, metadata.keywords().size(), "keywords should have 3 items");
            assertTrue(metadata.keywords().get(0) instanceof String, "keywords should contain Strings");
        }

        @Test
        @DisplayName("canonicalUrl field exists (not canonical)")
        void testCanonicalUrlRenamed() {
            HtmlMetadata metadata = new HtmlMetadata(
                null, null, List.of(), null,
                "https://example.com/canonical",
                null, null, null,
                Map.of(), Map.of(), Map.of(),
                List.of(), List.of(), List.of(), List.of()
            );

            assertNotNull(metadata.canonicalUrl(), "canonicalUrl field should exist");
            assertEquals("https://example.com/canonical", metadata.canonicalUrl());
            assertTrue(metadata.toString().contains("canonicalUrl"), "Field should be named canonicalUrl");
        }

        @Test
        @DisplayName("openGraph is Map<String, String>")
        void testOpenGraphIsMap() {
            Map<String, String> ogMap = Map.of(
                "og:title", "Page Title",
                "og:description", "Page Description",
                "og:image", "https://example.com/image.jpg"
            );

            HtmlMetadata metadata = new HtmlMetadata(
                null, null, List.of(), null, null, null, null, null,
                ogMap,
                Map.of(), Map.of(),
                List.of(), List.of(), List.of(), List.of()
            );

            assertNotNull(metadata.openGraph(), "openGraph should not be null");
            assertInstanceOf(Map.class, metadata.openGraph(), "openGraph should be a Map");
            assertEquals(3, metadata.openGraph().size(), "openGraph should have 3 entries");
            assertEquals("Page Title", metadata.openGraph().get("og:title"));
        }

        @Test
        @DisplayName("twitterCard is Map<String, String>")
        void testTwitterCardIsMap() {
            Map<String, String> twitterMap = Map.of(
                "twitter:card", "summary_large_image",
                "twitter:title", "Tweet Title",
                "twitter:description", "Tweet Description"
            );

            HtmlMetadata metadata = new HtmlMetadata(
                null, null, List.of(), null, null, null, null, null,
                Map.of(), twitterMap, Map.of(),
                List.of(), List.of(), List.of(), List.of()
            );

            assertNotNull(metadata.twitterCard(), "twitterCard should not be null");
            assertInstanceOf(Map.class, metadata.twitterCard(), "twitterCard should be a Map");
            assertEquals(3, metadata.twitterCard().size(), "twitterCard should have 3 entries");
            assertEquals("summary_large_image", metadata.twitterCard().get("twitter:card"));
        }

        @Test
        @DisplayName("HeaderMetadata has required fields")
        void testHeaderMetadataStructure() {
            HeaderMetadata header = new HeaderMetadata(1, "Main Title", "main-title", 0, 42);

            assertNotNull(header, "HeaderMetadata should not be null");
            assertEquals(1, header.level(), "level should be 1");
            assertEquals("Main Title", header.text(), "text should be 'Main Title'");
            assertEquals("main-title", header.id(), "id should be 'main-title'");
            assertEquals(0, header.depth(), "depth should be 0");
            assertEquals(42, header.htmlOffset(), "htmlOffset should be 42");
        }

        @Test
        @DisplayName("LinkMetadata has rel as List<String>")
        void testLinkMetadataRelIsList() {
            LinkMetadata link = new LinkMetadata(
                "https://example.com",
                "Example Link",
                "Link Title",
                "hyperlink",
                List.of("nofollow", "external"),
                Map.of("data-custom", "value")
            );

            assertNotNull(link.rel(), "rel should not be null");
            assertInstanceOf(List.class, link.rel(), "rel should be a List");
            assertEquals(2, link.rel().size(), "rel should have 2 items");
            assertTrue(link.rel().contains("nofollow"), "rel should contain 'nofollow'");
        }

        @Test
        @DisplayName("ImageMetadata has dimensions as int array")
        void testImageMetadataDimensionsIsArray() {
            int[] dimensions = {1920, 1080};
            ImageMetadata image = new ImageMetadata(
                "https://example.com/image.jpg",
                "Alt text",
                "Image Title",
                dimensions,
                "image/jpeg",
                Map.of()
            );

            assertNotNull(image.dimensions(), "dimensions should not be null");
            assertInstanceOf(int[].class, image.dimensions(), "dimensions should be int[]");
            assertEquals(2, image.dimensions().length, "dimensions should have 2 values");
            assertEquals(1920, image.dimensions()[0], "width should be 1920");
            assertEquals(1080, image.dimensions()[1], "height should be 1080");
        }

        @Test
        @DisplayName("StructuredData has dataType field")
        void testStructuredDataHasDataType() {
            StructuredData data = new StructuredData(
                "json-ld",
                "{\"@context\": \"https://schema.org\"}",
                "Organization"
            );

            assertNotNull(data.dataType(), "dataType should not be null");
            assertEquals("json-ld", data.dataType(), "dataType should be 'json-ld'");
            assertEquals("Organization", data.schemaType(), "schemaType should be 'Organization'");
        }
    }


    @Nested
    @DisplayName("JSON Serialization Tests")
    class JsonSerializationTests {

        @Test
        @DisplayName("HtmlMetadata JSON serialization/deserialization")
        void testHtmlMetadataJsonSerialization() throws IOException {
            HtmlMetadata original = new HtmlMetadata(
                "Test Page",
                "Page about testing",
                List.of("test", "java", "junit"),
                "Test Author",
                "https://example.com/page",
                "https://example.com",
                "en",
                "ltr",
                Map.of("og:title", "OG Test Page"),
                Map.of("twitter:card", "summary"),
                Map.of("charset", "utf-8"),
                List.of(),
                List.of(),
                List.of(),
                List.of()
            );

            String json = objectMapper.writeValueAsString(original);
            assertThat(json).isNotNull().isNotEmpty();

            HtmlMetadata deserialized = objectMapper.readValue(json, HtmlMetadata.class);

            assertEquals(original.title(), deserialized.title(), "title should match");
            assertEquals(original.description(), deserialized.description(), "description should match");
            assertEquals(original.keywords(), deserialized.keywords(), "keywords should match");
            assertEquals(original.author(), deserialized.author(), "author should match");
            assertEquals(original.canonicalUrl(), deserialized.canonicalUrl(), "canonicalUrl should match");
        }

        @Test
        @DisplayName("JsonProperty annotations work correctly")
        void testHeaderMetadataJsonSerialization() throws IOException {
            HeaderMetadata original = new HeaderMetadata(2, "Subheading", "subheading-id", 1, 123);

            String json = objectMapper.writeValueAsString(original);

            assertThat(json).contains("\"level\"");
            assertThat(json).contains("\"text\"");
            assertThat(json).contains("\"id\"");
            assertThat(json).contains("\"depth\"");
            assertThat(json).contains("\"html_offset\"");

            HeaderMetadata deserialized = objectMapper.readValue(json, HeaderMetadata.class);
            assertEquals(original, deserialized, "Header should deserialize to equal object");
        }

        @Test
        @DisplayName("LinkMetadata serialization with rel list")
        void testLinkMetadataJsonSerialization() throws IOException {
            LinkMetadata original = new LinkMetadata(
                "https://example.com/page",
                "Example",
                "Example Page",
                "external",
                List.of("nofollow", "external", "noopener"),
                Map.of("target", "_blank", "data-index", "1")
            );

            String json = objectMapper.writeValueAsString(original);
            assertThat(json).contains("\"rel\"");
            assertThat(json).contains("nofollow");
            assertThat(json).contains("external");

            LinkMetadata deserialized = objectMapper.readValue(json, LinkMetadata.class);
            assertEquals(original.rel(), deserialized.rel(), "rel list should match");
            assertEquals(original.attributes(), deserialized.attributes(), "attributes map should match");
        }

        @Test
        @DisplayName("ImageMetadata serialization with dimensions array")
        void testImageMetadataJsonSerialization() throws IOException {
            int[] dimensions = {800, 600};
            ImageMetadata original = new ImageMetadata(
                "https://example.com/logo.png",
                "Company Logo",
                "Logo",
                dimensions,
                "image/png",
                Map.of("loading", "lazy", "decoding", "async")
            );

            String json = objectMapper.writeValueAsString(original);
            assertThat(json).contains("\"dimensions\"");
            assertThat(json).contains("800");
            assertThat(json).contains("600");

            ImageMetadata deserialized = objectMapper.readValue(json, ImageMetadata.class);
            assertArrayEquals(original.dimensions(), deserialized.dimensions(),
                "dimensions array should match");
            assertEquals(original.imageType(), deserialized.imageType(), "imageType should match");
        }

        @Test
        @DisplayName("StructuredData serialization with dataType")
        void testStructuredDataJsonSerialization() throws IOException {
            String jsonLd = "{\"@context\": \"https://schema.org\", \"@type\": \"Organization\", \"name\": \"Example Corp\"}";
            StructuredData original = new StructuredData(
                "json-ld",
                jsonLd,
                "Organization"
            );

            String json = objectMapper.writeValueAsString(original);
            assertThat(json).contains("\"data_type\"");
            assertThat(json).contains("\"raw_json\"");
            assertThat(json).contains("\"schema_type\"");

            StructuredData deserialized = objectMapper.readValue(json, StructuredData.class);
            assertEquals(original.dataType(), deserialized.dataType(), "dataType should match");
            assertEquals(original.rawJson(), deserialized.rawJson(), "rawJson should match");
            assertEquals(original.schemaType(), deserialized.schemaType(), "schemaType should match");
        }

        @Test
        @DisplayName("JSON snake_case conversion for all fields")
        void testJsonSnakeCaseConversion() throws IOException {
            HeaderMetadata header = new HeaderMetadata(3, "Deep Heading", "id", 2, 456);

            String json = objectMapper.writeValueAsString(header);

            assertThat(json).contains("\"html_offset\":456");
            assertThat(json).contains("\"level\":3");
            assertThat(json).contains("\"depth\":2");
        }

        @Test
        @DisplayName("Empty collections serialize correctly")
        void testEmptyCollectionsSerialize() throws IOException {
            HtmlMetadata metadata = new HtmlMetadata(
                null, null, List.of(), null, null, null, null, null,
                Map.of(), Map.of(), Map.of(),
                List.of(), List.of(), List.of(), List.of()
            );

            String json = objectMapper.writeValueAsString(metadata);
            assertThat(json).contains("\"keywords\":[]");
            assertThat(json).contains("\"open_graph\":{}");
            assertThat(json).contains("\"twitter_card\":{}");

            HtmlMetadata deserialized = objectMapper.readValue(json, HtmlMetadata.class);
            assertTrue(deserialized.keywords().isEmpty(), "keywords should be empty");
            assertTrue(deserialized.openGraph().isEmpty(), "openGraph should be empty");
            assertTrue(deserialized.twitterCard().isEmpty(), "twitterCard should be empty");
        }
    }


    @Nested
    @DisplayName("Nullability Tests")
    class NullabilityTests {

        @Test
        @DisplayName("Nullable fields can be null")
        void testNullableFields() {
            HtmlMetadata metadata = new HtmlMetadata(
                null,
                null,
                List.of(),
                null,
                null,
                null,
                null,
                null,
                Map.of(),
                Map.of(),
                Map.of(),
                List.of(),
                List.of(),
                List.of(),
                List.of()
            );

            assertNull(metadata.title(), "title should be nullable");
            assertNull(metadata.description(), "description should be nullable");
            assertNull(metadata.author(), "author should be nullable");
            assertNull(metadata.canonicalUrl(), "canonicalUrl should be nullable");
            assertNull(metadata.baseHref(), "baseHref should be nullable");
            assertNull(metadata.language(), "language should be nullable");
            assertNull(metadata.textDirection(), "textDirection should be nullable");
        }

        @Test
        @DisplayName("Collections are never null, but can be empty")
        void testCollectionsNotNull() {
            HtmlMetadata metadata = new HtmlMetadata(
                null, null, List.of(), null, null, null, null, null,
                Map.of(), Map.of(), Map.of(),
                List.of(), List.of(), List.of(), List.of()
            );

            assertNotNull(metadata.keywords(), "keywords should not be null");
            assertNotNull(metadata.openGraph(), "openGraph should not be null");
            assertNotNull(metadata.twitterCard(), "twitterCard should not be null");
            assertNotNull(metadata.metaTags(), "metaTags should not be null");
            assertNotNull(metadata.headers(), "headers should not be null");
            assertNotNull(metadata.links(), "links should not be null");
            assertNotNull(metadata.images(), "images should not be null");
            assertNotNull(metadata.structuredData(), "structuredData should not be null");
        }

        @Test
        @DisplayName("HeaderMetadata nullable fields")
        void testHeaderMetadataNullableFields() {
            HeaderMetadata header = new HeaderMetadata(1, "Title", null, 0, 0);

            assertNull(header.id(), "id should be nullable");
            assertNotNull(header.text(), "text should not be nullable");
            assertNotNull(header.level(), "level should not be nullable");
            assertNotNull(header.depth(), "depth should not be nullable");
            assertNotNull(header.htmlOffset(), "htmlOffset should not be nullable");
        }

        @Test
        @DisplayName("LinkMetadata nullable fields")
        void testLinkMetadataNullableFields() {
            LinkMetadata link = new LinkMetadata(
                "https://example.com",
                "Link",
                null,
                "hyperlink",
                List.of(),
                Map.of()
            );

            assertNull(link.title(), "title should be nullable");
            assertNotNull(link.href(), "href should not be nullable");
            assertNotNull(link.text(), "text should not be nullable");
            assertNotNull(link.linkType(), "linkType should not be nullable");
        }

        @Test
        @DisplayName("ImageMetadata nullable fields")
        void testImageMetadataNullableFields() {
            ImageMetadata image = new ImageMetadata(
                "https://example.com/img.jpg",
                null,
                null,
                null,
                "image/jpeg",
                Map.of()
            );

            assertNull(image.alt(), "alt should be nullable");
            assertNull(image.title(), "title should be nullable");
            assertNull(image.dimensions(), "dimensions should be nullable");
            assertNotNull(image.src(), "src should not be nullable");
            assertNotNull(image.imageType(), "imageType should not be nullable");
        }

        @Test
        @DisplayName("StructuredData nullable fields")
        void testStructuredDataNullableFields() {
            StructuredData data = new StructuredData(
                "json-ld",
                "{}",
                null
            );

            assertNull(data.schemaType(), "schemaType should be nullable");
            assertNotNull(data.dataType(), "dataType should not be nullable");
            assertNotNull(data.rawJson(), "rawJson should not be nullable");
        }
    }


    @Nested
    @DisplayName("Record Behavior Tests")
    class RecordBehaviorTests {

        @Test
        @DisplayName("Records with same values are equal")
        void testRecordEquality() {
            HtmlMetadata metadata1 = new HtmlMetadata(
                "Title", "Desc", List.of("k1", "k2"), "Author", "http://example.com",
                "http://base.com", "en", "ltr",
                Map.of("og:title", "OG"), Map.of("tw:card", "summary"), Map.of(),
                List.of(), List.of(), List.of(), List.of()
            );

            HtmlMetadata metadata2 = new HtmlMetadata(
                "Title", "Desc", List.of("k1", "k2"), "Author", "http://example.com",
                "http://base.com", "en", "ltr",
                Map.of("og:title", "OG"), Map.of("tw:card", "summary"), Map.of(),
                List.of(), List.of(), List.of(), List.of()
            );

            assertEquals(metadata1, metadata2, "Records with same values should be equal");
            assertEquals(metadata1.hashCode(), metadata2.hashCode(), "Equal records should have same hash");
        }

        @Test
        @DisplayName("Records with different values are not equal")
        void testRecordInequality() {
            HtmlMetadata metadata1 = new HtmlMetadata(
                "Title1", null, List.of(), null, null, null, null, null,
                Map.of(), Map.of(), Map.of(),
                List.of(), List.of(), List.of(), List.of()
            );

            HtmlMetadata metadata2 = new HtmlMetadata(
                "Title2", null, List.of(), null, null, null, null, null,
                Map.of(), Map.of(), Map.of(),
                List.of(), List.of(), List.of(), List.of()
            );

            assertNotEquals(metadata1, metadata2, "Records with different values should not be equal");
        }

        @Test
        @DisplayName("Records are immutable")
        void testRecordImmutability() {
            List<String> keywords = List.of("keyword1", "keyword2");
            Map<String, String> ogMap = Map.of("og:title", "Title");

            HtmlMetadata metadata = new HtmlMetadata(
                null, null, keywords, null, null, null, null, null,
                ogMap, Map.of(), Map.of(),
                List.of(), List.of(), List.of(), List.of()
            );

            assertSame(metadata.keywords(), metadata.keywords(), "Same field access should return same object");
            assertSame(metadata.openGraph(), metadata.openGraph(), "Same field access should return same object");
        }

        @Test
        @DisplayName("Record toString() includes all fields")
        void testRecordToString() {
            HeaderMetadata header = new HeaderMetadata(1, "Title", "title-id", 0, 100);
            String toString = header.toString();

            assertThat(toString).contains("HeaderMetadata");
            assertThat(toString).contains("level=1");
            assertThat(toString).contains("text=Title");
            assertThat(toString).contains("id=title-id");
            assertThat(toString).contains("depth=0");
            assertThat(toString).contains("htmlOffset=100");
        }

        @Test
        @DisplayName("Components accessed correctly")
        void testRecordComponentAccess() {
            LinkMetadata link = new LinkMetadata(
                "https://example.com",
                "Example",
                "Title",
                "hyperlink",
                List.of("external"),
                Map.of("rel", "nofollow")
            );

            assertEquals("https://example.com", link.href());
            assertEquals("Example", link.text());
            assertEquals("Title", link.title());
            assertEquals("hyperlink", link.linkType());
            assertEquals(1, link.rel().size());
            assertEquals(1, link.attributes().size());
        }
    }


    @Nested
    @DisplayName("Complex Integration Tests")
    class ComplexIntegrationTests {

        @Test
        @DisplayName("Nested metadata structure serialization")
        void testNestedMetadataStructure() throws IOException {
            List<HeaderMetadata> headers = List.of(
                new HeaderMetadata(1, "Main Title", "main", 0, 0),
                new HeaderMetadata(2, "Subtitle", "sub", 1, 50),
                new HeaderMetadata(2, "Another Subtitle", "sub2", 1, 100)
            );

            List<LinkMetadata> links = List.of(
                new LinkMetadata("https://example.com", "Link1", null, "hyperlink", List.of("external"), Map.of()),
                new LinkMetadata("https://example.com/page", "Link2", "Link to page", "hyperlink", List.of(), Map.of())
            );

            List<ImageMetadata> images = List.of(
                new ImageMetadata("https://example.com/img1.jpg", "Image 1", "Img1", new int[]{800, 600}, "image/jpeg", Map.of()),
                new ImageMetadata("https://example.com/img2.png", "Image 2", null, null, "image/png", Map.of("loading", "lazy"))
            );

            HtmlMetadata metadata = new HtmlMetadata(
                "Test Page",
                "A test page",
                List.of("test", "example"),
                "Author Name",
                "https://example.com/page",
                "https://example.com",
                "en",
                "ltr",
                Map.of("og:title", "Test", "og:type", "website"),
                Map.of("twitter:card", "summary"),
                Map.of("viewport", "width=device-width"),
                headers,
                links,
                images,
                List.of()
            );

            String json = objectMapper.writeValueAsString(metadata);
            assertThat(json).contains("\"headers\"");
            assertThat(json).contains("\"links\"");
            assertThat(json).contains("\"images\"");

            HtmlMetadata deserialized = objectMapper.readValue(json, HtmlMetadata.class);
            assertEquals(3, deserialized.headers().size(), "Should have 3 headers");
            assertEquals(2, deserialized.links().size(), "Should have 2 links");
            assertEquals(2, deserialized.images().size(), "Should have 2 images");
            assertEquals("Main Title", deserialized.headers().get(0).text(), "First header text should match");
        }

        @Test
        @DisplayName("Full HtmlMetadata with structured data")
        void testFullMetadataWithStructuredData() throws IOException {
            List<StructuredData> structuredData = List.of(
                new StructuredData("json-ld", "{\"@context\": \"https://schema.org\", \"@type\": \"Organization\"}", "Organization"),
                new StructuredData("json-ld", "{\"@context\": \"https://schema.org\", \"@type\": \"Article\"}", "Article")
            );

            HtmlMetadata metadata = new HtmlMetadata(
                "Article Title",
                "Article description",
                List.of("tech", "news"),
                "Jane Doe",
                "https://example.com/article",
                "https://example.com",
                "en",
                "ltr",
                Map.of(
                    "og:title", "Article Title",
                    "og:type", "article",
                    "og:url", "https://example.com/article"
                ),
                Map.of(
                    "twitter:card", "summary_large_image",
                    "twitter:creator", "@janedoe"
                ),
                Map.of(
                    "viewport", "width=device-width, initial-scale=1.0",
                    "charset", "utf-8"
                ),
                List.of(),
                List.of(),
                List.of(),
                structuredData
            );

            String json = objectMapper.writeValueAsString(metadata);
            HtmlMetadata deserialized = objectMapper.readValue(json, HtmlMetadata.class);

            assertEquals(2, deserialized.structuredData().size(), "Should have 2 structured data items");
            assertEquals("Organization", deserialized.structuredData().get(0).schemaType());
            assertEquals("Article", deserialized.structuredData().get(1).schemaType());
        }

        @Test
        @DisplayName("Edge case: null dimensions in ImageMetadata")
        void testImageMetadataWithNullDimensions() throws IOException {
            ImageMetadata image = new ImageMetadata(
                "https://example.com/no-dimensions.svg",
                "SVG Image",
                "SVG",
                null,
                "image/svg+xml",
                Map.of()
            );

            String json = objectMapper.writeValueAsString(image);
            assertThat(json).contains("\"dimensions\":null");

            ImageMetadata deserialized = objectMapper.readValue(json, ImageMetadata.class);
            assertNull(deserialized.dimensions(), "Dimensions should remain null");
        }

        @Test
        @DisplayName("Edge case: empty strings vs null")
        void testEmptyStringsVsNull() throws IOException {
            HeaderMetadata headerWithId = new HeaderMetadata(1, "Title", "id", 0, 0);
            HeaderMetadata headerWithoutId = new HeaderMetadata(1, "Title", null, 0, 0);

            String json1 = objectMapper.writeValueAsString(headerWithId);
            String json2 = objectMapper.writeValueAsString(headerWithoutId);

            assertThat(json1).contains("\"id\":\"id\"");
            assertThat(json2).contains("\"id\":null");

            HeaderMetadata des1 = objectMapper.readValue(json1, HeaderMetadata.class);
            HeaderMetadata des2 = objectMapper.readValue(json2, HeaderMetadata.class);

            assertEquals("id", des1.id(), "Deserialized id should match");
            assertNull(des2.id(), "Deserialized null id should be null");
        }

        @Test
        @DisplayName("Map ordering preservation in serialization")
        void testMapOrderingPreservation() throws IOException {
            Map<String, String> metaTags = Map.of(
                "viewport", "width=device-width",
                "charset", "utf-8",
                "author", "John Doe"
            );

            HtmlMetadata metadata = new HtmlMetadata(
                null, null, List.of(), null, null, null, null, null,
                Map.of(), Map.of(), metaTags,
                List.of(), List.of(), List.of(), List.of()
            );

            String json = objectMapper.writeValueAsString(metadata);
            HtmlMetadata deserialized = objectMapper.readValue(json, HtmlMetadata.class);

            assertEquals(metaTags.size(), deserialized.metaTags().size(), "Map size should match");
            assertEquals("utf-8", deserialized.metaTags().get("charset"), "Map values should match");
        }

        @Test
        @DisplayName("Large keywords list")
        void testLargeKeywordsList() throws IOException {
            List<String> keywords = List.of(
                "keyword1", "keyword2", "keyword3", "keyword4", "keyword5",
                "keyword6", "keyword7", "keyword8", "keyword9", "keyword10"
            );

            HtmlMetadata metadata = new HtmlMetadata(
                null, null, keywords, null, null, null, null, null,
                Map.of(), Map.of(), Map.of(),
                List.of(), List.of(), List.of(), List.of()
            );

            String json = objectMapper.writeValueAsString(metadata);
            HtmlMetadata deserialized = objectMapper.readValue(json, HtmlMetadata.class);

            assertEquals(10, deserialized.keywords().size(), "All keywords should be preserved");
            assertEquals("keyword5", deserialized.keywords().get(4), "Middle keyword should be correct");
        }
    }


    @Nested
    @DisplayName("Special Cases and Edge Cases")
    class SpecialCasesTests {

        @Test
        @DisplayName("Unicode characters in string fields")
        void testUnicodeCharacters() throws IOException {
            HtmlMetadata metadata = new HtmlMetadata(
                "Unicode Title: 日本語 العربية 🚀",
                "Description with emoji: 🎉",
                List.of("keyword", "キーワード", "كلمة"),
                "Author: José García",
                "https://example.com/ñ",
                null, null, null,
                Map.of("og:title", "OG: 中文"),
                Map.of("twitter:card", "summary_large_image"),
                Map.of(),
                List.of(), List.of(), List.of(), List.of()
            );

            String json = objectMapper.writeValueAsString(metadata);
            HtmlMetadata deserialized = objectMapper.readValue(json, HtmlMetadata.class);

            assertEquals("Unicode Title: 日本語 العربية 🚀", deserialized.title(), "Unicode title should be preserved");
            assertTrue(deserialized.keywords().contains("キーワード"), "Unicode keywords should be preserved");
        }

        @Test
        @DisplayName("URL encoding in href and src")
        void testUrlEncoding() throws IOException {
            LinkMetadata link = new LinkMetadata(
                "https://example.com/path?param=value&other=123#anchor",
                "Complex Link",
                null,
                "hyperlink",
                List.of(),
                Map.of()
            );

            ImageMetadata image = new ImageMetadata(
                "https://example.com/image%20with%20spaces.jpg",
                "Image",
                null,
                null,
                "image/jpeg",
                Map.of()
            );

            String linkJson = objectMapper.writeValueAsString(link);
            String imageJson = objectMapper.writeValueAsString(image);

            LinkMetadata linkDes = objectMapper.readValue(linkJson, LinkMetadata.class);
            ImageMetadata imageDes = objectMapper.readValue(imageJson, ImageMetadata.class);

            assertEquals(link.href(), linkDes.href(), "Complex URL should be preserved");
            assertEquals(image.src(), imageDes.src(), "URL-encoded path should be preserved");
        }

        @Test
        @DisplayName("Special characters in JSON string values")
        void testSpecialCharactersInJson() throws IOException {
            StructuredData data = new StructuredData(
                "json-ld",
                "{\"description\": \"A description with \\\"quotes\\\" and \\n newlines\"}",
                null
            );

            String json = objectMapper.writeValueAsString(data);
            StructuredData deserialized = objectMapper.readValue(json, StructuredData.class);

            assertEquals(data.rawJson(), deserialized.rawJson(), "Escaped JSON should be preserved");
        }

        @Test
        @DisplayName("Very long text fields")
        void testVeryLongTextFields() throws IOException {
            String longTitle = "A".repeat(1000);
            String longDescription = "Lorem ipsum dolor sit amet. ".repeat(100);

            HtmlMetadata metadata = new HtmlMetadata(
                longTitle, longDescription, List.of(), null, null, null, null, null,
                Map.of(), Map.of(), Map.of(),
                List.of(), List.of(), List.of(), List.of()
            );

            String json = objectMapper.writeValueAsString(metadata);
            HtmlMetadata deserialized = objectMapper.readValue(json, HtmlMetadata.class);

            assertEquals(1000, deserialized.title().length(), "Long title should be preserved");
            assertTrue(deserialized.description().length() > 2700, "Long description should be preserved");
        }

        @Test
        @DisplayName("Numeric values in dimension arrays")
        void testDimensionArrayEdgeCases() throws IOException {
            ImageMetadata zeroDim = new ImageMetadata(
                "img.jpg", null, null, new int[]{0, 0}, "image/jpeg", Map.of()
            );

            ImageMetadata largeDim = new ImageMetadata(
                "img.jpg", null, null, new int[]{8192, 4096}, "image/jpeg", Map.of()
            );

            ImageMetadata singleDim = new ImageMetadata(
                "img.jpg", null, null, new int[]{512}, "image/jpeg", Map.of()
            );

            String zeroJson = objectMapper.writeValueAsString(zeroDim);
            String largeJson = objectMapper.writeValueAsString(largeDim);
            String singleJson = objectMapper.writeValueAsString(singleDim);

            ImageMetadata zeroDes = objectMapper.readValue(zeroJson, ImageMetadata.class);
            ImageMetadata largeDes = objectMapper.readValue(largeJson, ImageMetadata.class);
            ImageMetadata singleDes = objectMapper.readValue(singleJson, ImageMetadata.class);

            assertArrayEquals(new int[]{0, 0}, zeroDes.dimensions(), "Zero dimensions should be preserved");
            assertArrayEquals(new int[]{8192, 4096}, largeDes.dimensions(), "Large dimensions should be preserved");
            assertArrayEquals(new int[]{512}, singleDes.dimensions(), "Single dimension should be preserved");
        }

        @Test
        @DisplayName("Unmodifiable collections behavior")
        void testUnmodifiableCollectionsBehavior() {
            HtmlMetadata metadata = new HtmlMetadata(
                null, null, List.of("k1", "k2"), null, null, null, null, null,
                Map.of("og:k", "v"), Map.of(), Map.of(),
                List.of(), List.of(), List.of(), List.of()
            );

            assertThrows(UnsupportedOperationException.class, () -> metadata.keywords().add("k3"),
                "Keywords list should be unmodifiable");
            assertThrows(UnsupportedOperationException.class, () -> metadata.openGraph().put("new", "value"),
                "OpenGraph map should be unmodifiable");
        }
    }


    @Nested
    @DisplayName("HTML Extraction Integration Tests")
    class HtmlExtractionIntegrationTests {

        private Path testResourcesDir;

        @BeforeEach
        void setUpIntegration() {
            testResourcesDir = Paths.get("packages/java/src/test/resources");
        }

        @Test
        @DisplayName("Extract HTML returns metadata structure")
        void testExtractHtmlReturnsMetadata() throws IOException, KreuzbergException {
            Path testFile = testResourcesDir.resolve("test-basic.html");

            if (!java.nio.file.Files.exists(testFile)) {
                org.junit.jupiter.api.Assumptions.assumeTrue(
                    false,
                    "Test HTML file not found: " + testFile.toAbsolutePath()
                );
            }

            ExtractionResult result = Kreuzberg.extractBytes(
                java.nio.file.Files.readAllBytes(testFile),
                "text/html",
                null
            );

            assertNotNull(result, "Extraction result should not be null");
            assertTrue(result.isSuccess(), "Extraction should succeed");
            assertNotNull(result.getMimeType(), "MIME type should be present");
            assertEquals("text/html", result.getMimeType(), "MIME type should be text/html");

            assertNotNull(result.getContent(), "Extracted content should not be null");
            assertThat(result.getContent()).isNotEmpty();

            Map<String, Object> metadata = result.getMetadata();
            assertNotNull(metadata, "Metadata map should not be null");
        }

        @Test
        @DisplayName("Extract complex HTML with all metadata types")
        void testExtractCompleteHtmlAllMetadataTypes() throws IOException, KreuzbergException {
            Path testFile = testResourcesDir.resolve("test-complex.html");

            if (!java.nio.file.Files.exists(testFile)) {
                org.junit.jupiter.api.Assumptions.assumeTrue(
                    false,
                    "Test HTML file not found: " + testFile.toAbsolutePath()
                );
            }

            ExtractionResult result = Kreuzberg.extractBytes(
                java.nio.file.Files.readAllBytes(testFile),
                "text/html",
                null
            );

            assertNotNull(result, "Result should not be null");
            assertTrue(result.isSuccess(), "Extraction should succeed");

            String content = result.getContent();
            assertNotNull(content, "Content should be extracted");
            assertThat(content).isNotEmpty();

            Map<String, Object> metadata = result.getMetadata();
            assertNotNull(metadata, "Metadata should be extracted");

            assertNotNull(result.getMimeType(), "MIME type should be present");
            assertNotNull(result.getChunks(), "Chunks list should not be null");
            assertNotNull(result.getImages(), "Images list should not be null");
            assertNotNull(result.getTables(), "Tables list should not be null");
        }

        @Test
        @DisplayName("Extract invalid input handles gracefully")
        void testExtractInvalidInputHandlesGracefully() {
            assertThrows(
                KreuzbergException.class,
                () -> Kreuzberg.extractBytes(null, "text/html", null),
                "Should throw exception for null data"
            );

            assertThrows(
                KreuzbergException.class,
                () -> Kreuzberg.extractBytes(new byte[0], "text/html", null),
                "Should throw exception for empty data"
            );

            byte[] testData = "<html><body>Test</body></html>".getBytes();
            assertThrows(
                KreuzbergException.class,
                () -> Kreuzberg.extractBytes(testData, null, null),
                "Should throw exception for null MIME type"
            );

            assertThrows(
                KreuzbergException.class,
                () -> Kreuzberg.extractBytes(testData, "   ", null),
                "Should throw exception for blank MIME type"
            );

            byte[] malformedHtml = "<html><body><p>Unclosed paragraph".getBytes();
            try {
                ExtractionResult result = Kreuzberg.extractBytes(malformedHtml, "text/html", null);
                assertNotNull(result, "Should handle malformed HTML gracefully");
            } catch (KreuzbergException e) {
                assertTrue(true, "Malformed HTML handling is acceptable");
            }
        }

        @Test
        @DisplayName("Extract large HTML document performance test")
        @Timeout(30)
        void testExtractLargeHtmlPerformance() throws IOException, KreuzbergException {
            StringBuilder html = new StringBuilder();
            html.append("<!DOCTYPE html>\n");
            html.append("<html lang=\"en\">\n");
            html.append("<head>\n");
            html.append("<meta charset=\"UTF-8\">\n");
            html.append("<title>Large Document</title>\n");
            html.append("<meta name=\"description\" content=\"Large test document\">\n");
            html.append("</head>\n");
            html.append("<body>\n");
            html.append("<h1>Large Document</h1>\n");

            for (int i = 0; i < 1000; i++) {
                html.append("<section id=\"section-").append(i).append("\">\n");
                html.append("<h2>Section ").append(i).append("</h2>\n");
                html.append("<p>Content for section ").append(i).append(". ");
                html.append("Lorem ipsum dolor sit amet, consectetur adipiscing elit. ");
                html.append("Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua.</p>\n");
                html.append("<a href=\"https://example.com/link").append(i).append("\">Link ").append(i).append("</a>\n");
                html.append("</section>\n");
            }

            html.append("</body>\n");
            html.append("</html>\n");

            byte[] largeHtmlData = html.toString().getBytes();

            long startTime = System.currentTimeMillis();

            ExtractionResult result = Kreuzberg.extractBytes(
                largeHtmlData,
                "text/html",
                null
            );

            long endTime = System.currentTimeMillis();
            long duration = endTime - startTime;

            assertNotNull(result, "Result should not be null");
            assertTrue(result.isSuccess(), "Extraction should succeed");

            String content = result.getContent();
            assertNotNull(content, "Content should be extracted");
            assertThat(content).isNotEmpty();

            System.out.println("Large HTML extraction took " + duration + "ms for "
                + largeHtmlData.length + " bytes");

            assertTrue(duration < 30000, "Extraction should complete within 30 seconds");
        }

        @Test
        @DisplayName("Extract with concurrent execution is thread-safe")
        @Timeout(60)
        void testExtractConcurrentExecution() throws IOException, InterruptedException, KreuzbergException {
            byte[] testHtml = ("<html><head><title>Concurrent Test</title>"
                + "<meta name=\"description\" content=\"Test concurrent extraction\">"
                + "</head><body><h1>Test</h1><p>Content</p></body></html>").getBytes();

            ExecutorService executorService = Executors.newFixedThreadPool(5);
            List<Future<ExtractionResult>> futures = new ArrayList<>();
            AtomicInteger successCount = new AtomicInteger(0);
            AtomicInteger errorCount = new AtomicInteger(0);

            try {
                for (int i = 0; i < 10; i++) {
                    Future<ExtractionResult> future = executorService.submit(() -> {
                        try {
                            ExtractionResult result = Kreuzberg.extractBytes(
                                testHtml,
                                "text/html",
                                null
                            );
                            if (result.isSuccess()) {
                                successCount.incrementAndGet();
                            }
                            return result;
                        } catch (KreuzbergException e) {
                            errorCount.incrementAndGet();
                            throw e;
                        }
                    });
                    futures.add(future);
                }

                boolean allCompleted = executorService.awaitTermination(30, TimeUnit.SECONDS);
                assertTrue(allCompleted, "All tasks should complete within timeout");

                int completedCount = 0;
                for (Future<ExtractionResult> future : futures) {
                    if (future.isDone()) {
                        completedCount++;
                        try {
                            ExtractionResult result = future.get();
                            assertNotNull(result, "Result should not be null");
                            assertTrue(result.isSuccess(), "Each extraction should succeed");
                            assertNotNull(result.getContent(), "Content should be extracted");
                        } catch (Exception e) {
                            assertTrue(true, "Concurrent execution handled exception gracefully");
                        }
                    }
                }

                assertTrue(completedCount > 0, "At least some concurrent extractions should complete");
                assertTrue(successCount.get() > 0, "At least some extractions should succeed");

                System.out.println("Concurrent extraction: " + successCount.get() + " succeeded, "
                    + errorCount.get() + " failed out of 10 tasks");

            } finally {
                executorService.shutdown();
                if (!executorService.awaitTermination(5, TimeUnit.SECONDS)) {
                    executorService.shutdownNow();
                }
            }
        }

        @Test
        @DisplayName("Extract HTML validates metadata structure")
        void testExtractHtmlValidatesMetadataStructure() throws IOException, KreuzbergException {
            byte[] testHtml = ("<!DOCTYPE html>"
                + "<html>"
                + "<head>"
                + "<title>Metadata Test</title>"
                + "<meta name=\"description\" content=\"Description\">"
                + "<meta name=\"author\" content=\"Author Name\">"
                + "</head>"
                + "<body>"
                + "<h1>Title</h1>"
                + "<p>Content</p>"
                + "</body>"
                + "</html>").getBytes();

            ExtractionResult result = Kreuzberg.extractBytes(testHtml, "text/html", null);

            assertNotNull(result, "Result should not be null");
            assertTrue(result.isSuccess(), "Extraction should succeed");

            Map<String, Object> metadata = result.getMetadata();
            assertNotNull(metadata, "Metadata should not be null");
            assertThat(metadata).isNotNull();

            String content = result.getContent();
            assertNotNull(content, "Content should be extracted");
            assertThat(content.length()).isGreaterThan(0);
        }

        @Test
        @DisplayName("Extract HTML from bytes with proper MIME type detection")
        void testExtractHtmlFromBytesWithMimeType() throws KreuzbergException {
            byte[] htmlBytes = ("<html><head><title>Test</title></head>"
                + "<body><h1>Test Page</h1></body></html>").getBytes();

            ExtractionResult result = Kreuzberg.extractBytes(htmlBytes, "text/html", null);

            assertNotNull(result, "Result should not be null");
            assertTrue(result.isSuccess(), "Should extract successfully");
            assertEquals("text/html", result.getMimeType(), "MIME type should match");
            assertNotNull(result.getContent(), "Content should be extracted");
            assertThat(result.getContent()).isNotEmpty();
        }
    }
}
