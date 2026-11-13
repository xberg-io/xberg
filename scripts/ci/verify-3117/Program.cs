using System;
using System.Diagnostics;
using System.IO;
using System.Linq;
using System.Text.Json;
using System.Threading.Tasks;
using Xberg;

var config =
    ExtractionConfig.Default() with { UseCache = false, DisableOcr = true };
await ExtractHello(args[0]);

using var process = Process.GetCurrentProcess();
process.Refresh();
long before = process.PeakPagedMemorySize64;
var document = await ExtractHello(args[1]);
process.Refresh();
long after = process.PeakPagedMemorySize64;
Console.WriteLine(JsonSerializer.Serialize(new {
  fixture = Path.GetFileName(args[1]),
  beforeBytes = before,
  afterBytes = after,
  deltaBytes = after - before,
  resultCount = 1,
  content = document.Content.Trim(),
}));

async Task<ExtractedDocument> ExtractHello(string path) {
  var result = await XbergConverter.ExtractAsync(
      new ExtractInput { Kind = ExtractInputKind.Uri,
                         Uri = Path.GetFullPath(path) },
      config);
  var document = result.Results.Single();
  if (result.Errors.Any() || document.Content.Trim() != "Hello")
    throw new InvalidOperationException(
        $"Unexpected extraction result for {path}");
  return document;
}
