```csharp title="disk_cache.cs"
using Xberg;
using System;
using System.IO;
using System.Threading.Tasks;

var config = new ExtractionConfig
{
    UseCache = true,
    CacheConfig = new CacheConfig
    {
        CachePath = Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData), "xberg_cache"),
        MaxCacheSize = 1024 * 1024 * 500, 
        CacheTtlSeconds = 86400 * 7,      
        EnableCompression = true
    }
};

Console.WriteLine("First extraction (will be cached)...");
var result1 = await XbergLib.ExtractAsync("document.pdf", config);
Console.WriteLine($"  - Content length: {result1.Content.Length}");
Console.WriteLine($"  - Cached: {result1.Metadata.WasCached}");

Console.WriteLine("\nSecond extraction (from cache)...");
var result2 = await XbergLib.ExtractAsync("document.pdf", config);
Console.WriteLine($"  - Content length: {result2.Content.Length}");
Console.WriteLine($"  - Cached: {result2.Metadata.WasCached}");

Console.WriteLine($"\nResults are identical: {result1.Content == result2.Content}");

await XbergLib.ClearCacheAsync("document.pdf");
Console.WriteLine("\nCache cleared for document.pdf");

await XbergLib.ClearAllCacheAsync();
Console.WriteLine("All cache cleared");

var cacheStats = await XbergLib.GetCacheStatsAsync();
Console.WriteLine($"\nCache Statistics:");
Console.WriteLine($"  - Total entries: {cacheStats.TotalEntries}");
Console.WriteLine($"  - Cache size: {cacheStats.CacheSizeBytes / 1024 / 1024} MB");
Console.WriteLine($"  - Hit rate: {cacheStats.HitRate:P}");
```
