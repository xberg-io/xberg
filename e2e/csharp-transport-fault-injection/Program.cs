using System.Runtime.InteropServices;
using Xberg;

foreach (var (mode, expected) in new[] { (0, 1), (1, 111), (2, 110) }) {
  Shim.SetMode(mode);
  try {
    await XbergConverter.ExtractWithLayoutPagesAsync(
        new ExtractInput { Kind = ExtractInputKind.Bytes, Bytes = [0, 128, 255],
                           Filename = "probe.pdf" },
        new ExtractionConfig(), []);
    throw new Exception("Expected failure injection");
  } catch (XbergException) {
  }
  var receipts = Shim.Receipts();
  if (receipts != expected)
    throw new Exception($"Mode {mode} expected {expected}, got {receipts}");
  Console.WriteLine($"mode={mode}, receipts={receipts}");
}
static class Shim {
  [DllImport("xberg_ffi", EntryPoint = "set_mode")]
  public static extern void SetMode(int mode);
  [DllImport("xberg_ffi", EntryPoint = "receipts")]
  public static extern int Receipts();
}
