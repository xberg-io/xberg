using System.ComponentModel;
using System.Diagnostics;
using System.Runtime.InteropServices;
using System.Security.Cryptography;
using System.Text;
using System.Text.Json;
using Xberg;

var (native, mode, filename) = (args[0], args[1], args[2]);
if (mode is not("path" or "bytes" or "factory"))
  throw new ArgumentException("Unknown input mode");
var library = NativeLibrary.Load(native);
foreach (var symbol in new[] { "xberg_extract_owned",
                               "xberg_extract_input_from_json_with_bytes" })
  _ = NativeLibrary.GetExport(library, symbol);
NativeLibrary.SetDllImportResolver(typeof(XbergConverter).Assembly,
                                   (name, _, _) => name == "xberg_ffi"
                                                       ? library
                                                       : IntPtr.Zero);

var config = new ExtractionConfig { UseCache = false, DisableOcr = true };
var mime =
    Path.GetExtension(filename) == ".pdf"
        ? "application/pdf"
        : "application/" +
              "vnd.openxmlformats-officedocument.wordprocessingml.document";
var warmPath = args[3];
var warmPayload = mode == "path" ? null : File.ReadAllBytes(warmPath);
Check(await XbergConverter.ExtractAsync(Input(warmPath, warmPayload), config));
var payload = mode == "path" ? null : File.ReadAllBytes(filename);
GC.Collect();
GC.WaitForPendingFinalizers();
GC.Collect();
var (readyCurrent, readyPeak) = Memory.Read();
Console.WriteLine(JsonSerializer.Serialize(
    new { stage = "ready", pid = Environment.ProcessId, readyCurrent, readyPeak,
          runtime = RuntimeInformation.FrameworkDescription,
          architecture = RuntimeInformation.ProcessArchitecture.ToString() }));
if (Console.ReadLine() != "go")
  throw new InvalidOperationException("Missing go signal");

var start = GC.GetTotalAllocatedBytes(precise: true);
var input = Input(filename, payload);
var result = await XbergConverter.ExtractAsync(input, config);
var managed = GC.GetTotalAllocatedBytes(precise: true) - start;
var (endCurrent, endPeak) = Memory.Read();
Console.WriteLine("{\"stage\":\"measured\"}");
if (Console.ReadLine() != "report")
  throw new InvalidOperationException("Missing report signal");

var content = Check(result);
var sha256 =
    Convert.ToHexString(SHA256.HashData(Encoding.UTF8.GetBytes(content)));
using var process = Process.GetCurrentProcess();
var modules = process.Modules.Cast<ProcessModule>()
                  .Select(module => module.FileName)
                  .ToArray();
Console.WriteLine(JsonSerializer.Serialize(new {
  stage = "result", managed, endCurrent, endPeak, sha256,
  mime = result.Results[0].MimeType, length = content.Length, modules
}));
GC.KeepAlive(payload);

ExtractInput Input(string path, byte[]? bytes) => mode switch {
  "path" => new ExtractInput { Kind = ExtractInputKind.Uri, Uri = path,
                               MimeType = mime },
  "factory" => XbergConverter.ExtractInputFromBytes(bytes!, mime,
                                                    Path.GetFileName(path)),
  _ => new ExtractInput { Kind = ExtractInputKind.Bytes, Bytes = bytes,
                          MimeType = mime, Filename = Path.GetFileName(path) },
};

static string Check(ExtractionResult result) {
  if (result.Errors.Count != 0 || result.Results.Count != 1)
    throw new InvalidOperationException(
        $"Unexpected result counts {result.Results.Count}/{result.Errors.Count}");
  var content = result.Results[0].Content;
  if (content.Trim() != "Xberg transport acceptance 3152")
    throw new InvalidOperationException(
        "Extraction did not return the expected marker");
  return content;
}

internal static class Memory {
  [StructLayout(LayoutKind.Sequential)]
  private struct Counters {
    public uint Size, PageFaultCount;
    public nuint PeakWorkingSet, WorkingSet, PeakPagedPool, PagedPool,
        PeakNonPagedPool, NonPagedPool;
    public nuint Commit, PeakCommit, PrivateCommit;
  }

  [DllImport("kernel32.dll")]
  private static extern IntPtr GetCurrentProcess();

  [DllImport("kernel32.dll", SetLastError = true)]
  [return:MarshalAs(UnmanagedType.Bool)]
  private static extern bool
  K32GetProcessMemoryInfo(IntPtr process, ref Counters counters, uint size);

  public static (ulong Current, ulong Peak) Read() {
    if (!OperatingSystem.IsWindows())
      return (0, 0);
    var counters = new Counters { Size = (uint)Marshal.SizeOf<Counters>() };
    if (!K32GetProcessMemoryInfo(GetCurrentProcess(), ref counters,
                                 counters.Size))
      throw new Win32Exception(Marshal.GetLastPInvokeError());
    return ((ulong)counters.PrivateCommit, (ulong)counters.PeakCommit);
  }
}
