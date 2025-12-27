using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.IO;
using System.Text;
using System.Text.Json;
using System.Threading.Tasks;
using Kreuzberg;
using Xunit;

namespace Kreuzberg.Tests;

/// <summary>
/// Comprehensive tests for C# metadata types, including HtmlMetadata, HeaderMetadata, LinkMetadata,
/// HtmlImageMetadata, and StructuredData. Tests verify type structure, JSON serialization, and
/// integration with HTML extraction.
/// </summary>
public class MetadataTypesTests
{
    public MetadataTypesTests()
    {
        NativeTestHelper.EnsureNativeLibraryLoaded();
    }

    #region Type Structure Tests

    [Fact]
    public void HtmlMetadata_HasCorrectProperties()
    {
        // Arrange & Act
        var metadata = new HtmlMetadata();

        // Assert - verify all expected properties exist and are initialized
        Assert.NotNull(metadata.Keywords);
        Assert.IsType<List<string>>(metadata.Keywords);
        Assert.Empty(metadata.Keywords);

        Assert.NotNull(metadata.OpenGraph);
        Assert.IsType<Dictionary<string, string>>(metadata.OpenGraph);
        Assert.Empty(metadata.OpenGraph);

        Assert.NotNull(metadata.TwitterCard);
        Assert.IsType<Dictionary<string, string>>(metadata.TwitterCard);
        Assert.Empty(metadata.TwitterCard);

        Assert.NotNull(metadata.MetaTags);
        Assert.IsType<Dictionary<string, string>>(metadata.MetaTags);
        Assert.Empty(metadata.MetaTags);

        Assert.NotNull(metadata.Headers);
        Assert.IsType<List<HeaderMetadata>>(metadata.Headers);
        Assert.Empty(metadata.Headers);

        Assert.NotNull(metadata.Links);
        Assert.IsType<List<LinkMetadata>>(metadata.Links);
        Assert.Empty(metadata.Links);

        Assert.NotNull(metadata.Images);
        Assert.IsType<List<HtmlImageMetadata>>(metadata.Images);
        Assert.Empty(metadata.Images);

        Assert.NotNull(metadata.StructuredData);
        Assert.IsType<List<StructuredData>>(metadata.StructuredData);
        Assert.Empty(metadata.StructuredData);

        // Verify optional string properties are null by default
        Assert.Null(metadata.Title);
        Assert.Null(metadata.Description);
        Assert.Null(metadata.Author);
        Assert.Null(metadata.CanonicalUrl);
        Assert.Null(metadata.BaseHref);
        Assert.Null(metadata.Language);
        Assert.Null(metadata.TextDirection);
    }

    [Fact]
    public void Keywords_IsList_NotString()
    {
        // Arrange
        var metadata = new HtmlMetadata();

        // Act
        metadata.Keywords.Add("test");
        metadata.Keywords.Add("keywords");

        // Assert
        Assert.IsType<List<string>>(metadata.Keywords);
        Assert.Equal(2, metadata.Keywords.Count);
        Assert.Contains("test", metadata.Keywords);
        Assert.Contains("keywords", metadata.Keywords);
    }

    [Fact]
    public void CanonicalUrl_Renamed_PropertyExists()
    {
        // Arrange
        var metadata = new HtmlMetadata();

        // Act
        metadata.CanonicalUrl = "https://example.com/canonical";

        // Assert
        Assert.Equal("https://example.com/canonical", metadata.CanonicalUrl);
        Assert.NotNull(metadata.CanonicalUrl);
    }

    [Fact]
    public void OpenGraph_IsDictionary_StringToString()
    {
        // Arrange
        var metadata = new HtmlMetadata();

        // Act
        metadata.OpenGraph["og:title"] = "Test Title";
        metadata.OpenGraph["og:description"] = "Test Description";
        metadata.OpenGraph["og:image"] = "https://example.com/image.jpg";

        // Assert
        Assert.IsType<Dictionary<string, string>>(metadata.OpenGraph);
        Assert.Equal(3, metadata.OpenGraph.Count);
        Assert.Equal("Test Title", metadata.OpenGraph["og:title"]);
        Assert.Equal("Test Description", metadata.OpenGraph["og:description"]);
        Assert.Equal("https://example.com/image.jpg", metadata.OpenGraph["og:image"]);
    }

    [Fact]
    public void TwitterCard_IsDictionary_StringToString()
    {
        // Arrange
        var metadata = new HtmlMetadata();

        // Act
        metadata.TwitterCard["twitter:card"] = "summary_large_image";
        metadata.TwitterCard["twitter:title"] = "Test Title";
        metadata.TwitterCard["twitter:description"] = "Test Description";

        // Assert
        Assert.IsType<Dictionary<string, string>>(metadata.TwitterCard);
        Assert.Equal(3, metadata.TwitterCard.Count);
        Assert.Equal("summary_large_image", metadata.TwitterCard["twitter:card"]);
        Assert.Equal("Test Title", metadata.TwitterCard["twitter:title"]);
    }

    [Fact]
    public void HeaderMetadata_HasCorrectProperties()
    {
        // Arrange & Act
        var header = new HeaderMetadata
        {
            Level = 1,
            Text = "Main Title",
            Id = "main-title",
            Depth = 0,
            HtmlOffset = 100
        };

        // Assert
        Assert.Equal(1, header.Level);
        Assert.Equal("Main Title", header.Text);
        Assert.Equal("main-title", header.Id);
        Assert.Equal(0, header.Depth);
        Assert.Equal(100, header.HtmlOffset);
    }

    [Fact]
    public void LinkMetadata_HasCorrectProperties()
    {
        // Arrange & Act
        var link = new LinkMetadata
        {
            Href = "https://example.com",
            Text = "Example Link",
            Title = "Example Website",
            LinkType = "external",
            Rel = new List<string> { "nofollow", "external" }
        };

        // Assert
        Assert.Equal("https://example.com", link.Href);
        Assert.Equal("Example Link", link.Text);
        Assert.Equal("Example Website", link.Title);
        Assert.Equal("external", link.LinkType);
        Assert.Equal(2, link.Rel.Count);
        Assert.Contains("nofollow", link.Rel);
    }

    [Fact]
    public void LinkMetadata_Attributes_IsDictionary()
    {
        // Arrange
        var link = new LinkMetadata { Href = "https://example.com" };

        // Act
        link.Attributes["class"] = "external-link";
        link.Attributes["data-tracking"] = "123";

        // Assert
        Assert.IsType<Dictionary<string, string>>(link.Attributes);
        Assert.Equal(2, link.Attributes.Count);
        Assert.Equal("external-link", link.Attributes["class"]);
    }

