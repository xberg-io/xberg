<!-- snippet:skip reason="Plugin trait lifecycle methods (initialize, shutdown) are not exposed in Dart; logging must be implemented in Rust." -->
```dart title="Dart"
import 'package:kreuzberg/kreuzberg.dart';

Future<void> main() async {
  // Plugin lifecycle logging hooks are not available in Dart. The Plugin
  // trait methods (initialize, shutdown) that enable structured logging are
  // only exposed in Rust. Dart plugins (OcrBackend, PostProcessor, Validator,
  // EmbeddingBackend) cannot implement Plugin methods directly.
  //
  // For logging, implement plugins in Rust using the tracing or log crate,
  // then register them via a Rust shim crate before the Dart host loads the
  // dynamic library.
}
```
