using Xberg;

var data = File.ReadAllBytes("document.pdf");
var result = XbergClient.Extract(data, "application/pdf");

Console.WriteLine(result.Content);
Console.WriteLine($"Success: {result.Success}");
Console.WriteLine($"Content Length: {result.Content.Length}");
