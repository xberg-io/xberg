using Xberg;

var config = new ExtractionConfig
{
    UseCache = true,
    EnableQualityProcessing = true
};

var cts = new System.Threading.CancellationTokenSource(TimeSpan.FromSeconds(30));
var result = await XbergLib.ExtractAsync("document.pdf", config, cts.Token);
