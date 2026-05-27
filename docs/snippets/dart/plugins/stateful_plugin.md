<!-- snippet:skip reason="Dart cannot construct the opaque BoxFn closure types required by createPostProcessorDartImpl — flutter_rust_bridge generates them as RustOpaqueInterface with no Dart-side wrapper. The closure-captured state pattern is therefore unreachable. Custom plugins must be written in Rust." -->
```dart title="Dart"
import 'package:kreuzberg/kreuzberg.dart';

Future<void> main() async {
  // A stateful Dart `PostProcessor` that captures mutable counters in its
  // closure cannot be plugged into the global registry.
  // `Kreuzberg.registerPostProcessor(impl)` exists, but the
  // `createPostProcessorDartImpl` factory takes opaque `BoxFn*` closure
  // values whose constructors are not surfaced through flutter_rust_bridge,
  // so the closure-capture pattern is unreachable from Dart.
  //
  // Implement stateful plugins in Rust using `Mutex`/`AtomicU64` for
  // interior mutability, then register them in a Rust shim crate.
}
```
