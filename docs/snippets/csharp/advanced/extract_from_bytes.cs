using Kreuzberg;

class Program
{
    static async Task Main()
    {
        try
        {
            var pdfBytes = await File.ReadAllBytesAsync("document.pdf");

            var result = await KreuzbergClient.ExtractBytesAsync(
                pdfBytes,
                "application/pdf"
            );

            Console.WriteLine($"Content: {result.Content}");
            Console.WriteLine($"MIME type: {result.MimeType}");

            var config = new ExtractionConfig
            {
                UseCache = true,
                EnableQualityProcessing = true
            };

            var result2 = await KreuzbergClient.ExtractBytesAsync(
                pdfBytes,
                "application/pdf",
                config
            );

            Console.WriteLine($"Configured extraction: {result2.Content.Length} chars");

            var imageBytes = new byte[] {  };

            var imageResult = await KreuzbergClient.ExtractBytesAsync(
                imageBytes,
                "image/jpeg"
            );

            Console.WriteLine($"Image text: {imageResult.Content}");

            var multipleFiles = new Dictionary<string, (byte[], string)>
            {
                { "file1", (await File.ReadAllBytesAsync("file1.pdf"), "application/pdf") },
                { "file2", (await File.ReadAllBytesAsync("file2.pdf"), "application/pdf") }
            };

            foreach (var (name, (bytes, mimeType)) in multipleFiles)
            {
                var extractResult = await KreuzbergClient.ExtractBytesAsync(
                    bytes,
                    mimeType
                );
                Console.WriteLine($"{name}: {extractResult.Content.Length} chars");
            }
        }
        catch (KreuzbergException ex)
        {
            Console.WriteLine($"Extraction error: {ex.Message}");
        }
        catch (IOException ex)
        {
            Console.WriteLine($"File I/O error: {ex.Message}");
        }
    }
}
