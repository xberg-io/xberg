using System;
using System.Collections.Generic;
using System.Threading.Tasks;
using Xunit;

namespace Kreuzberg.Tests;

/// <summary>
/// Tests verifying that deprecation markers are properly applied to C# bindings.
/// </summary>
public class DeprecationTests
{
    [Fact]
    public async Task DeprecatedExtractWithOcr_GeneratesWarning()
    {
        // This test verifies that [Obsolete] attribute is applied
        // Compilation with warnings-as-errors would fail here without suppression

        var input = new byte[] { 0x25, 0x50, 0x44, 0x46 }; // PDF magic bytes
        var mimeType = "application/pdf";

#pragma warning disable CS0618
        // This should compile but generate CS0618 warning
        // var result = await LegacyExtractionAPI.ExtractAsyncWithOcr(input, mimeType, enableOcr: true);
        // We can't actually call it without the full FFI implementation
#pragma warning restore CS0618

        // Verify the attribute is defined
        var method = typeof(LegacyExtractionAPI).GetMethod("ExtractAsyncWithOcr");
        Assert.NotNull(method);

        var obsoleteAttr = method?.GetCustomAttributes(typeof(ObsoleteAttribute), false);
        Assert.NotNull(obsoleteAttr);
        Assert.Single(obsoleteAttr);
    }

    [Fact]
    public void DeprecatedConfigModel_HasObsoleteAttributes()
    {
        // Verify [Obsolete] attributes are applied to deprecated properties
        var model = typeof(DeprecatedConfigurationModel);

        var ocrBackendProp = model.GetProperty("OcrBackend");
        var ocrBackendAttrs = ocrBackendProp?.GetCustomAttributes(typeof(ObsoleteAttribute), false);
        Assert.NotNull(ocrBackendAttrs);
        Assert.Single(ocrBackendAttrs);

        var enableOcrProp = model.GetProperty("EnableOcr");
        var enableOcrAttrs = enableOcrProp?.GetCustomAttributes(typeof(ObsoleteAttribute), false);
        Assert.NotNull(enableOcrAttrs);
        Assert.Single(enableOcrAttrs);

        var ocrLanguageProp = model.GetProperty("OcrLanguage");
        var ocrLanguageAttrs = ocrLanguageProp?.GetCustomAttributes(typeof(ObsoleteAttribute), false);
        Assert.NotNull(ocrLanguageAttrs);
        Assert.Single(ocrLanguageAttrs);
    }

    [Fact]
    public void DeprecatedExtensions_HaveObsoleteAttributes()
    {
        // Verify extension methods are marked as obsolete
        var withQualityProcessing = typeof(DeprecatedExtensions).GetMethod("WithQualityProcessing");
        Assert.NotNull(withQualityProcessing);

        var attrs = withQualityProcessing?.GetCustomAttributes(typeof(ObsoleteAttribute), false);
        Assert.NotNull(attrs);
        Assert.Single(attrs);

        var obsoleteAttr = attrs?[0] as ObsoleteAttribute;
        Assert.NotNull(obsoleteAttr);
        Assert.Contains("ExtractionConfig.EnableQualityProcessing", obsoleteAttr?.Message);
    }

    [Fact]
    public void DeprecatedValidationLogic_HasObsoleteAttribute()
    {
        // Verify validation function is marked as obsolete
        var method = typeof(DeprecatedValidationLogic).GetMethod("IsOcrEnabledDeprecated");
        Assert.NotNull(method);

        var attrs = method?.GetCustomAttributes(typeof(ObsoleteAttribute), false);
        Assert.NotNull(attrs);
        Assert.Single(attrs);

        var obsoleteAttr = attrs?[0] as ObsoleteAttribute;
        Assert.NotNull(obsoleteAttr);
        Assert.Contains("deprecated", obsoleteAttr?.Message?.ToLower());
    }

    [Fact]
    public void ObsoleteAttribute_ContainsMigrationGuidance()
    {
        // Verify that obsolete attributes include helpful migration guidance
        var method = typeof(LegacyExtractionAPI).GetMethod("ExtractAsyncWithOcr");
        var attrs = method?.GetCustomAttributes(typeof(ObsoleteAttribute), false);
        var obsoleteAttr = attrs?[0] as ObsoleteAttribute;

        Assert.NotNull(obsoleteAttr);
        Assert.NotNull(obsoleteAttr?.Message);

        // Should mention the alternative approach
        Assert.True(
            obsoleteAttr!.Message.Contains("ExtractAsyncWithConfig") ||
            obsoleteAttr.Message.Contains("ExtractionConfig"),
            "Obsolete message should guide users to the new API"
        );
    }

    [Fact]
    public void ObsoleteAttribute_SpecifiesRemovalVersion()
    {
        // Verify that obsolete attributes specify when code will be removed
        var method = typeof(LegacyExtractionAPI).GetMethod("ExtractAsyncWithOcr");
        var attrs = method?.GetCustomAttributes(typeof(ObsoleteAttribute), false);
        var obsoleteAttr = attrs?[0] as ObsoleteAttribute;

        Assert.NotNull(obsoleteAttr?.Message);
        Assert.Contains("v2.0.0", obsoleteAttr!.Message);
    }

    [Theory]
    [InlineData("WithQualityProcessing")]
    [InlineData("WithOcrBackend")]
    public void AllDeprecatedExtensions_AreMarked(string methodName)
    {
        var method = typeof(DeprecatedExtensions).GetMethod(methodName);
        Assert.NotNull(method);

        var attrs = method?.GetCustomAttributes(typeof(ObsoleteAttribute), false);
        Assert.NotNull(attrs);
        Assert.NotEmpty(attrs!);
    }
}
