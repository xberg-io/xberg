using System.Text;
using System.Text.Json;
using Xunit;

namespace Xberg;

public class BytesTransportTests {
  [Fact]
  public async Task BytesWithoutMimeHintPreserveFilenameAndInputOverrides() {
    const string source = "# Transport probe";
    var input = new ExtractInput {
      Kind = ExtractInputKind.Bytes,
      Bytes = Encoding.UTF8.GetBytes(source),
      Filename = "probe.md",
      Config =
          new FileExtractionConfig { OutputFormat = new OutputFormat.Plain() },
    };
    var config =
        new ExtractionConfig { UseCache = false,
                               OutputFormat = new OutputFormat.Markdown() };
    var result = await XbergConverter.ExtractAsync(input, config);
    Assert.Empty(result.Errors);
    Assert.Equal("Transport probe",
                 Assert.Single(result.Results).Content.Trim());
  }

  [Fact]
  public async Task ExternalRedactionAcceptsBytesWithoutMimeHint() {
    var input = new ExtractInput {
      Kind = ExtractInputKind.Bytes,
      Bytes = Encoding.UTF8.GetBytes("Transport probe"),
      Filename = "probe.txt",
    };
    var config = new ExtractionConfig { UseCache = false };
    var result = await XbergConverter.ExtractWithExternalRedactionAsync(
        input, config, "[]", null, null);
    Assert.Empty(result.Errors);
    Assert.Equal("Transport probe",
                 Assert.Single(result.Results).Content.Trim());
  }

  [Fact]
  public void BytesFactoriesKeepAnIndependentPayload() {
    byte[] bytes = [0, 128, 255];
    var input = ExtractInput.FromBytes(bytes, "application/pdf", "probe.pdf");
    var facade =
        XbergConverter.ExtractInputFromBytes(bytes, "application/pdf", null);
    bytes[0] = 42;
    Assert.Equal(ExtractInputKind.Bytes, input.Kind);
    Assert.Equal("application/pdf", input.MimeType);
    Assert.Equal("probe.pdf", input.Filename);
    Assert.Null(facade.Filename);
    Assert.Equal(new byte[] { 0, 128, 255 }, input.Bytes);
    Assert.Equal(input.Bytes, facade.Bytes);
  }

  [Fact]
  public void BytesFacadeAllocatesOnlyOneManagedPayload() {
    const int payloadSize = 4 * 1024 * 1024;
    _ = XbergConverter.ExtractInputFromBytes([42], "application/octet-stream",
                                             "probe.bin");
    var bytes = new byte[payloadSize];
    bytes.AsSpan().Fill(255);
    long before = GC.GetAllocatedBytesForCurrentThread();
    var input = XbergConverter.ExtractInputFromBytes(
        bytes, "application/octet-stream", "probe.bin");
    long allocated = GC.GetAllocatedBytesForCurrentThread() - before;

    Assert.True(
        allocated <= payloadSize + 512 * 1024,
        $"Facade allocated {allocated} bytes for a {payloadSize}-byte payload");
    Assert.NotSame(bytes, input.Bytes);
    Assert.True(bytes.AsSpan().SequenceEqual(input.Bytes));
  }

  [Fact]
  public async Task LayoutSelectionAcceptsPdfBytesWithoutMimeHint() {
    const string content = "BT /F1 12 Tf 72 720 Td (Transport probe) Tj ET";
    var objects = new[] {
      "<< /Type /Catalog /Pages 2 0 R >>",
      "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
      "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Resources << " +
          "/Font << /F1 4 0 R >> >> /Contents 5 0 R >>",
      "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>",
      $"<< /Length {content.Length} >>\nstream\n{content}\nendstream",
    };
    var pdf = new StringBuilder("%PDF-1.4\n");
    var offsets = new List<int>();
    foreach (var value in objects) {
      offsets.Add(pdf.Length);
      pdf.Append($"{offsets.Count} 0 obj\n{value}\nendobj\n");
    }
    var xref = pdf.Length;
    pdf.Append($"xref\n0 {offsets.Count + 1}\n0000000000 65535 f \n");
    foreach (var offset in offsets)
      pdf.Append($"{offset:D10} 00000 n \n");
    pdf.Append(
        $"trailer\n<< /Size {offsets.Count + 1} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n");
    var input = new ExtractInput {
      Kind = ExtractInputKind.Bytes,
      Bytes = Encoding.ASCII.GetBytes(pdf.ToString()),
      Filename = "probe.pdf",
    };
    var result = await XbergConverter.ExtractWithLayoutPagesAsync(
        input, new ExtractionConfig { UseCache = false, DisableOcr = true },
        []);
    Assert.Empty(result.Errors);
    Assert.Equal("Transport probe",
                 Assert.Single(result.Results).Content.Trim());
  }

  [Fact]
  public void PublicJsonRoundTripsBytesAsNumbers() {
    byte[] bytes = [0, 128, 255];
    var input =
        new ExtractInput { Kind = ExtractInputKind.Bytes, Bytes = bytes };
    var json = JsonSerializer.Serialize(input);
    using var document = JsonDocument.Parse(json);
    Assert.Equal(JsonValueKind.Array,
                 document.RootElement.GetProperty("bytes").ValueKind);
    Assert.Equal(bytes, ExtractInput.FromJson(json).Bytes);
  }
}
