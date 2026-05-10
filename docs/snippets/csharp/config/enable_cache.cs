using Kreuzberg;

var config = new ExtractionConfig
{
    UseCache = true
};

var result = KreuzbergLib.ExtractFileSync("document.pdf", config);
