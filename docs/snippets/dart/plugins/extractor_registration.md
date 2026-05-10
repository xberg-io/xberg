<!-- snippet:skip -->
```dart title="Dart"
import 'package:kreuzberg/kreuzberg.dart';

Future<void> main() async {
  // Note: the Dart binding does not expose `registerDocumentExtractor` /
  // `getDocumentExtractorRegistry`. flutter_rust_bridge does not surface
  // a Dart-side `DocumentExtractor` trait class, and the global registry
  // accessors are Rust-only.
  //
  // Built-in extractors (PDF, DOCX, HTML, etc.) are registered automatically
  // by the kreuzberg core when the library initializes. Custom extractors
  // must be written in Rust and linked into a Rust shim crate that is
  // loaded before the Dart host process opens the dynamic library.
}
```
