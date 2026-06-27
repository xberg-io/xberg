using Xberg;

var version = XbergClient.GetVersion();
Console.WriteLine($"Xberg version: {version}");

var result = XbergClient.Extract("document.pdf");
Console.WriteLine($"Extraction successful: {result.Success}");
