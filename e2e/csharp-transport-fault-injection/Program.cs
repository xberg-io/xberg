using System.Runtime.CompilerServices;
using System.Runtime.InteropServices;
using System.Text.Json;
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
foreach (var (mode, expected) in new[] { (3, 0), (4, 101), (5, 11100) }) {
  Shim.SetMode(mode);
  var payload = InvokeFaultingFactory(mode);
  GC.Collect();
  var receipts = Shim.Receipts();
  if (receipts != expected || payload.IsAlive)
    throw new Exception(
        $"Factory mode {mode} expected {expected}, got {receipts}, payload alive={payload.IsAlive}");
  Console.WriteLine(
      $"factory mode={mode}, receipts={receipts}, payload collected");
}

Shim.SetMode(6);
var splitPayload = InvokeFaultingSplitInput();
GC.Collect();
var splitReceipts = Shim.Receipts();
if (splitReceipts != 100 || splitPayload.IsAlive)
  throw new Exception(
      $"Split input expected 100, got {splitReceipts}, payload alive={splitPayload.IsAlive}");
Console.WriteLine($"split input receipts={splitReceipts}, payload collected");

[MethodImpl(MethodImplOptions.NoInlining)]
static WeakReference InvokeFaultingSplitInput() {
  byte[] bytes = [0, 128, 255];
  var weak = new WeakReference(bytes);
  try {
    XbergConverter
        .ExtractWithLayoutPagesAsync(
            new ExtractInput { Kind = ExtractInputKind.Bytes, Bytes = bytes,
                               Filename = "probe.pdf" },
            new ExtractionConfig(), [])
        .GetAwaiter()
        .GetResult();
    throw new Exception("Expected split input failure injection");
  } catch (XbergException e)
      when (e.Message == "ExtractInputFromJsonWithBytes failed") {
  }
  return weak;
}

[MethodImpl(MethodImplOptions.NoInlining)]
static WeakReference InvokeFaultingFactory(int mode) {
  byte[] bytes = [0, 128, 255];
  var weak = new WeakReference(bytes);
  try {
    _ = XbergConverter.ExtractInputFromBytes(bytes, "probe/type", "probe.bin");
    throw new Exception("Expected factory failure injection");
  } catch (XbergException) when (mode is 3 or 4) {
  } catch (JsonException) when (mode == 5) {
  }
  return weak;
}

static class Shim {
  [DllImport("xberg_ffi", EntryPoint = "set_mode")]
  public static extern void SetMode(int mode);
  [DllImport("xberg_ffi", EntryPoint = "receipts")]
  public static extern int Receipts();
}