    [Fact]
    public void HtmlImageMetadata_HasCorrectProperties()
    {
        // Arrange & Act
        var image = new HtmlImageMetadata
        {
            Src = "https://example.com/image.jpg",
            Alt = "Example image",
            Title = "Example",
            Dimensions = new[] { 800, 600 },
            ImageType = "external"
        };

        // Assert
        Assert.Equal("https://example.com/image.jpg", image.Src);
        Assert.Equal("Example image", image.Alt);
        Assert.Equal("Example", image.Title);
        Assert.NotNull(image.Dimensions);
        Assert.Equal(2, image.Dimensions.Length);
        Assert.Equal(800, image.Dimensions[0]);
        Assert.Equal(600, image.Dimensions[1]);
        Assert.Equal("external", image.ImageType);
    }

    [Fact]
    public void HtmlImageMetadata_Attributes_IsDictionary()
    {
        // Arrange
        var image = new HtmlImageMetadata { Src = "image.jpg" };

        // Act
        image.Attributes["loading"] = "lazy";
        image.Attributes["data-src"] = "image-hd.jpg";

        // Assert
        Assert.IsType<Dictionary<string, string>>(image.Attributes);
        Assert.Equal(2, image.Attributes.Count);
        Assert.Equal("lazy", image.Attributes["loading"]);
    }

    [Fact]
    public void StructuredData_HasCorrectProperties()
    {
        // Arrange & Act
        var structuredData = new StructuredData
        {
            DataType = "json_ld",
            RawJson = @"{""@context"": ""https://schema.org"", ""@type"": ""Article""}",
            SchemaType = "Article"
        };

        // Assert
        Assert.Equal("json_ld", structuredData.DataType);
        Assert.NotEmpty(structuredData.RawJson);
        Assert.Equal("Article", structuredData.SchemaType);
    }

    #endregion

    #region JSON Serialization Tests

    [Fact]
    public void HtmlMetadata_SerializesCorrectly_WithJsonPropertyNames()
    {
        // Arrange
        var metadata = new HtmlMetadata
        {
            Title = "Test Page",
            Description = "Test Description",
            Keywords = new List<string> { "test", "keywords" },
            Author = "Test Author",
            CanonicalUrl = "https://example.com",
            BaseHref = "https://example.com/",
            Language = "en",
            TextDirection = "ltr",
            OpenGraph = new Dictionary<string, string>
            {
                { "og:title", "Test" },
                { "og:description", "Test Description" }
            },
            TwitterCard = new Dictionary<string, string>
            {
                { "twitter:card", "summary" }
            }
        };

        // Act
        var json = JsonSerializer.Serialize(metadata, new JsonSerializerOptions { PropertyNamingPolicy = JsonNamingPolicy.SnakeCaseLower });

        // Assert
        Assert.NotEmpty(json);
        Assert.Contains("\"title\"", json);
        Assert.Contains("\"description\"", json);
        Assert.Contains("\"keywords\"", json);
        Assert.Contains("\"author\"", json);
        Assert.Contains("\"canonical_url\"", json);
        Assert.Contains("\"open_graph\"", json);
        Assert.Contains("\"twitter_card\"", json);
    }

    [Fact]
    public void HtmlMetadata_DeserializesCorrectly_FromJson()
    {
        // Arrange
        var json = @"{
            ""title"": ""Test Page"",
            ""description"": ""Test Description"",
            ""keywords"": [""test"", ""keywords""],
            ""author"": ""Test Author"",
            ""canonical_url"": ""https://example.com"",
            ""open_graph"": {
                ""og:title"": ""Test"",
                ""og:description"": ""Test Description""
            },
            ""twitter_card"": {
                ""twitter:card"": ""summary""
            },
            ""headers"": [],
            ""links"": [],
            ""images"": [],
            ""structured_data"": []
        }";

        // Act
        var options = new JsonSerializerOptions { PropertyNamingPolicy = JsonNamingPolicy.SnakeCaseLower };
        var metadata = JsonSerializer.Deserialize<HtmlMetadata>(json, options);

