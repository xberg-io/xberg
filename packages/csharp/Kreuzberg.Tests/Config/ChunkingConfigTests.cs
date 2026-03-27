using System;
using System.Collections.Generic;
using System.Text.Json;
using Xunit;

namespace Kreuzberg.Tests.Config;

/// <summary>
/// Comprehensive configuration tests for ChunkingConfig.
/// Tests text splitting strategies, chunk sizes, and embedding configuration.
/// </summary>
public class ChunkingConfigTests
{
    [Fact]
    public void Constructor_ShouldCreateWithDefaults()
    {
        var config = new ChunkingConfig();

        Assert.Null(config.MaxChars);
        Assert.Null(config.MaxOverlap);
        Assert.Null(config.Preset);
        Assert.Null(config.Enabled);
        Assert.Null(config.Embedding);
    }

    [Fact]
    public void Constructor_ShouldCreateWithCustomValues()
    {
        var config = new ChunkingConfig
        {
            MaxChars = 1000,
            MaxOverlap = 100,
            ChunkSize = 512,
            ChunkOverlap = 50,
            Enabled = true
        };

        Assert.Equal(1000, config.MaxChars);
        Assert.Equal(100, config.MaxOverlap);
        Assert.Equal(512, config.ChunkSize);
        Assert.Equal(50, config.ChunkOverlap);
        Assert.True(config.Enabled);
    }

    [Fact]
    public void Serialize_ShouldRoundTrip()
    {
        var original = new ChunkingConfig
        {
            MaxChars = 2000,
            MaxOverlap = 200,
            Preset = "semantic",
            Enabled = true
        };

        var json = JsonSerializer.Serialize(original);
        var restored = JsonSerializer.Deserialize<ChunkingConfig>(json);

        Assert.NotNull(restored);
        Assert.Equal(original.MaxChars, restored.MaxChars);
        Assert.Equal(original.MaxOverlap, restored.MaxOverlap);
        Assert.Equal(original.Preset, restored.Preset);
        Assert.Equal(original.Enabled, restored.Enabled);
    }

    [Fact]
    public void Serialize_ShouldIncludeEmbedding()
    {
        var embedding = new EmbeddingConfig
        {
            Model = "text-embedding-ada-002",
            Dimensions = 1536,
            BatchSize = 32,
            Normalize = true
        };

        var config = new ChunkingConfig
        {
            MaxChars = 1500,
            Embedding = embedding
        };

        var json = JsonSerializer.Serialize(config);
        var restored = JsonSerializer.Deserialize<ChunkingConfig>(json);

        Assert.NotNull(restored?.Embedding);
        Assert.Equal("text-embedding-ada-002", restored.Embedding.Model);
        Assert.Equal(1536, restored.Embedding.Dimensions);
    }

    [Theory]
    [InlineData("default")]
    [InlineData("semantic")]
    [InlineData("token-based")]
    public void Preset_ShouldAcceptValidValues(string preset)
    {
        var config = new ChunkingConfig { Preset = preset };

        Assert.Equal(preset, config.Preset);
    }

    [Fact]
    public void MaxChars_ShouldAcceptValidValues()
    {
        var config1 = new ChunkingConfig { MaxChars = 256 };
        var config2 = new ChunkingConfig { MaxChars = 4096 };
        var config3 = new ChunkingConfig { MaxChars = 16384 };

        Assert.Equal(256, config1.MaxChars);
        Assert.Equal(4096, config2.MaxChars);
        Assert.Equal(16384, config3.MaxChars);
    }

    [Fact]
    public void MaxOverlap_ShouldAcceptValidValues()
    {
        var config1 = new ChunkingConfig { MaxOverlap = 0 };
        var config2 = new ChunkingConfig { MaxOverlap = 100 };
        var config3 = new ChunkingConfig { MaxOverlap = 500 };

        Assert.Equal(0, config1.MaxOverlap);
        Assert.Equal(100, config2.MaxOverlap);
        Assert.Equal(500, config3.MaxOverlap);
    }

