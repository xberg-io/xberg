using Kreuzberg;
using System.Collections.Generic;

class CustomCacheBackend
{
    private Dictionary<string, ExtractionResult> _cache = new();

    public async Task<ExtractionResult> GetOrExtractAsync(
        string filePath,
        ExtractionConfig config)
    {
        var cacheKey = GenerateCacheKey(filePath, config);

        if (_cache.TryGetValue(cacheKey, out var cachedResult))
        {
            Console.WriteLine("Using cached result");
            return cachedResult;
        }

        var result = await KreuzbergClient.ExtractFileAsync(filePath, config);

        _cache[cacheKey] = result;
        Console.WriteLine("Result cached");

        return result;
    }

    private string GenerateCacheKey(string filePath, ExtractionConfig config)
    {
        var configHash = config.ToString().GetHashCode();
        return $"{filePath}:{configHash}";
    }

    public void ClearCache()
    {
        _cache.Clear();
        Console.WriteLine("Cache cleared");
    }
}

class Program
{
    static async Task Main()
    {
        var cacheBackend = new CustomCacheBackend();
        var config = new ExtractionConfig { UseCache = true };

        try
        {
            var result1 = await cacheBackend.GetOrExtractAsync("document.pdf", config);
            Console.WriteLine($"Result 1: {result1.Content.Length} chars");

            var result2 = await cacheBackend.GetOrExtractAsync("document.pdf", config);
            Console.WriteLine($"Result 2: {result2.Content.Length} chars");

            cacheBackend.ClearCache();
        }
        catch (KreuzbergException ex)
        {
            Console.WriteLine($"Error: {ex.Message}");
        }
    }
}
