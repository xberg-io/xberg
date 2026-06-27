using Xberg;
using System.Collections.Generic;

class Program
{
    static void Main()
    {
        try
        {
            var extractors = XbergLib.ListDocumentExtractors();
            Console.WriteLine("Registered Document Extractors:");
            foreach (var extractor in extractors)
            {
                Console.WriteLine($"  - {extractor}");
            }

            var ocrBackends = XbergLib.ListOcrBackends();
            Console.WriteLine("\nRegistered OCR Backends:");
            foreach (var backend in ocrBackends)
            {
                Console.WriteLine($"  - {backend}");
            }

            var processors = XbergLib.ListPostProcessors();
            Console.WriteLine("\nRegistered Post-Processors:");
            foreach (var processor in processors)
            {
                Console.WriteLine($"  - {processor}");
            }

            var validators = XbergLib.ListValidators();
            Console.WriteLine("\nRegistered Validators:");
            foreach (var validator in validators)
            {
                Console.WriteLine($"  - {validator}");
            }

            var customProcessor = new CustomPostProcessor();
            XbergLib.RegisterPostProcessor(customProcessor);
            Console.WriteLine($"\nRegistered custom post-processor: {customProcessor.Name}");

            XbergLib.UnregisterPostProcessor(customProcessor.Name);
            Console.WriteLine($"Unregistered post-processor: {customProcessor.Name}");

            XbergLib.ClearValidators();
            Console.WriteLine("All validators cleared");
        }
        catch (XbergException ex)
        {
            Console.WriteLine($"Plugin registry error: {ex.Message}");
        }
    }
}

class CustomPostProcessor : IPostProcessor
{
    public string Name => "custom-processor";
    public int Priority => 50;

    public ExtractedDocument Process(ExtractedDocument result)
    {
        result.Content = result.Content.ToUpper();
        return result;
    }
}
