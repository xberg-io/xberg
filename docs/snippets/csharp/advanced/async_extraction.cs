using Xberg;

class Program
{
    static async Task Main()
    {
        try
        {
            var result = await XbergLib.ExtractAsync("document.pdf");

            Console.WriteLine($"Content length: {result.Content.Length}");
            Console.WriteLine($"MIME type: {result.MimeType}");

            var tasks = new[]
            {
                XbergLib.ExtractAsync("file1.pdf"),
                XbergLib.ExtractAsync("file2.pdf"),
                XbergLib.ExtractAsync("file3.pdf")
            };

            var results = await Task.WhenAll(tasks);

            foreach (var r in results)
            {
                Console.WriteLine($"Extracted {r.Content.Length} characters");
            }
        }
        catch (XbergException ex)
        {
            Console.WriteLine($"Extraction failed: {ex.Message}");
        }
    }
}
