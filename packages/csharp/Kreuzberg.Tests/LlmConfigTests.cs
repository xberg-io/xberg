using System.Text.Json;
using Xunit;

namespace Kreuzberg.Tests;

/// <summary>
/// Round-trip tests for <see cref="LlmConfig"/> and the <c>OcrConfig.VlmConfig</c>
/// kwarg added for issue #965. Verifies the JSON shape matches the Rust core
/// snake_case schema and that the source-generated context registers the type.
/// </summary>
public class LlmConfigTests
{
    private static readonly JsonSerializerOptions Options = new()
    {
        TypeInfoResolver = KreuzbergJsonContext.DefaultContext,
    };

    [Fact]
    public void LlmConfig_RoundTrips_WithAllFields()
    {
        var config = new LlmConfig
        {
            Model = "openai/gpt-4o",
            ApiKey = "sk-test",
            BaseUrl = "https://api.example.com",
            TimeoutSecs = 90,
            MaxRetries = 5,
            Temperature = 0.2,
            MaxTokens = 2048,
        };

        var json = JsonSerializer.Serialize(config, Options);
        var roundTripped = JsonSerializer.Deserialize<LlmConfig>(json, Options);

        Assert.NotNull(roundTripped);
        Assert.Equal(config.Model, roundTripped!.Model);
        Assert.Equal(config.ApiKey, roundTripped.ApiKey);
        Assert.Equal(config.BaseUrl, roundTripped.BaseUrl);
        Assert.Equal(config.TimeoutSecs, roundTripped.TimeoutSecs);
        Assert.Equal(config.MaxRetries, roundTripped.MaxRetries);
        Assert.Equal(config.Temperature, roundTripped.Temperature);
        Assert.Equal(config.MaxTokens, roundTripped.MaxTokens);
    }

    [Fact]
    public void LlmConfig_UsesSnakeCaseJsonKeys_MatchingRustCore()
    {
        var config = new LlmConfig
        {
            Model = "anthropic/claude-sonnet-4-20250514",
            ApiKey = "sk-ant-test",
            BaseUrl = "https://example",
            TimeoutSecs = 30,
            MaxRetries = 1,
            Temperature = 0.7,
            MaxTokens = 1024,
        };

        var json = JsonSerializer.Serialize(config, Options);

        Assert.Contains("\"model\"", json);
        Assert.Contains("\"api_key\"", json);
        Assert.Contains("\"base_url\"", json);
        Assert.Contains("\"timeout_secs\"", json);
        Assert.Contains("\"max_retries\"", json);
        Assert.Contains("\"temperature\"", json);
        Assert.Contains("\"max_tokens\"", json);
    }

    [Fact]
    public void OcrConfig_VlmConfig_RoundTripsThroughJson()
    {
        var ocr = new OcrConfig
        {
            Backend = "vlm",
            VlmConfig = new LlmConfig { Model = "openai/gpt-4o-mini" },
        };

        var json = JsonSerializer.Serialize(ocr, Options);
        Assert.Contains("\"vlm_config\"", json);

        var roundTripped = JsonSerializer.Deserialize<OcrConfig>(json, Options);
        Assert.NotNull(roundTripped);
        Assert.Equal("vlm", roundTripped!.Backend);
        Assert.NotNull(roundTripped.VlmConfig);
        Assert.Equal("openai/gpt-4o-mini", roundTripped.VlmConfig!.Model);
    }

    [Fact]
    public void OcrConfig_VlmConfig_DeserializesFromRustShapedJson()
    {
        // Matches what the Rust core emits for OcrConfig { vlm_config: Some(...) }.
        const string rustJson = """
            {
                "backend": "vlm",
                "vlm_config": {
                    "model": "openai/gpt-4o",
                    "api_key": "sk-test",
                    "max_tokens": 4096
                }
            }
            """;

        var ocr = JsonSerializer.Deserialize<OcrConfig>(rustJson, Options);

        Assert.NotNull(ocr);
        Assert.Equal("vlm", ocr!.Backend);
        Assert.NotNull(ocr.VlmConfig);
        Assert.Equal("openai/gpt-4o", ocr.VlmConfig!.Model);
        Assert.Equal("sk-test", ocr.VlmConfig.ApiKey);
        Assert.Equal(4096UL, ocr.VlmConfig.MaxTokens);
    }
}