    [Fact]
    public void Nesting_ShouldWorkInExtractionConfig()
    {
        var chunkingConfig = new ChunkingConfig
        {
            MaxChars = 2000,
            MaxOverlap = 200,
            Enabled = true
        };
        var extractionConfig = new ExtractionConfig { Chunking = chunkingConfig };

        Assert.Equal(2000, extractionConfig.Chunking?.MaxChars);
        Assert.Equal(200, extractionConfig.Chunking?.MaxOverlap);
        Assert.True(extractionConfig.Chunking?.Enabled);
    }

    [Fact]
    public void AllPropertiesAreInitOnly()
    {
        var properties = typeof(ChunkingConfig)
            .GetProperties(System.Reflection.BindingFlags.Public | System.Reflection.BindingFlags.Instance)
            .Where(p => p.SetMethod != null)
            .ToList();

        Assert.True(properties.Count > 0, "ChunkingConfig should have at least one settable property");

        foreach (var prop in properties)
        {
            var hasInitOnly = prop.SetMethod.ReturnParameter?
                .GetRequiredCustomModifiers()
                .Any(m => m.Name == "IsExternalInit") ?? false;

            Assert.True(hasInitOnly, $"{prop.Name} must have init-only accessor");
        }
    }

    [Fact]
    public void NullHandling_ShouldHandleAllNullableFields()
    {
        var config = new ChunkingConfig
        {
            MaxChars = null,
            MaxOverlap = null,
            Preset = null,
            Enabled = null,
            Embedding = null
        };

        Assert.Null(config.MaxChars);
        Assert.Null(config.MaxOverlap);
        Assert.Null(config.Preset);
        Assert.Null(config.Enabled);
        Assert.Null(config.Embedding);
    }

    [Fact]
    public void EmbeddingConfig_ShouldSupportComplexValues()
    {
        var embedding = new EmbeddingConfig
        {
            Model = "text-embedding-ada-002",
            Dimensions = 1536,
            BatchSize = 100,
            Normalize = true,
            UseCache = true
        };

        var config = new ChunkingConfig { Embedding = embedding };

        Assert.NotNull(config.Embedding);
        Assert.Equal("text-embedding-ada-002", config.Embedding.Model);
        Assert.Equal(1536, config.Embedding.Dimensions);
        Assert.Equal(100, config.Embedding.BatchSize);
        Assert.True(config.Embedding.Normalize);
        Assert.True(config.Embedding.UseCache);
    }

    [Fact]
    public void ChunkSize_AlternativeToMaxChars()
    {
        var config1 = new ChunkingConfig { MaxChars = 1000 };
        var config2 = new ChunkingConfig { ChunkSize = 1000 };

        Assert.Equal(1000, config1.MaxChars);
        Assert.Equal(1000, config2.ChunkSize);
    }

    [Fact]
    public void ChunkOverlap_AlternativeToMaxOverlap()
    {
        var config1 = new ChunkingConfig { MaxOverlap = 100 };
        var config2 = new ChunkingConfig { ChunkOverlap = 100 };

        Assert.Equal(100, config1.MaxOverlap);
        Assert.Equal(100, config2.ChunkOverlap);
    }

    [Theory]
    [InlineData("text")]
    [InlineData("markdown")]
    [InlineData("yaml")]
    public void ChunkerType_ShouldAcceptValidValues(string chunkerType)
    {
        var config = new ChunkingConfig { ChunkerType = chunkerType };

        Assert.Equal(chunkerType, config.ChunkerType);
    }

    [Fact]
    public void ChunkerType_ShouldBeNullByDefault()
    {
        var config = new ChunkingConfig();

        Assert.Null(config.ChunkerType);
    }

    [Fact]
    public void ChunkerType_ShouldRoundTripThroughJson()
    {
        var original = new ChunkingConfig { ChunkerType = "markdown" };

        var json = JsonSerializer.Serialize(original);
        var restored = JsonSerializer.Deserialize<ChunkingConfig>(json);

        Assert.NotNull(restored);
        Assert.Equal("markdown", restored.ChunkerType);
        Assert.Contains("chunker_type", json);
    }
}
