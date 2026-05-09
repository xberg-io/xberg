<!-- snippet:skip -->
```dart title="Dart"
import 'package:kreuzberg/kreuzberg.dart';

Future<void> main() async {
  // Note: the Dart binding surfaces an `EmbeddingBackend` abstract class
  // and a `createEmbeddingBackendDartImpl` factory, but does not expose
  // `registerEmbeddingBackend`. Without a registration entry point, a
  // Dart-side `EmbeddingBackendDartImpl` cannot be wired into
  // `EmbeddingModelType.plugin { name }` dispatch.
  //
  // Implement the backend in Rust as `Plugin + EmbeddingBackend` and
  // register it via `register_embedding_backend` in a Rust shim crate
  // that links kreuzberg before the Dart host process loads the dynamic
  // library. Dart code can then call `KreuzbergBridge.embedTextsAsync`
  // with an `EmbeddingConfig` whose model selects the registered plugin.
}
```