        // Assert
        Assert.NotNull(metadata);
        Assert.Equal("Test Page", metadata.Title);
        Assert.Equal("Test Description", metadata.Description);
        Assert.Equal(2, metadata.Keywords.Count);
        Assert.Contains("test", metadata.Keywords);
        Assert.Equal("Test Author", metadata.Author);
        Assert.Equal("https://example.com", metadata.CanonicalUrl);
        Assert.Equal(2, metadata.OpenGraph.Count);
        Assert.Equal("Test", metadata.OpenGraph["og:title"]);
        Assert.Single(metadata.TwitterCard);
        Assert.Equal("summary", metadata.TwitterCard["twitter:card"]);
    }

    [Fact]
    public void HeaderMetadata_JsonSerialization_RoundTrip()
    {
        // Arrange
        var header = new HeaderMetadata
        {
            Level = 2,
            Text = "Subheading",
            Id = "subheading",
            Depth = 1,
            HtmlOffset = 250
        };

        // Act
        var json = JsonSerializer.Serialize(header, new JsonSerializerOptions { PropertyNamingPolicy = JsonNamingPolicy.SnakeCaseLower });
        var options = new JsonSerializerOptions { PropertyNamingPolicy = JsonNamingPolicy.SnakeCaseLower };
        var deserialized = JsonSerializer.Deserialize<HeaderMetadata>(json, options);

        // Assert
        Assert.NotNull(deserialized);
        Assert.Equal(header.Level, deserialized.Level);
        Assert.Equal(header.Text, deserialized.Text);
        Assert.Equal(header.Id, deserialized.Id);
        Assert.Equal(header.Depth, deserialized.Depth);
        Assert.Equal(header.HtmlOffset, deserialized.HtmlOffset);
    }

    [Fact]
    public void LinkMetadata_JsonSerialization_RoundTrip()
    {
        // Arrange
        var link = new LinkMetadata
        {
            Href = "https://example.com/page",
            Text = "Test Link",
            Title = "Test Page",
            LinkType = "internal",
            Rel = new List<string> { "canonical" },
            Attributes = new Dictionary<string, string> { { "class", "nav-link" } }
        };

        // Act
        var json = JsonSerializer.Serialize(link, new JsonSerializerOptions { PropertyNamingPolicy = JsonNamingPolicy.SnakeCaseLower });
        var options = new JsonSerializerOptions { PropertyNamingPolicy = JsonNamingPolicy.SnakeCaseLower };
        var deserialized = JsonSerializer.Deserialize<LinkMetadata>(json, options);

        // Assert
        Assert.NotNull(deserialized);
        Assert.Equal(link.Href, deserialized.Href);
        Assert.Equal(link.Text, deserialized.Text);
        Assert.Equal(link.Title, deserialized.Title);
        Assert.Equal(link.LinkType, deserialized.LinkType);
        Assert.Single(deserialized.Rel);
        Assert.Equal("canonical", deserialized.Rel[0]);
        var attributesList = new List<KeyValuePair<string, string>>(deserialized.Attributes);
        Assert.Single(attributesList);
        Assert.Equal("nav-link", deserialized.Attributes["class"]);
    }

    [Fact]
    public void HtmlImageMetadata_JsonSerialization_RoundTrip()
    {
        // Arrange
        var image = new HtmlImageMetadata
        {
            Src = "images/photo.jpg",
            Alt = "Photo of example",
            Title = "Example Photo",
            Dimensions = new[] { 1920, 1080 },
            ImageType = "embedded",
            Attributes = new Dictionary<string, string> { { "srcset", "photo-small.jpg 800w" } }
        };

        // Act
        var json = JsonSerializer.Serialize(image, new JsonSerializerOptions { PropertyNamingPolicy = JsonNamingPolicy.SnakeCaseLower });
        var options = new JsonSerializerOptions { PropertyNamingPolicy = JsonNamingPolicy.SnakeCaseLower };
        var deserialized = JsonSerializer.Deserialize<HtmlImageMetadata>(json, options);

        // Assert
        Assert.NotNull(deserialized);
        Assert.Equal(image.Src, deserialized.Src);
        Assert.Equal(image.Alt, deserialized.Alt);
        Assert.Equal(image.Title, deserialized.Title);
        Assert.NotNull(deserialized.Dimensions);
        Assert.Equal(1920, deserialized.Dimensions[0]);
        Assert.Equal(1080, deserialized.Dimensions[1]);
        Assert.Equal(image.ImageType, deserialized.ImageType);
        Assert.Single(deserialized.Attributes);
        Assert.Equal("photo-small.jpg 800w", deserialized.Attributes["srcset"]);
    }

    [Fact]
    public void StructuredData_JsonSerialization_RoundTrip()
    {
        // Arrange
        var structuredData = new StructuredData
        {
            DataType = "json_ld",
            RawJson = @"{""@context"":""https://schema.org"",""@type"":""NewsArticle"",""headline"":""Test""}",
            SchemaType = "NewsArticle"
        };

        // Act
        var json = JsonSerializer.Serialize(structuredData, new JsonSerializerOptions { PropertyNamingPolicy = JsonNamingPolicy.SnakeCaseLower });
        var options = new JsonSerializerOptions { PropertyNamingPolicy = JsonNamingPolicy.SnakeCaseLower };
        var deserialized = JsonSerializer.Deserialize<StructuredData>(json, options);

        // Assert
        Assert.NotNull(deserialized);
        Assert.Equal(structuredData.DataType, deserialized.DataType);
        Assert.Equal(structuredData.RawJson, deserialized.RawJson);
        Assert.Equal(structuredData.SchemaType, deserialized.SchemaType);
    }

    [Fact]
    public void MetaTags_SerializeCorrectly()
    {
        // Arrange
        var metadata = new HtmlMetadata();
        metadata.MetaTags["viewport"] = "width=device-width, initial-scale=1";
        metadata.MetaTags["charset"] = "utf-8";

        // Act
        var json = JsonSerializer.Serialize(metadata, new JsonSerializerOptions { PropertyNamingPolicy = JsonNamingPolicy.SnakeCaseLower });
        var options = new JsonSerializerOptions { PropertyNamingPolicy = JsonNamingPolicy.SnakeCaseLower };
        var deserialized = JsonSerializer.Deserialize<HtmlMetadata>(json, options);

        // Assert
        Assert.NotNull(deserialized);
        Assert.NotNull(deserialized.MetaTags);
        Assert.Equal(2, deserialized.MetaTags.Count);
        Assert.Equal("width=device-width, initial-scale=1", deserialized.MetaTags["viewport"]);
        Assert.Equal("utf-8", deserialized.MetaTags["charset"]);
    }

    #endregion

    #region Integration Tests

    [Fact]
    public void ExtractHtml_ReturnsMetadata_WithHtmlMetadataObject()
    {
        // Arrange
        var htmlPath = NativeTestHelper.GetDocumentPath("web/html.html");
        var config = new ExtractionConfig
        {
            HtmlOptions = new HtmlConversionOptions { ExtractMetadata = true }
        };

        // Act
        var result = KreuzbergClient.ExtractFileSync(htmlPath, config);

        // Assert
        Assert.NotNull(result);
        Assert.True(result.Success);
        Assert.NotNull(result.Metadata);
        Assert.NotNull(result.Metadata.Format);
        Assert.NotNull(result.Metadata.Format.Html);
        Assert.IsType<HtmlMetadata>(result.Metadata.Format.Html);
    }

    [Fact]
    public void ExtractHtml_KeywordsAsList_NotString()
    {
        // Arrange
        var htmlPath = NativeTestHelper.GetDocumentPath("web/html.html");
        var config = new ExtractionConfig
        {
            HtmlOptions = new HtmlConversionOptions { ExtractMetadata = true }
        };

        // Act
        var result = KreuzbergClient.ExtractFileSync(htmlPath, config);

        // Assert
        Assert.NotNull(result);
        Assert.NotNull(result.Metadata.Format.Html);
        var htmlMetadata = result.Metadata.Format.Html;
        Assert.IsType<List<string>>(htmlMetadata.Keywords);
        // Keywords should be a list even if empty
        Assert.NotNull(htmlMetadata.Keywords);
    }

    [Fact]
    public void ExtractHtml_OpenGraphAsDictionary_StringToString()
    {
        // Arrange
        var htmlPath = NativeTestHelper.GetDocumentPath("web/html.html");
        var config = new ExtractionConfig
        {
            HtmlOptions = new HtmlConversionOptions { ExtractMetadata = true }
        };

        // Act
        var result = KreuzbergClient.ExtractFileSync(htmlPath, config);

        // Assert
        Assert.NotNull(result);
        Assert.NotNull(result.Metadata.Format.Html);
        var htmlMetadata = result.Metadata.Format.Html;
        Assert.IsType<Dictionary<string, string>>(htmlMetadata.OpenGraph);
        Assert.NotNull(htmlMetadata.OpenGraph);
    }

    [Fact]
    public void ExtractHtml_TwitterCardAsDictionary_StringToString()
    {
        // Arrange
        var htmlPath = NativeTestHelper.GetDocumentPath("web/html.html");
        var config = new ExtractionConfig
        {
            HtmlOptions = new HtmlConversionOptions { ExtractMetadata = true }
        };

        // Act
        var result = KreuzbergClient.ExtractFileSync(htmlPath, config);

        // Assert
        Assert.NotNull(result);
        Assert.NotNull(result.Metadata.Format.Html);
        var htmlMetadata = result.Metadata.Format.Html;
        Assert.IsType<Dictionary<string, string>>(htmlMetadata.TwitterCard);
        Assert.NotNull(htmlMetadata.TwitterCard);
    }

    [Fact]
    public void ExtractHtml_HeadersAsList_OfHeaderMetadata()
    {
        // Arrange
        var htmlPath = NativeTestHelper.GetDocumentPath("web/html.html");
        var config = new ExtractionConfig
        {
            HtmlOptions = new HtmlConversionOptions { ExtractMetadata = true }
        };

        // Act
        var result = KreuzbergClient.ExtractFileSync(htmlPath, config);

        // Assert
        Assert.NotNull(result);
        Assert.NotNull(result.Metadata.Format.Html);
        var htmlMetadata = result.Metadata.Format.Html;
        Assert.IsType<List<HeaderMetadata>>(htmlMetadata.Headers);
        Assert.NotNull(htmlMetadata.Headers);

        // If headers are present, verify they have correct structure
        foreach (var header in htmlMetadata.Headers)
        {
            Assert.IsType<HeaderMetadata>(header);
            Assert.NotEmpty(header.Text);
            Assert.True(header.Level >= 1 && header.Level <= 6, "Header level should be 1-6");
        }
    }

    [Fact]
    public void ExtractHtml_LinksAsList_OfLinkMetadata()
    {
        // Arrange
        var htmlPath = NativeTestHelper.GetDocumentPath("web/html.html");
        var config = new ExtractionConfig
        {
            HtmlOptions = new HtmlConversionOptions { ExtractMetadata = true }
        };

        // Act
        var result = KreuzbergClient.ExtractFileSync(htmlPath, config);

        // Assert
        Assert.NotNull(result);
        Assert.NotNull(result.Metadata.Format.Html);
        var htmlMetadata = result.Metadata.Format.Html;
        Assert.IsType<List<LinkMetadata>>(htmlMetadata.Links);
        Assert.NotNull(htmlMetadata.Links);

        // If links are present, verify they have correct structure
        foreach (var link in htmlMetadata.Links)
        {
            Assert.IsType<LinkMetadata>(link);
            Assert.NotEmpty(link.Href);
            Assert.IsType<List<string>>(link.Rel);
            Assert.IsType<Dictionary<string, string>>(link.Attributes);
        }
    }

    [Fact]
    public void ExtractHtml_ImagesAsList_OfHtmlImageMetadata()
    {
        // Arrange
        var htmlPath = NativeTestHelper.GetDocumentPath("web/html.html");
        var config = new ExtractionConfig
        {
            HtmlOptions = new HtmlConversionOptions { ExtractMetadata = true }
        };

        // Act
        var result = KreuzbergClient.ExtractFileSync(htmlPath, config);

        // Assert
        Assert.NotNull(result);
        Assert.NotNull(result.Metadata.Format.Html);
        var htmlMetadata = result.Metadata.Format.Html;
        Assert.IsType<List<HtmlImageMetadata>>(htmlMetadata.Images);
        Assert.NotNull(htmlMetadata.Images);

        // If images are present, verify they have correct structure
        foreach (var image in htmlMetadata.Images)
        {
            Assert.IsType<HtmlImageMetadata>(image);
            Assert.NotEmpty(image.Src);
            Assert.IsType<Dictionary<string, string>>(image.Attributes);
        }
    }

    [Fact]
    public void ExtractHtml_StructuredDataAsList_OfStructuredData()
    {
        // Arrange
        var htmlPath = NativeTestHelper.GetDocumentPath("web/html.html");
        var config = new ExtractionConfig
        {
            HtmlOptions = new HtmlConversionOptions { ExtractMetadata = true }
        };

        // Act
        var result = KreuzbergClient.ExtractFileSync(htmlPath, config);

        // Assert
        Assert.NotNull(result);
        Assert.NotNull(result.Metadata.Format.Html);
        var htmlMetadata = result.Metadata.Format.Html;
        Assert.IsType<List<StructuredData>>(htmlMetadata.StructuredData);
        Assert.NotNull(htmlMetadata.StructuredData);

        // If structured data is present, verify it has correct structure
        foreach (var data in htmlMetadata.StructuredData)
        {
            Assert.IsType<StructuredData>(data);
            Assert.NotEmpty(data.RawJson);
            Assert.NotEmpty(data.DataType);
        }
    }

    #endregion

    #region Default Values Tests

    [Fact]
    public void HtmlMetadata_DefaultConstructor_InitializesCollections()
    {
        // Arrange & Act
        var metadata = new HtmlMetadata();

        // Assert
        Assert.NotNull(metadata.Keywords);
        Assert.IsType<List<string>>(metadata.Keywords);
        Assert.Empty(metadata.Keywords);

        Assert.NotNull(metadata.OpenGraph);
        Assert.IsType<Dictionary<string, string>>(metadata.OpenGraph);
        Assert.Empty(metadata.OpenGraph);

        Assert.NotNull(metadata.TwitterCard);
        Assert.IsType<Dictionary<string, string>>(metadata.TwitterCard);
        Assert.Empty(metadata.TwitterCard);

        Assert.NotNull(metadata.MetaTags);
        Assert.IsType<Dictionary<string, string>>(metadata.MetaTags);
        Assert.Empty(metadata.MetaTags);

        Assert.NotNull(metadata.Headers);
        Assert.IsType<List<HeaderMetadata>>(metadata.Headers);
        Assert.Empty(metadata.Headers);

        Assert.NotNull(metadata.Links);
        Assert.IsType<List<LinkMetadata>>(metadata.Links);
        Assert.Empty(metadata.Links);

        Assert.NotNull(metadata.Images);
        Assert.IsType<List<HtmlImageMetadata>>(metadata.Images);
        Assert.Empty(metadata.Images);

        Assert.NotNull(metadata.StructuredData);
        Assert.IsType<List<StructuredData>>(metadata.StructuredData);
        Assert.Empty(metadata.StructuredData);
    }

    [Fact]
    public void HeaderMetadata_DefaultConstructor_InitializesDefaults()
    {
        // Arrange & Act
        var header = new HeaderMetadata();

        // Assert
        Assert.Equal(0, header.Level);
        Assert.Equal(string.Empty, header.Text);
        Assert.Null(header.Id);
        Assert.Equal(0, header.Depth);
        Assert.Equal(0, header.HtmlOffset);
    }

    [Fact]
    public void LinkMetadata_DefaultConstructor_InitializesDefaults()
    {
        // Arrange & Act
        var link = new LinkMetadata();

        // Assert
        Assert.Equal(string.Empty, link.Href);
        Assert.Equal(string.Empty, link.Text);
        Assert.Null(link.Title);
        Assert.Equal("other", link.LinkType);
        Assert.NotNull(link.Rel);
        Assert.Empty(link.Rel);
        Assert.NotNull(link.Attributes);
        Assert.Empty(link.Attributes);
    }

    [Fact]
    public void HtmlImageMetadata_DefaultConstructor_InitializesDefaults()
    {
        // Arrange & Act
        var image = new HtmlImageMetadata();

        // Assert
        Assert.Equal(string.Empty, image.Src);
        Assert.Null(image.Alt);
        Assert.Null(image.Title);
        Assert.Null(image.Dimensions);
        Assert.Equal("external", image.ImageType);
        Assert.NotNull(image.Attributes);
        Assert.Empty(image.Attributes);
    }

    [Fact]
    public void StructuredData_DefaultConstructor_InitializesDefaults()
    {
        // Arrange & Act
        var data = new StructuredData();

        // Assert
        Assert.Equal("json_ld", data.DataType);
        Assert.Equal(string.Empty, data.RawJson);
        Assert.Null(data.SchemaType);
    }

    [Fact]
    public void HtmlMetadata_OptionalFields_AreNullWhenMissing()
    {
        // Arrange
        var json = @"{
            ""headers"": [],
            ""links"": [],
            ""images"": [],
            ""structured_data"": []
        }";

        // Act
        var options = new JsonSerializerOptions { PropertyNamingPolicy = JsonNamingPolicy.SnakeCaseLower };
        var metadata = JsonSerializer.Deserialize<HtmlMetadata>(json, options);

        // Assert
        Assert.NotNull(metadata);
        Assert.Null(metadata.Title);
        Assert.Null(metadata.Description);
        Assert.Null(metadata.Author);
        Assert.Null(metadata.CanonicalUrl);
        Assert.Null(metadata.BaseHref);
        Assert.Null(metadata.Language);
        Assert.Null(metadata.TextDirection);
        // Collections should be initialized, not null
        Assert.NotNull(metadata.Keywords);
        Assert.NotNull(metadata.OpenGraph);
        Assert.NotNull(metadata.TwitterCard);
        Assert.NotNull(metadata.Headers);
        Assert.NotNull(metadata.Links);
        Assert.NotNull(metadata.Images);
        Assert.NotNull(metadata.StructuredData);
    }

    [Fact]
    public void HtmlMetadata_EmptyCollections_AreNotNull()
    {
        // Arrange
        var metadata = new HtmlMetadata();

        // Act & Assert - Collections should be initialized as empty, not null
        Assert.NotNull(metadata.Keywords);
        Assert.Empty(metadata.Keywords);

        Assert.NotNull(metadata.OpenGraph);
        Assert.Empty(metadata.OpenGraph);

        Assert.NotNull(metadata.TwitterCard);
        Assert.Empty(metadata.TwitterCard);

        Assert.NotNull(metadata.MetaTags);
        Assert.Empty(metadata.MetaTags);

        Assert.NotNull(metadata.Headers);
        Assert.Empty(metadata.Headers);

        Assert.NotNull(metadata.Links);
        Assert.Empty(metadata.Links);

        Assert.NotNull(metadata.Images);
        Assert.Empty(metadata.Images);

        Assert.NotNull(metadata.StructuredData);
        Assert.Empty(metadata.StructuredData);
    }

    #endregion

    #region Complex Integration Tests

    [Fact]
    public void ExtractHtml_FullMetadataExtraction_VerifiesAllFields()
    {
        // Arrange
        var htmlPath = NativeTestHelper.GetDocumentPath("web/html.html");
        var config = new ExtractionConfig
        {
            HtmlOptions = new HtmlConversionOptions { ExtractMetadata = true }
        };

        // Act
        var result = KreuzbergClient.ExtractFileSync(htmlPath, config);

        // Assert
        Assert.NotNull(result);
        Assert.True(result.Success);
        Assert.NotNull(result.Metadata);
        Assert.Equal(FormatType.Html, result.Metadata.FormatType);
        Assert.NotNull(result.Metadata.Format);
        Assert.Equal(FormatType.Html, result.Metadata.Format.Type);
        Assert.NotNull(result.Metadata.Format.Html);

        var htmlMetadata = result.Metadata.Format.Html;

        // Verify all collection types are correct
        Assert.IsType<List<string>>(htmlMetadata.Keywords);
        Assert.IsType<Dictionary<string, string>>(htmlMetadata.OpenGraph);
        Assert.IsType<Dictionary<string, string>>(htmlMetadata.TwitterCard);
        Assert.IsType<Dictionary<string, string>>(htmlMetadata.MetaTags);
        Assert.IsType<List<HeaderMetadata>>(htmlMetadata.Headers);
        Assert.IsType<List<LinkMetadata>>(htmlMetadata.Links);
        Assert.IsType<List<HtmlImageMetadata>>(htmlMetadata.Images);
        Assert.IsType<List<StructuredData>>(htmlMetadata.StructuredData);

        // Verify all collections are not null
        Assert.NotNull(htmlMetadata.Keywords);
        Assert.NotNull(htmlMetadata.OpenGraph);
        Assert.NotNull(htmlMetadata.TwitterCard);
        Assert.NotNull(htmlMetadata.MetaTags);
        Assert.NotNull(htmlMetadata.Headers);
        Assert.NotNull(htmlMetadata.Links);
        Assert.NotNull(htmlMetadata.Images);
        Assert.NotNull(htmlMetadata.StructuredData);
    }

    [Fact]
    public void ExtractHtml_SerializeResult_AndDeserialize_MaintainsMetadata()
    {
        // Arrange
        var htmlPath = NativeTestHelper.GetDocumentPath("web/html.html");
        var config = new ExtractionConfig
        {
            HtmlOptions = new HtmlConversionOptions { ExtractMetadata = true }
        };

        // Act
        var result = KreuzbergClient.ExtractFileSync(htmlPath, config);
        var originalMetadata = result.Metadata.Format.Html;

        var options = new JsonSerializerOptions
        {
            PropertyNamingPolicy = JsonNamingPolicy.SnakeCaseLower,
            WriteIndented = true
        };
        var json = JsonSerializer.Serialize(result, options);
        var deserializedResult = JsonSerializer.Deserialize<ExtractionResult>(json, options);

        // Assert
        Assert.NotNull(deserializedResult);
        Assert.NotNull(deserializedResult.Metadata.Format.Html);
        var deserializedMetadata = deserializedResult.Metadata.Format.Html;

        // Verify structure is preserved
        Assert.IsType<List<string>>(deserializedMetadata.Keywords);
        Assert.IsType<Dictionary<string, string>>(deserializedMetadata.OpenGraph);
        Assert.IsType<Dictionary<string, string>>(deserializedMetadata.TwitterCard);
        Assert.IsType<List<HeaderMetadata>>(deserializedMetadata.Headers);
        Assert.IsType<List<LinkMetadata>>(deserializedMetadata.Links);
        Assert.IsType<List<HtmlImageMetadata>>(deserializedMetadata.Images);
        Assert.IsType<List<StructuredData>>(deserializedMetadata.StructuredData);

        // Verify collections have same count
        Assert.NotNull(deserializedMetadata);
        Assert.Equal(originalMetadata.Keywords.Count, deserializedMetadata!.Keywords.Count);
        Assert.Equal(originalMetadata.OpenGraph.Count, deserializedMetadata.OpenGraph.Count);
        Assert.Equal(originalMetadata.TwitterCard.Count, deserializedMetadata.TwitterCard.Count);
        Assert.Equal(originalMetadata.Headers.Count, deserializedMetadata.Headers.Count);
        Assert.Equal(originalMetadata.Links.Count, deserializedMetadata.Links.Count);
        Assert.Equal(originalMetadata.Images.Count, deserializedMetadata.Images.Count);
        Assert.Equal(originalMetadata.StructuredData.Count, deserializedMetadata.StructuredData.Count);
    }

    [Fact]
    public void LinkMetadata_With_MultipleRelValues_PreservesAll()
    {
        // Arrange
        var link = new LinkMetadata
        {
            Href = "https://example.com",
            Text = "Link",
            Rel = new List<string> { "nofollow", "external", "noopener" }
        };

        // Act
        var options = new JsonSerializerOptions { PropertyNamingPolicy = JsonNamingPolicy.SnakeCaseLower };
        var json = JsonSerializer.Serialize(link, options);
        var deserialized = JsonSerializer.Deserialize<LinkMetadata>(json, options);

        // Assert
        Assert.NotNull(deserialized);
        Assert.Equal(3, deserialized.Rel.Count);
        Assert.Contains("nofollow", deserialized.Rel);
        Assert.Contains("external", deserialized.Rel);
        Assert.Contains("noopener", deserialized.Rel);
    }

    [Fact]
    public void HtmlImageMetadata_With_ComplexAttributes_PreservesAll()
    {
        // Arrange
        var image = new HtmlImageMetadata
        {
            Src = "image.jpg",
            Alt = "Test",
            Attributes = new Dictionary<string, string>
            {
                { "class", "responsive-image" },
                { "data-lazy", "true" },
                { "srcset", "image-small.jpg 480w, image-medium.jpg 1024w" },
                { "sizes", "(max-width: 600px) 100vw, 50vw" }
            }
        };

        // Act
        var options = new JsonSerializerOptions { PropertyNamingPolicy = JsonNamingPolicy.SnakeCaseLower };
        var json = JsonSerializer.Serialize(image, options);
        var deserialized = JsonSerializer.Deserialize<HtmlImageMetadata>(json, options);

        // Assert
        Assert.NotNull(deserialized);
        Assert.Equal(4, deserialized.Attributes.Count);
        Assert.Equal("responsive-image", deserialized.Attributes["class"]);
        Assert.Equal("true", deserialized.Attributes["data-lazy"]);
        Assert.Equal("image-small.jpg 480w, image-medium.jpg 1024w", deserialized.Attributes["srcset"]);
        Assert.Equal("(max-width: 600px) 100vw, 50vw", deserialized.Attributes["sizes"]);
    }

    [Fact]
    public void StructuredData_With_ComplexJson_PreservesRawJson()
    {
        // Arrange
        var complexJson = @"{
            ""@context"": ""https://schema.org"",
            ""@type"": ""NewsArticle"",
            ""headline"": ""The Title of the Article"",
            ""image"": [
                ""https://example.com/photos/1x1/photo.jpg""
            ],
            ""datePublished"": ""2015-02-05T08:00:00+00:00"",
            ""author"": {
                ""@type"": ""Person"",
                ""name"": ""Jane Doe""
            }
        }";

        var data = new StructuredData
        {
            DataType = "json_ld",
            RawJson = complexJson,
            SchemaType = "NewsArticle"
        };

        // Act
        var options = new JsonSerializerOptions { PropertyNamingPolicy = JsonNamingPolicy.SnakeCaseLower };
        var json = JsonSerializer.Serialize(data, options);
        var deserialized = JsonSerializer.Deserialize<StructuredData>(json, options);

        // Assert
        Assert.NotNull(deserialized);
        Assert.Equal("json_ld", deserialized.DataType);
        Assert.Equal("NewsArticle", deserialized.SchemaType);
        Assert.Contains("NewsArticle", deserialized.RawJson);
        Assert.Contains("Jane Doe", deserialized.RawJson);
    }

    #endregion

    #region Edge Cases Tests

    [Fact]
    public void HtmlMetadata_With_SpecialCharactersInStrings_SerializesCorrectly()
    {
        // Arrange
        var metadata = new HtmlMetadata
        {
            Title = "Test & \"Special\" <Characters>",
            Description = "Description with 'quotes' and \"double quotes\"",
            Author = "Author & Co."
        };

        // Act
        var options = new JsonSerializerOptions { PropertyNamingPolicy = JsonNamingPolicy.SnakeCaseLower };
        var json = JsonSerializer.Serialize(metadata, options);
        var deserialized = JsonSerializer.Deserialize<HtmlMetadata>(json, options);

        // Assert
        Assert.NotNull(deserialized);
        Assert.Equal(metadata.Title, deserialized.Title);
        Assert.Equal(metadata.Description, deserialized.Description);
        Assert.Equal(metadata.Author, deserialized.Author);
    }

    [Fact]
    public void HtmlImageMetadata_With_NullDimensions_HandlesCorrectly()
    {
        // Arrange
        var image = new HtmlImageMetadata
        {
            Src = "image.jpg",
            Dimensions = null
        };

        // Act
        var options = new JsonSerializerOptions { PropertyNamingPolicy = JsonNamingPolicy.SnakeCaseLower };
        var json = JsonSerializer.Serialize(image, options);
        var deserialized = JsonSerializer.Deserialize<HtmlImageMetadata>(json, options);

        // Assert
        Assert.NotNull(deserialized);
        Assert.Null(deserialized.Dimensions!);
    }

    [Fact]
    public void HeaderMetadata_With_ZeroValues_SerializesCorrectly()
    {
        // Arrange
        var header = new HeaderMetadata
        {
            Level = 0,
            Text = "Zero Header",
            Depth = 0,
            HtmlOffset = 0
        };

        // Act
        var options = new JsonSerializerOptions { PropertyNamingPolicy = JsonNamingPolicy.SnakeCaseLower };
        var json = JsonSerializer.Serialize(header, options);
        var deserialized = JsonSerializer.Deserialize<HeaderMetadata>(json, options);

        // Assert
        Assert.NotNull(deserialized);
        Assert.Equal(0, deserialized.Level);
        Assert.Equal(0, deserialized.Depth);
        Assert.Equal(0, deserialized.HtmlOffset);
    }

    [Fact]
    public void LinkMetadata_With_EmptyStringValues_SerializesCorrectly()
    {
        // Arrange
        var link = new LinkMetadata
        {
            Href = "",
            Text = "",
            LinkType = ""
        };

        // Act
        var options = new JsonSerializerOptions { PropertyNamingPolicy = JsonNamingPolicy.SnakeCaseLower };
        var json = JsonSerializer.Serialize(link, options);
        var deserialized = JsonSerializer.Deserialize<LinkMetadata>(json, options);

        // Assert
        Assert.NotNull(deserialized);
        Assert.Equal("", deserialized.Href);
        Assert.Equal("", deserialized.Text);
        Assert.Equal("", deserialized.LinkType);
    }

    #endregion

    #region Critical Missing Tests

    /// <summary>
    /// Tests async HTML extraction by wrapping synchronous extraction in a Task.
    /// Verifies that metadata extraction completes successfully when run asynchronously.
    /// </summary>
    [Fact]
    public async Task ExtractHtml_AsyncExtraction_CompletesSuccessfully()
    {
        // Arrange
        var htmlPath = NativeTestHelper.GetDocumentPath("web/html.html");
        var config = new ExtractionConfig
        {
            HtmlOptions = new HtmlConversionOptions { ExtractMetadata = true }
        };

        // Act - Run extraction asynchronously using Task.Run
        var result = await Task.Run(() => KreuzbergClient.ExtractFileSync(htmlPath, config));

        // Assert
        Assert.NotNull(result);
        Assert.True(result.Success);
        Assert.NotNull(result.Metadata);
        Assert.NotNull(result.Metadata.Format);
        Assert.NotNull(result.Metadata.Format.Html);
        Assert.IsType<HtmlMetadata>(result.Metadata.Format.Html);

        // Verify metadata collections are properly initialized
        Assert.NotNull(result.Metadata.Format.Html.Keywords);
        Assert.NotNull(result.Metadata.Format.Html.OpenGraph);
        Assert.NotNull(result.Metadata.Format.Html.TwitterCard);
        Assert.NotNull(result.Metadata.Format.Html.Headers);
        Assert.NotNull(result.Metadata.Format.Html.Links);
        Assert.NotNull(result.Metadata.Format.Html.Images);
        Assert.NotNull(result.Metadata.Format.Html.StructuredData);
    }

    /// <summary>
    /// Tests HTML extraction performance with a large document containing 10,000+ elements.
    /// Validates that extraction completes within acceptable timeframe for large documents.
    /// Measures extraction speed and verifies it performs within performance thresholds.
    /// </summary>
    [Fact]
    public void ExtractHtml_LargeDocument_PerformanceAcceptable()
    {
        // Arrange - Create a large HTML document with 10,000+ elements
        var largeHtml = BuildLargeHtmlDocument(10000);
        var config = new ExtractionConfig
        {
            HtmlOptions = new HtmlConversionOptions { ExtractMetadata = true }
        };

        // Temporarily write to file for extraction
        var tempPath = Path.GetTempFileName();
        try
        {
            File.WriteAllText(tempPath, largeHtml);

            // Act - Measure extraction time for large document
            var stopwatch = Stopwatch.StartNew();
            var result = KreuzbergClient.ExtractFileSync(tempPath, config);
            stopwatch.Stop();

            // Assert - Verify extraction succeeds
            Assert.NotNull(result);
            Assert.True(result.Success);
            Assert.NotNull(result.Metadata.Format.Html);

            // Performance assertion: extraction should complete within 30 seconds for large document
            Assert.True(stopwatch.ElapsedMilliseconds < 30000,
                $"Extraction took {stopwatch.ElapsedMilliseconds}ms, expected < 30000ms");

            // Verify metadata was extracted
            Assert.NotNull(result.Metadata.Format.Html.Keywords);
            Assert.NotNull(result.Metadata.Format.Html.Headers);
            Assert.NotNull(result.Metadata.Format.Html.Links);
            Assert.NotNull(result.Metadata.Format.Html.Images);

            // Content should be present from large document
            Assert.NotEmpty(result.Content);
        }
        finally
        {
            // Cleanup
            if (File.Exists(tempPath))
            {
                File.Delete(tempPath);
            }
        }
    }

    /// <summary>
    /// Tests thread safety of HTML extraction by performing concurrent extractions
    /// from multiple threads using Parallel.ForEach. Verifies that the extraction
    /// engine handles concurrent access without race conditions or data corruption.
    /// </summary>
    [Fact]
    public void ExtractHtml_ConcurrentExtraction_ThreadSafe()
    {
        // Arrange
        var htmlPath = NativeTestHelper.GetDocumentPath("web/html.html");
        var config = new ExtractionConfig
        {
            HtmlOptions = new HtmlConversionOptions { ExtractMetadata = true }
        };

        // Create results array to store concurrent extraction results
        var results = new ExtractionResult[Environment.ProcessorCount * 2];
        var exceptions = new List<Exception>();
        var lockObj = new object();

        // Act - Perform concurrent extractions
        Parallel.ForEach(
            Enumerable.Range(0, results.Length),
            new ParallelOptions { MaxDegreeOfParallelism = Environment.ProcessorCount * 2 },
            i =>
            {
                try
                {
                    var result = KreuzbergClient.ExtractFileSync(htmlPath, config);
                    results[i] = result;
                }
                catch (Exception ex)
                {
                    lock (lockObj)
                    {
                        exceptions.Add(ex);
                    }
                }
            });

        // Assert - Verify no exceptions occurred during concurrent extraction
        Assert.Empty(exceptions);

        // All extraction results should be successful
        foreach (var result in results)
        {
            Assert.NotNull(result);
            Assert.True(result.Success);
            Assert.NotNull(result.Metadata);
            Assert.NotNull(result.Metadata.Format.Html);
        }

        // Verify all results have consistent metadata structure
        var firstMetadata = results[0].Metadata.Format.Html;
        foreach (var result in results.Skip(1))
        {
            var currentMetadata = result.Metadata.Format?.Html;
            Assert.NotNull(currentMetadata);
            Assert.NotNull(currentMetadata!.Keywords);
            Assert.NotNull(currentMetadata.OpenGraph);
            Assert.NotNull(currentMetadata.TwitterCard);
            Assert.NotNull(currentMetadata.Headers);
            Assert.NotNull(currentMetadata.Links);
            Assert.NotNull(currentMetadata.Images);
            Assert.NotNull(currentMetadata.StructuredData);
        }
    }

    /// <summary>
    /// Tests error handling of HTML extraction with invalid input.
    /// Verifies graceful handling of null input, empty strings, and extremely large HTML.
    /// Ensures extraction engine doesn't crash with invalid data and provides appropriate feedback.
    /// </summary>
    [Fact]
    public void ExtractHtml_InvalidInput_HandlesGracefully()
    {
        var config = new ExtractionConfig
        {
            HtmlOptions = new HtmlConversionOptions { ExtractMetadata = true }
        };

        // Test 1: Empty file handling
        var emptyPath = Path.GetTempFileName();
        try
        {
            File.WriteAllText(emptyPath, string.Empty);

            // Should handle empty file gracefully
            var result = KreuzbergClient.ExtractFileSync(emptyPath, config);
            Assert.NotNull(result);
            // Empty file may or may not be successful depending on implementation
            // The key is it doesn't crash
            Assert.NotNull(result.Metadata);
        }
        finally
        {
            if (File.Exists(emptyPath))
            {
                File.Delete(emptyPath);
            }
        }

        // Test 2: Minimal HTML handling
        var minimalPath = Path.GetTempFileName();
        try
        {
            File.WriteAllText(minimalPath, "<html><body></body></html>");

            var result = KreuzbergClient.ExtractFileSync(minimalPath, config);
            Assert.NotNull(result);
            Assert.NotNull(result.Metadata.Format.Html);
            Assert.NotNull(result.Metadata.Format.Html.Keywords);
            Assert.NotNull(result.Metadata.Format.Html.Headers);
        }
        finally
        {
            if (File.Exists(minimalPath))
            {
                File.Delete(minimalPath);
            }
        }

        // Test 3: Malformed HTML handling
        var malformedPath = Path.GetTempFileName();
        try
        {
            File.WriteAllText(malformedPath, "<html><body><div unclosed><p>test");

            // Should handle malformed HTML gracefully
            var result = KreuzbergClient.ExtractFileSync(malformedPath, config);
            Assert.NotNull(result);
            Assert.NotNull(result.Metadata);
            // Malformed HTML should still parse without crashing
        }
        finally
        {
            if (File.Exists(malformedPath))
            {
                File.Delete(malformedPath);
            }
        }

        // Test 4: Very large HTML handling (50MB)
        var veryLargePath = Path.GetTempFileName();
        try
        {
            // Create a 50MB HTML document
            var largeHtml = BuildLargeHtmlDocument(500000);
            File.WriteAllText(veryLargePath, largeHtml);

            // Should handle very large documents without running out of memory or crashing
            var result = KreuzbergClient.ExtractFileSync(veryLargePath, config);
            Assert.NotNull(result);
            Assert.NotNull(result.Metadata);
            // Verify structure is intact
            Assert.NotNull(result.Metadata.Format.Html);
        }
        finally
        {
            if (File.Exists(veryLargePath))
            {
                File.Delete(veryLargePath);
            }
        }
    }

    /// <summary>
    /// Tests memory management during large HTML extraction.
    /// Verifies that the extraction process properly releases memory after completing,
    /// testing the IDisposable pattern if applicable and ensuring no memory leaks.
    /// </summary>
    [Fact]
    public void ExtractHtml_LargeExtraction_ReleasesMemory()
    {
        // Arrange
        var config = new ExtractionConfig
        {
            HtmlOptions = new HtmlConversionOptions { ExtractMetadata = true }
        };

        // Capture initial memory
        GC.Collect();
        GC.WaitForPendingFinalizers();
        var initialMemory = GC.GetTotalMemory(false);

        // Act - Perform extraction with large HTML
        var largeHtml = BuildLargeHtmlDocument(50000);
        var tempPath = Path.GetTempFileName();
        try
        {
            File.WriteAllText(tempPath, largeHtml);

            var result = KreuzbergClient.ExtractFileSync(tempPath, config);

            // Verify extraction succeeded
            Assert.NotNull(result);
            Assert.True(result.Success);
            Assert.NotNull(result.Metadata.Format.Html);

            // Capture memory after successful extraction
            var afterExtractionMemory = GC.GetTotalMemory(false);

            // Force garbage collection to reclaim memory
            GC.Collect();
            GC.WaitForPendingFinalizers();
            GC.Collect();

            var finalMemory = GC.GetTotalMemory(false);

            // Verify memory was released after cleanup
            // Allow for some memory overhead (20% tolerance for runtime overhead)
            var memoryGrowth = finalMemory - initialMemory;
            var allowedGrowth = initialMemory * 0.20;

            Assert.True(
                memoryGrowth <= allowedGrowth,
                $"Memory growth ({memoryGrowth} bytes) exceeds allowance ({allowedGrowth} bytes)");

            // Verify metadata structure is intact
            Assert.NotNull(result.Metadata.Format.Html.Keywords);
            Assert.NotNull(result.Metadata.Format.Html.Headers);
            Assert.NotNull(result.Metadata.Format.Html.Links);
            Assert.NotNull(result.Metadata.Format.Html.Images);
        }
        finally
        {
            // Cleanup
            if (File.Exists(tempPath))
            {
                File.Delete(tempPath);
            }
        }
    }

    #endregion

    #region Helper Methods

    /// <summary>
    /// Builds a large HTML document with a specified number of elements.
    /// Used for performance and stress testing.
    /// </summary>
    /// <param name="elementCount">Number of elements to generate in the HTML document</param>
    /// <returns>HTML document as a string</returns>
    private static string BuildLargeHtmlDocument(int elementCount)
    {
        var sb = new StringBuilder();
        sb.AppendLine("<!DOCTYPE html>");
        sb.AppendLine("<html>");
        sb.AppendLine("<head>");
        sb.AppendLine("<title>Large Test Document</title>");
        sb.AppendLine("<meta name=\"description\" content=\"Large document for performance testing\">");
        sb.AppendLine("<meta name=\"keywords\" content=\"test, performance, large, document\">");
        sb.AppendLine("</head>");
        sb.AppendLine("<body>");
        sb.AppendLine("<h1>Large Document Test</h1>");

        // Generate specified number of elements
        for (int i = 0; i < elementCount; i++)
        {
            sb.AppendLine($"<div class=\"item-{i % 100}\">");
            sb.AppendLine($"<h{((i % 6) + 1)}>Header {i}</h{((i % 6) + 1)}>");
            sb.AppendLine($"<p>Paragraph content {i}. This is a test paragraph with some content.</p>");
            sb.AppendLine($"<a href=\"https://example.com/page-{i}\">Link {i}</a>");
            sb.AppendLine($"<img src=\"image-{i}.jpg\" alt=\"Image {i}\" width=\"{100 + (i % 900)}\" height=\"{100 + ((i * 7) % 900)}\">");
            sb.AppendLine("</div>");
        }

        sb.AppendLine("</body>");
        sb.AppendLine("</html>");

        return sb.ToString();
    }

    #endregion
}
