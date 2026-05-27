<!-- snippet:skip reason="Dart cannot construct the opaque BoxFn closure types required by createEmbeddingBackendDartImpl — flutter_rust_bridge generates them as RustOpaqueInterface with no Dart-side wrapper. Custom embedding backends must be written in Rust." -->
```dart title="Dart"
import 'package:kreuzberg/kreuzberg.dart';

Future<void> main() async {
  // A Dart implementation of the `EmbeddingBackend` trait cannot be plugged
  // into the global registry. `Kreuzberg.registerEmbeddingBackend(impl)`
  // exists, but its `createEmbeddingBackendDartImpl` factory takes opaque
  // `BoxFn*` closure values whose constructors are not surfaced through
  // flutter_rust_bridge.
  //
  // Implement the backend in Rust as `Plugin + EmbeddingBackend` and register
  // it via `register_embedding_backend` in a Rust shim crate.
}
```
