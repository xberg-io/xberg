```csharp title="C#"
using System.Net.Http;
using System.Net.Http.Json;

var client = new HttpClient();

using (var fileStream = File.OpenRead("document.pdf"))
{
    using (var content = new MultipartFormDataContent())
    {
        content.Add(new StreamContent(fileStream), "files", "document.pdf");
        var response = await client.PostAsync("http://localhost:8000/extract", content);

        if (response.IsSuccessStatusCode)
        {
            var json = await response.Content.ReadAsStringAsync();
            Console.WriteLine(json);
        }
        else
        {
            Console.WriteLine($"Error: {response.StatusCode}");
        }
    }
}
```
