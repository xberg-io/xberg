using Kreuzberg;

class Program
{
    static async Task Main()
    {
        try
        {
            var result = await KreuzbergLib.ExtractFileAsync("document.pdf");

            Console.WriteLine($"Content length: {result.Content.Length}");
            Console.WriteLine($"MIME type: {result.MimeType}");

            var tasks = new[]
            {
                KreuzbergLib.ExtractFileAsync("file1.pdf"),
                KreuzbergLib.ExtractFileAsync("file2.pdf"),
                KreuzbergLib.ExtractFileAsync("file3.pdf")
            };

            var results = await Task.WhenAll(tasks);

            foreach (var r in results)
            {
                Console.WriteLine($"Extracted {r.Content.Length} characters");
            }
        }
        catch (KreuzbergException ex)
        {
            Console.WriteLine($"Extraction failed: {ex.Message}");
        }
    }
}
