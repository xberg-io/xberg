using Xberg;

var result = XbergClient.Extract("document.pdf");

Console.WriteLine(result.Content);
