using Xberg;
using System.Text.Json;

// NOTE: IDocumentExtractor interface is not available in C# bindings

class CustomJsonProcessor
{
    public static ExtractedDocument ProcessJson(byte[] content, string mimeType)
    {
        try
        {
            var jsonContent = System.Text.Encoding.UTF8.GetString(content);
            var document = JsonDocument.Parse(jsonContent);

            var text = ExtractText(document.RootElement);

            return new ExtractedDocument
            {
                Content = text,
                MimeType = mimeType,
                Metadata = new Metadata(),
                Tables = new List<Table>(),
                Success = true
            };
        }
        catch (JsonException ex)
        {
            throw new XbergParsingException($"Failed to parse JSON: {ex.Message}");
        }
    }

    private static string ExtractText(JsonElement element)
    {
        return element.ValueKind switch
        {
            JsonValueKind.String => element.GetString() + "\n",
            JsonValueKind.Array => string.Concat(
                element.EnumerateArray().Select(ExtractText)
            ),
            JsonValueKind.Object => string.Concat(
                element.EnumerateObject()
                    .Select(p => ExtractText(p.Value))
            ),
            _ => ""
        };
    }
}

class Program
{
    static void Main()
    {
        try
        {
            var jsonData = new { message = "Hello, world!", timestamp = DateTime.UtcNow };
            var jsonBytes = System.Text.Encoding.UTF8.GetBytes(
                JsonSerializer.Serialize(jsonData)
            );

            var result = CustomJsonProcessor.ProcessJson(jsonBytes, "application/json");

            Console.WriteLine($"Extracted: {result.Content}");
            Console.WriteLine($"MIME type: {result.MimeType}");
        }
        catch (XbergException ex)
        {
            Console.WriteLine($"Error: {ex.Message}");
        }
    }
}
