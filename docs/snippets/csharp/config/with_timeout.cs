using Kreuzberg;

var config = new ExtractionConfig
{
    UseCache = true,
    EnableQualityProcessing = true
};

var cts = new System.Threading.CancellationTokenSource(TimeSpan.FromSeconds(30));
var result = await KreuzbergLib.ExtractFileAsync("document.pdf", config, cts.Token);
