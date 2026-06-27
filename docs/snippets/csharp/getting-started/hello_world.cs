using Xberg;

var result = XbergClient.ExtractSync("document.pdf");

Console.WriteLine(result.Content);
