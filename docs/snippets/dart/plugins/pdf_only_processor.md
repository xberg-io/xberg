<!-- snippet:skip reason="Dart cannot construct the opaque BoxFn closure types required by createPostProcessorDartImpl — flutter_rust_bridge generates them as RustOpaqueInterface with no Dart-side wrapper. The ProcessingStage enum is also not surfaced. Custom post-processors must be written in Rust." -->
```dart title="Dart"
import 'package:kreuzberg/kreuzberg.dart';

Future<void> main() async {
  // A Dart implementation of the `PostProcessor` trait that gates on PDF
  // MIME type cannot be plugged into the global registry.
  // `Kreuzberg.registerPostProcessor(impl)` exists, but its
  // `createPostProcessorDartImpl` factory takes opaque `BoxFn*` closure
  // values plus a `BoxFnDartFnFutureProcessingStage` whose constructors are
  // not surfaced through flutter_rust_bridge. The `ProcessingStage` enum is
  // not exported to Dart either.
  //
  // Implement the post-processor in Rust as `Plugin + PostProcessor` and
  // register it via `register_post_processor` in a Rust shim crate.
}
```
