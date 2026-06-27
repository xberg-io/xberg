```csharp title="C#"
using Xberg;

public class QualityScoreValidator : IValidator
{
    private const float MinimumQuality = 0.7f;

    public string Name => "quality-score-validator";
    public string Version => "1.0.0";

    public void Initialize()
    {
        Console.WriteLine($"Quality score validator initialized (min score: {MinimumQuality})");
    }

    public void Shutdown()
    {
        Console.WriteLine("Quality score validator shut down");
    }

    public void Validate(ExtractedDocument result, ExtractionConfig config)
    {
        var qualityScore = CalculateQualityScore(result);

        if (qualityScore < MinimumQuality)
        {
            throw new XbergException(
                $"Quality score {qualityScore:F2} below minimum {MinimumQuality}",
                1003
            );
        }
    }

    public bool ShouldValidate(ExtractedDocument result, ExtractionConfig config)
    {
        return !string.IsNullOrEmpty(result.Content);
    }

    public int Priority()
    {
        return 50;
    }

    private float CalculateQualityScore(ExtractedDocument result)
    {
        var contentLength = result.Content.Length;
        var hasMetadata = result.Metadata != null;

        var score = (contentLength > 100 ? 0.8f : 0.5f) + (hasMetadata ? 0.2f : 0.0f);
        return Math.Min(score, 1.0f);
    }
}

class Program
{
    static void Main()
    {
        var validator = new QualityScoreValidator();
        ValidatorRegistry.Register(validator);
    }
}
```
