```csharp title="simple_benchmark.cs"
using BenchmarkDotNet.Attributes;
using BenchmarkDotNet.Running;
using Xberg;
using System;
using System.Diagnostics;
using System.Threading.Tasks;

[MemoryDiagnoser]
[SimpleJob(warmupCount: 3, targetCount: 5)]
public class XbergBenchmark
{
    private string _testFilePath;
    private ExtractionConfig _config;

    [GlobalSetup]
    public void Setup()
    {
        _testFilePath = "document.pdf";
        _config = new ExtractionConfig
        {
            UseCache = false,
            EnableQualityProcessing = true,
        };
    }

    [Benchmark]
    public void ExtractSync()
    {
        var result = XbergLib.ExtractSync(_testFilePath, _config);
        _ = result.Content.Length;
    }

    [Benchmark]
    public async Task ExtractAsync()
    {
        var result = await XbergLib.ExtractAsync(_testFilePath, _config);
        _ = result.Content.Length;
    }

    [Benchmark]
    public async Task ExtractWithOcr()
    {
        var ocrConfig = new ExtractionConfig
        {
            ForceOcr = true,
            Ocr = new OcrConfig
            {
                Backend = "tesseract",
                Language = "eng",
            }
        };

        var result = await XbergLib.ExtractAsync(_testFilePath, ocrConfig);
        _ = result.Content.Length;
    }

    [Benchmark]
    public async Task ExtractWithCache()
    {
        var cacheConfig = new ExtractionConfig
        {
            UseCache = true,
            EnableQualityProcessing = true,
        };

        var result = await XbergLib.ExtractAsync(_testFilePath, cacheConfig);
        _ = result.Content.Length;
    }
}

public class ManualBenchmark
{
    public static async Task Main(string[] args)
    {
        var filePath = "document.pdf";
        var config = new ExtractionConfig();

        await XbergLib.ExtractAsync(filePath, config);

        var sw = Stopwatch.StartNew();
        for (int i = 0; i < 10; i++)
        {
            XbergLib.ExtractSync(filePath, config);
        }
        sw.Stop();
        Console.WriteLine($"Sync extraction (10 runs): {sw.ElapsedMilliseconds}ms avg {sw.ElapsedMilliseconds / 10f}ms");

        sw.Restart();
        var tasks = new System.Collections.Generic.List<Task>();
        for (int i = 0; i < 10; i++)
        {
            tasks.Add(XbergLib.ExtractAsync(filePath, config));
        }
        await Task.WhenAll(tasks);
        sw.Stop();
        Console.WriteLine($"Async extraction (10 parallel runs): {sw.ElapsedMilliseconds}ms");

        var summary = BenchmarkRunner.Run<XbergBenchmark>();
    }
}
```
