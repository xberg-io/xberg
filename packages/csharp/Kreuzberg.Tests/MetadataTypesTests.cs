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
        var metadata = new HtmlMetadata();

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
        var metadata = new HtmlMetadata();

        metadata.Keywords.Add("test");
        metadata.Keywords.Add("keywords");

        Assert.IsType<List<string>>(metadata.Keywords);
        Assert.Equal(2, metadata.Keywords.Count);
        Assert.Contains("test", metadata.Keywords);
        Assert.Contains("keywords", metadata.Keywords);
    }

    [Fact]
    public void CanonicalUrl_Renamed_PropertyExists()
    {
        var metadata = new HtmlMetadata();

        metadata.CanonicalUrl = "https://example.com/canonical";

        Assert.Equal("https://example.com/canonical", metadata.CanonicalUrl);
        Assert.NotNull(metadata.CanonicalUrl);
    }

    [Fact]
    public void OpenGraph_IsDictionary_StringToString()
    {
        var metadata = new HtmlMetadata();

        metadata.OpenGraph["og:title"] = "Test Title";
        metadata.OpenGraph["og:description"] = "Test Description";
        metadata.OpenGraph["og:image"] = "https://example.com/image.jpg";

        Assert.IsType<Dictionary<string, string>>(metadata.OpenGraph);
        Assert.Equal(3, metadata.OpenGraph.Count);
        Assert.Equal("Test Title", metadata.OpenGraph["og:title"]);
        Assert.Equal("Test Description", metadata.OpenGraph["og:description"]);
        Assert.Equal("https://example.com/image.jpg", metadata.OpenGraph["og:image"]);
    }

    [Fact]
    public void TwitterCard_IsDictionary_StringToString()
    {
        var metadata = new HtmlMetadata();

        metadata.TwitterCard["twitter:card"] = "summary_large_image";
        metadata.TwitterCard["twitter:title"] = "Test Title";
        metadata.TwitterCard["twitter:description"] = "Test Description";

        Assert.IsType<Dictionary<string, string>>(metadata.TwitterCard);
        Assert.Equal(3, metadata.TwitterCard.Count);
        Assert.Equal("summary_large_image", metadata.TwitterCard["twitter:card"]);
        Assert.Equal("Test Title", metadata.TwitterCard["twitter:title"]);
    }

    [Fact]
    public void HeaderMetadata_HasCorrectProperties()
    {
        var header = new HeaderMetadata
        {
            Level = 1,
            Text = "Main Title",
            Id = "main-title",
            Depth = 0,
            HtmlOffset = 100
        };

        Assert.Equal(1, header.Level);
        Assert.Equal("Main Title", header.Text);
        Assert.Equal("main-title", header.Id);
        Assert.Equal(0, header.Depth);
        Assert.Equal(100, header.HtmlOffset);
    }

    [Fact]
    public void LinkMetadata_HasCorrectProperties()
    {
        var link = new LinkMetadata
        {
            Href = "https://example.com",
            Text = "Example Link",
            Title = "Example Website",
            LinkType = "external",
            Rel = new List<string> { "nofollow", "external" }
        };

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
        var link = new LinkMetadata { Href = "https://example.com" };

        link.Attributes["class"] = "external-link";
        link.Attributes["data-tracking"] = "123";

        Assert.IsType<Dictionary<string, string>>(link.Attributes);
        Assert.Equal(2, link.Attributes.Count);
        Assert.Equal("external-link", link.Attributes["class"]);
    }

    [Fact]
    public void HtmlImageMetadata_HasCorrectProperties()
    {
        var image = new HtmlImageMetadata
        {
            Src = "https://example.com/image.jpg",
            Alt = "Example image",
            Title = "Example",
            Dimensions = new[] { 800, 600 },
            ImageType = "external"
        };

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
        var image = new HtmlImageMetadata { Src = "image.jpg" };

        image.Attributes["loading"] = "lazy";
        image.Attributes["data-src"] = "image-hd.jpg";

        Assert.IsType<Dictionary<string, string>>(image.Attributes);
        Assert.Equal(2, image.Attributes.Count);
        Assert.Equal("lazy", image.Attributes["loading"]);
    }

    [Fact]
    public void StructuredData_HasCorrectProperties()
    {
        var structuredData = new StructuredData
        {
            DataType = "json_ld",
            RawJson = @"{""@context"": ""https://schema.org"", ""@type"": ""Article""}",
            SchemaType = "Article"
        };

        Assert.Equal("json_ld", structuredData.DataType);
        Assert.NotEmpty(structuredData.RawJson);
        Assert.Equal("Article", structuredData.SchemaType);
    }

    #endregion

    #region JSON Serialization Tests

    [Fact]
    public void HtmlMetadata_SerializesCorrectly_WithJsonPropertyNames()
    {
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

        var json = JsonSerializer.Serialize(metadata, new JsonSerializerOptions { PropertyNamingPolicy = JsonNamingPolicy.SnakeCaseLower });

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

        var options = new JsonSerializerOptions { PropertyNamingPolicy = JsonNamingPolicy.SnakeCaseLower };
        var metadata = JsonSerializer.Deserialize<HtmlMetadata>(json, options);

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
        var header = new HeaderMetadata
        {
            Level = 2,
            Text = "Subheading",
            Id = "subheading",
            Depth = 1,
            HtmlOffset = 250
        };

        var json = JsonSerializer.Serialize(header, new JsonSerializerOptions { PropertyNamingPolicy = JsonNamingPolicy.SnakeCaseLower });
        var options = new JsonSerializerOptions { PropertyNamingPolicy = JsonNamingPolicy.SnakeCaseLower };
        var deserialized = JsonSerializer.Deserialize<HeaderMetadata>(json, options);

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
        var link = new LinkMetadata
        {
            Href = "https://example.com/page",
            Text = "Test Link",
            Title = "Test Page",
            LinkType = "internal",
            Rel = new List<string> { "canonical" },
            Attributes = new Dictionary<string, string> { { "class", "nav-link" } }
        };

        var json = JsonSerializer.Serialize(link, new JsonSerializerOptions { PropertyNamingPolicy = JsonNamingPolicy.SnakeCaseLower });
        var options = new JsonSerializerOptions { PropertyNamingPolicy = JsonNamingPolicy.SnakeCaseLower };
        var deserialized = JsonSerializer.Deserialize<LinkMetadata>(json, options);

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
        var image = new HtmlImageMetadata
        {
            Src = "images/photo.jpg",
            Alt = "Photo of example",
            Title = "Example Photo",
            Dimensions = new[] { 1920, 1080 },
            ImageType = "embedded",
            Attributes = new Dictionary<string, string> { { "srcset", "photo-small.jpg 800w" } }
        };

        var json = JsonSerializer.Serialize(image, new JsonSerializerOptions { PropertyNamingPolicy = JsonNamingPolicy.SnakeCaseLower });
        var options = new JsonSerializerOptions { PropertyNamingPolicy = JsonNamingPolicy.SnakeCaseLower };
        var deserialized = JsonSerializer.Deserialize<HtmlImageMetadata>(json, options);

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
        var structuredData = new StructuredData
        {
            DataType = "json_ld",
            RawJson = @"{""@context"":""https://schema.org"",""@type"":""NewsArticle"",""headline"":""Test""}",
            SchemaType = "NewsArticle"
        };

        var json = JsonSerializer.Serialize(structuredData, new JsonSerializerOptions { PropertyNamingPolicy = JsonNamingPolicy.SnakeCaseLower });
        var options = new JsonSerializerOptions { PropertyNamingPolicy = JsonNamingPolicy.SnakeCaseLower };
        var deserialized = JsonSerializer.Deserialize<StructuredData>(json, options);

        Assert.NotNull(deserialized);
        Assert.Equal(structuredData.DataType, deserialized.DataType);
        Assert.Equal(structuredData.RawJson, deserialized.RawJson);
        Assert.Equal(structuredData.SchemaType, deserialized.SchemaType);
    }

    [Fact]
    public void MetaTags_SerializeCorrectly()
    {
        var metadata = new HtmlMetadata();
        metadata.MetaTags["viewport"] = "width=device-width, initial-scale=1";
        metadata.MetaTags["charset"] = "utf-8";

        var json = JsonSerializer.Serialize(metadata, new JsonSerializerOptions { PropertyNamingPolicy = JsonNamingPolicy.SnakeCaseLower });
        var options = new JsonSerializerOptions { PropertyNamingPolicy = JsonNamingPolicy.SnakeCaseLower };
        var deserialized = JsonSerializer.Deserialize<HtmlMetadata>(json, options);

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
        var htmlPath = NativeTestHelper.GetDocumentPath("web/html.html");
        var config = new ExtractionConfig
        {
            HtmlOptions = new HtmlConversionOptions { ExtractMetadata = true }
        };

        var result = KreuzbergClient.ExtractFileSync(htmlPath, config);

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
        var htmlPath = NativeTestHelper.GetDocumentPath("web/html.html");
        var config = new ExtractionConfig
        {
            HtmlOptions = new HtmlConversionOptions { ExtractMetadata = true }
        };

        var result = KreuzbergClient.ExtractFileSync(htmlPath, config);

        Assert.NotNull(result);
        Assert.NotNull(result.Metadata.Format.Html);
        var htmlMetadata = result.Metadata.Format.Html;
        Assert.IsType<List<string>>(htmlMetadata.Keywords);
        Assert.NotNull(htmlMetadata.Keywords);
    }

    [Fact]
    public void ExtractHtml_OpenGraphAsDictionary_StringToString()
    {
        var htmlPath = NativeTestHelper.GetDocumentPath("web/html.html");
        var config = new ExtractionConfig
        {
            HtmlOptions = new HtmlConversionOptions { ExtractMetadata = true }
        };

        var result = KreuzbergClient.ExtractFileSync(htmlPath, config);

        Assert.NotNull(result);
        Assert.NotNull(result.Metadata.Format.Html);
        var htmlMetadata = result.Metadata.Format.Html;
        Assert.IsType<Dictionary<string, string>>(htmlMetadata.OpenGraph);
        Assert.NotNull(htmlMetadata.OpenGraph);
    }

    [Fact]
    public void ExtractHtml_TwitterCardAsDictionary_StringToString()
    {
        var htmlPath = NativeTestHelper.GetDocumentPath("web/html.html");
        var config = new ExtractionConfig
        {
            HtmlOptions = new HtmlConversionOptions { ExtractMetadata = true }
        };

        var result = KreuzbergClient.ExtractFileSync(htmlPath, config);

        Assert.NotNull(result);
        Assert.NotNull(result.Metadata.Format.Html);
        var htmlMetadata = result.Metadata.Format.Html;
        Assert.IsType<Dictionary<string, string>>(htmlMetadata.TwitterCard);
        Assert.NotNull(htmlMetadata.TwitterCard);
    }

    [Fact]
    public void ExtractHtml_HeadersAsList_OfHeaderMetadata()
    {
        var htmlPath = NativeTestHelper.GetDocumentPath("web/html.html");
        var config = new ExtractionConfig
        {
            HtmlOptions = new HtmlConversionOptions { ExtractMetadata = true }
        };

        var result = KreuzbergClient.ExtractFileSync(htmlPath, config);

        Assert.NotNull(result);
        Assert.NotNull(result.Metadata.Format.Html);
        var htmlMetadata = result.Metadata.Format.Html;
        Assert.IsType<List<HeaderMetadata>>(htmlMetadata.Headers);
        Assert.NotNull(htmlMetadata.Headers);

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
        var htmlPath = NativeTestHelper.GetDocumentPath("web/html.html");
        var config = new ExtractionConfig
        {
            HtmlOptions = new HtmlConversionOptions { ExtractMetadata = true }
        };

        var result = KreuzbergClient.ExtractFileSync(htmlPath, config);

        Assert.NotNull(result);
        Assert.NotNull(result.Metadata.Format.Html);
        var htmlMetadata = result.Metadata.Format.Html;
        Assert.IsType<List<LinkMetadata>>(htmlMetadata.Links);
        Assert.NotNull(htmlMetadata.Links);

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
        var htmlPath = NativeTestHelper.GetDocumentPath("web/html.html");
        var config = new ExtractionConfig
        {
            HtmlOptions = new HtmlConversionOptions { ExtractMetadata = true }
        };

        var result = KreuzbergClient.ExtractFileSync(htmlPath, config);

        Assert.NotNull(result);
        Assert.NotNull(result.Metadata.Format.Html);
        var htmlMetadata = result.Metadata.Format.Html;
        Assert.IsType<List<HtmlImageMetadata>>(htmlMetadata.Images);
        Assert.NotNull(htmlMetadata.Images);

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
        var htmlPath = NativeTestHelper.GetDocumentPath("web/html.html");
        var config = new ExtractionConfig
        {
            HtmlOptions = new HtmlConversionOptions { ExtractMetadata = true }
        };

        var result = KreuzbergClient.ExtractFileSync(htmlPath, config);

        Assert.NotNull(result);
        Assert.NotNull(result.Metadata.Format.Html);
        var htmlMetadata = result.Metadata.Format.Html;
        Assert.IsType<List<StructuredData>>(htmlMetadata.StructuredData);
        Assert.NotNull(htmlMetadata.StructuredData);

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
        var metadata = new HtmlMetadata();

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
        var header = new HeaderMetadata();

        Assert.Equal(0, header.Level);
        Assert.Equal(string.Empty, header.Text);
        Assert.Null(header.Id);
        Assert.Equal(0, header.Depth);
        Assert.Equal(0, header.HtmlOffset);
    }

    [Fact]
    public void LinkMetadata_DefaultConstructor_InitializesDefaults()
    {
        var link = new LinkMetadata();

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
        var image = new HtmlImageMetadata();

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
        var data = new StructuredData();

        Assert.Equal("json_ld", data.DataType);
        Assert.Equal(string.Empty, data.RawJson);
        Assert.Null(data.SchemaType);
    }

    [Fact]
    public void HtmlMetadata_OptionalFields_AreNullWhenMissing()
    {
        var json = @"{
            ""headers"": [],
            ""links"": [],
            ""images"": [],
            ""structured_data"": []
        }";

        var options = new JsonSerializerOptions { PropertyNamingPolicy = JsonNamingPolicy.SnakeCaseLower };
        var metadata = JsonSerializer.Deserialize<HtmlMetadata>(json, options);

        Assert.NotNull(metadata);
        Assert.Null(metadata.Title);
        Assert.Null(metadata.Description);
        Assert.Null(metadata.Author);
        Assert.Null(metadata.CanonicalUrl);
        Assert.Null(metadata.BaseHref);
        Assert.Null(metadata.Language);
        Assert.Null(metadata.TextDirection);
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
        var metadata = new HtmlMetadata();

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
        var htmlPath = NativeTestHelper.GetDocumentPath("web/html.html");
        var config = new ExtractionConfig
        {
            HtmlOptions = new HtmlConversionOptions { ExtractMetadata = true }
        };

        var result = KreuzbergClient.ExtractFileSync(htmlPath, config);

        Assert.NotNull(result);
        Assert.True(result.Success);
        Assert.NotNull(result.Metadata);
        Assert.Equal(FormatType.Html, result.Metadata.FormatType);
        Assert.NotNull(result.Metadata.Format);
        Assert.Equal(FormatType.Html, result.Metadata.Format.Type);
        Assert.NotNull(result.Metadata.Format.Html);

        var htmlMetadata = result.Metadata.Format.Html;

        Assert.IsType<List<string>>(htmlMetadata.Keywords);
        Assert.IsType<Dictionary<string, string>>(htmlMetadata.OpenGraph);
        Assert.IsType<Dictionary<string, string>>(htmlMetadata.TwitterCard);
        Assert.IsType<Dictionary<string, string>>(htmlMetadata.MetaTags);
        Assert.IsType<List<HeaderMetadata>>(htmlMetadata.Headers);
        Assert.IsType<List<LinkMetadata>>(htmlMetadata.Links);
        Assert.IsType<List<HtmlImageMetadata>>(htmlMetadata.Images);
        Assert.IsType<List<StructuredData>>(htmlMetadata.StructuredData);

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
        var htmlPath = NativeTestHelper.GetDocumentPath("web/html.html");
        var config = new ExtractionConfig
        {
            HtmlOptions = new HtmlConversionOptions { ExtractMetadata = true }
        };

        var result = KreuzbergClient.ExtractFileSync(htmlPath, config);
        var originalMetadata = result.Metadata.Format.Html;

        var options = new JsonSerializerOptions
        {
            PropertyNamingPolicy = JsonNamingPolicy.SnakeCaseLower,
            WriteIndented = true
        };
        var json = JsonSerializer.Serialize(result, options);
        var deserializedResult = JsonSerializer.Deserialize<ExtractionResult>(json, options);

        Assert.NotNull(deserializedResult);
        Assert.NotNull(deserializedResult.Metadata.Format.Html);
        var deserializedMetadata = deserializedResult.Metadata.Format.Html;

        Assert.IsType<List<string>>(deserializedMetadata.Keywords);
        Assert.IsType<Dictionary<string, string>>(deserializedMetadata.OpenGraph);
        Assert.IsType<Dictionary<string, string>>(deserializedMetadata.TwitterCard);
        Assert.IsType<List<HeaderMetadata>>(deserializedMetadata.Headers);
        Assert.IsType<List<LinkMetadata>>(deserializedMetadata.Links);
        Assert.IsType<List<HtmlImageMetadata>>(deserializedMetadata.Images);
        Assert.IsType<List<StructuredData>>(deserializedMetadata.StructuredData);

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
        var link = new LinkMetadata
        {
            Href = "https://example.com",
            Text = "Link",
            Rel = new List<string> { "nofollow", "external", "noopener" }
        };

        var options = new JsonSerializerOptions { PropertyNamingPolicy = JsonNamingPolicy.SnakeCaseLower };
        var json = JsonSerializer.Serialize(link, options);
        var deserialized = JsonSerializer.Deserialize<LinkMetadata>(json, options);

        Assert.NotNull(deserialized);
        Assert.Equal(3, deserialized.Rel.Count);
        Assert.Contains("nofollow", deserialized.Rel);
        Assert.Contains("external", deserialized.Rel);
        Assert.Contains("noopener", deserialized.Rel);
    }

    [Fact]
    public void HtmlImageMetadata_With_ComplexAttributes_PreservesAll()
    {
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

        var options = new JsonSerializerOptions { PropertyNamingPolicy = JsonNamingPolicy.SnakeCaseLower };
        var json = JsonSerializer.Serialize(image, options);
        var deserialized = JsonSerializer.Deserialize<HtmlImageMetadata>(json, options);

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

        var options = new JsonSerializerOptions { PropertyNamingPolicy = JsonNamingPolicy.SnakeCaseLower };
        var json = JsonSerializer.Serialize(data, options);
        var deserialized = JsonSerializer.Deserialize<StructuredData>(json, options);

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
        var metadata = new HtmlMetadata
        {
            Title = "Test & \"Special\" <Characters>",
            Description = "Description with 'quotes' and \"double quotes\"",
            Author = "Author & Co."
        };

        var options = new JsonSerializerOptions { PropertyNamingPolicy = JsonNamingPolicy.SnakeCaseLower };
        var json = JsonSerializer.Serialize(metadata, options);
        var deserialized = JsonSerializer.Deserialize<HtmlMetadata>(json, options);

        Assert.NotNull(deserialized);
        Assert.Equal(metadata.Title, deserialized.Title);
        Assert.Equal(metadata.Description, deserialized.Description);
        Assert.Equal(metadata.Author, deserialized.Author);
    }

    [Fact]
    public void HtmlImageMetadata_With_NullDimensions_HandlesCorrectly()
    {
        var image = new HtmlImageMetadata
        {
            Src = "image.jpg",
            Dimensions = null
        };

        var options = new JsonSerializerOptions { PropertyNamingPolicy = JsonNamingPolicy.SnakeCaseLower };
        var json = JsonSerializer.Serialize(image, options);
        var deserialized = JsonSerializer.Deserialize<HtmlImageMetadata>(json, options);

        Assert.NotNull(deserialized);
        Assert.Null(deserialized.Dimensions!);
    }

    [Fact]
    public void HeaderMetadata_With_ZeroValues_SerializesCorrectly()
    {
        var header = new HeaderMetadata
        {
            Level = 0,
            Text = "Zero Header",
            Depth = 0,
            HtmlOffset = 0
        };

        var options = new JsonSerializerOptions { PropertyNamingPolicy = JsonNamingPolicy.SnakeCaseLower };
        var json = JsonSerializer.Serialize(header, options);
        var deserialized = JsonSerializer.Deserialize<HeaderMetadata>(json, options);

        Assert.NotNull(deserialized);
        Assert.Equal(0, deserialized.Level);
        Assert.Equal(0, deserialized.Depth);
        Assert.Equal(0, deserialized.HtmlOffset);
    }

    [Fact]
    public void LinkMetadata_With_EmptyStringValues_SerializesCorrectly()
    {
        var link = new LinkMetadata
        {
            Href = "",
            Text = "",
            LinkType = ""
        };

        var options = new JsonSerializerOptions { PropertyNamingPolicy = JsonNamingPolicy.SnakeCaseLower };
        var json = JsonSerializer.Serialize(link, options);
        var deserialized = JsonSerializer.Deserialize<LinkMetadata>(json, options);

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
        var htmlPath = NativeTestHelper.GetDocumentPath("web/html.html");
        var config = new ExtractionConfig
        {
            HtmlOptions = new HtmlConversionOptions { ExtractMetadata = true }
        };

        var result = await Task.Run(() => KreuzbergClient.ExtractFileSync(htmlPath, config));

        Assert.NotNull(result);
        Assert.True(result.Success);
        Assert.NotNull(result.Metadata);
        Assert.NotNull(result.Metadata.Format);
        Assert.NotNull(result.Metadata.Format.Html);
        Assert.IsType<HtmlMetadata>(result.Metadata.Format.Html);

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
        var largeHtml = BuildLargeHtmlDocument(10000);
        var config = new ExtractionConfig
        {
            HtmlOptions = new HtmlConversionOptions { ExtractMetadata = true }
        };

        var tempPath = Path.GetTempFileName();
        try
        {
            File.WriteAllText(tempPath, largeHtml);

            var stopwatch = Stopwatch.StartNew();
            var result = KreuzbergClient.ExtractFileSync(tempPath, config);
            stopwatch.Stop();

            Assert.NotNull(result);
            Assert.True(result.Success);
            Assert.NotNull(result.Metadata.Format.Html);

            Assert.True(stopwatch.ElapsedMilliseconds < 30000,
                $"Extraction took {stopwatch.ElapsedMilliseconds}ms, expected < 30000ms");

            Assert.NotNull(result.Metadata.Format.Html.Keywords);
            Assert.NotNull(result.Metadata.Format.Html.Headers);
            Assert.NotNull(result.Metadata.Format.Html.Links);
            Assert.NotNull(result.Metadata.Format.Html.Images);

            Assert.NotEmpty(result.Content);
        }
        finally
        {
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
        var htmlPath = NativeTestHelper.GetDocumentPath("web/html.html");
        var config = new ExtractionConfig
        {
            HtmlOptions = new HtmlConversionOptions { ExtractMetadata = true }
        };

        var results = new ExtractionResult[Environment.ProcessorCount * 2];
        var exceptions = new List<Exception>();
        var lockObj = new object();

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

        Assert.Empty(exceptions);

        foreach (var result in results)
        {
            Assert.NotNull(result);
            Assert.True(result.Success);
            Assert.NotNull(result.Metadata);
            Assert.NotNull(result.Metadata.Format.Html);
        }

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

        var emptyPath = Path.GetTempFileName();
        try
        {
            File.WriteAllText(emptyPath, string.Empty);

            var result = KreuzbergClient.ExtractFileSync(emptyPath, config);
            Assert.NotNull(result);
            Assert.NotNull(result.Metadata);
        }
        finally
        {
            if (File.Exists(emptyPath))
            {
                File.Delete(emptyPath);
            }
        }

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

        var malformedPath = Path.GetTempFileName();
        try
        {
            File.WriteAllText(malformedPath, "<html><body><div unclosed><p>test");

            var result = KreuzbergClient.ExtractFileSync(malformedPath, config);
            Assert.NotNull(result);
            Assert.NotNull(result.Metadata);
        }
        finally
        {
            if (File.Exists(malformedPath))
            {
                File.Delete(malformedPath);
            }
        }

        var veryLargePath = Path.GetTempFileName();
        try
        {
            var largeHtml = BuildLargeHtmlDocument(500000);
            File.WriteAllText(veryLargePath, largeHtml);

            var result = KreuzbergClient.ExtractFileSync(veryLargePath, config);
            Assert.NotNull(result);
            Assert.NotNull(result.Metadata);
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
        var config = new ExtractionConfig
        {
            HtmlOptions = new HtmlConversionOptions { ExtractMetadata = true }
        };

        GC.Collect();
        GC.WaitForPendingFinalizers();
        var initialMemory = GC.GetTotalMemory(false);

        var largeHtml = BuildLargeHtmlDocument(50000);
        var tempPath = Path.GetTempFileName();
        try
        {
            File.WriteAllText(tempPath, largeHtml);

            var result = KreuzbergClient.ExtractFileSync(tempPath, config);

            Assert.NotNull(result);
            Assert.True(result.Success);
            Assert.NotNull(result.Metadata.Format.Html);

            var afterExtractionMemory = GC.GetTotalMemory(false);

            GC.Collect();
            GC.WaitForPendingFinalizers();
            GC.Collect();

            var finalMemory = GC.GetTotalMemory(false);

            var memoryGrowth = finalMemory - initialMemory;
            var allowedGrowth = initialMemory * 0.20;

            Assert.True(
                memoryGrowth <= allowedGrowth,
                $"Memory growth ({memoryGrowth} bytes) exceeds allowance ({allowedGrowth} bytes)");

            Assert.NotNull(result.Metadata.Format.Html.Keywords);
            Assert.NotNull(result.Metadata.Format.Html.Headers);
            Assert.NotNull(result.Metadata.Format.Html.Links);
            Assert.NotNull(result.Metadata.Format.Html.Images);
        }
        finally
        {
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
