```csharp title="C#"
using Kreuzberg;

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

    public void Validate(ExtractionResult result, ExtractionConfig config)
    {
        var qualityScore = CalculateQualityScore(result);
        
        if (qualityScore < MinimumQuality)
        {
            throw new KreuzbergException(
                $"Quality score {qualityScore:F2} below minimum {MinimumQuality}",
                1003
            );
        }
    }

    public bool ShouldValidate(ExtractionResult result, ExtractionConfig config)
    {
        return !string.IsNullOrEmpty(result.Content);
    }

    public int Priority()
    {
        return 50;
    }

    private float CalculateQualityScore(ExtractionResult result)
    {
        var contentLength = result.Content.Length;
        var hasMetadata = result.Metadata != null;
        
        var score = (contentLength > 100 ? 0.8f : 0.5f) + (hasMetadata ? 0.2f : 0.0f);
        return Math.Min(score, 1.0f);
    }
}

var validator = new QualityScoreValidator();
ValidatorRegistry.Register(validator);
```
