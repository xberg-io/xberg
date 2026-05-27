<!-- snippet:skip reason="DocumentExtractor trait has no createDocumentExtractorDartImpl factory; custom extractors must be implemented in Rust." -->
```dart title="Dart"
import 'package:kreuzberg/kreuzberg.dart';

Future<void> main() async {
  // Custom document extractors cannot be implemented in Dart. Creating a
  // PDF metadata extractor would require implementing the DocumentExtractor
  // trait, but flutter_rust_bridge does not generate the
  // createDocumentExtractorDartImpl factory function.
  //
  // Implement the PDF metadata extractor in Rust and register it via a
  // Rust shim crate that links kreuzberg before the Dart host loads the
  // dynamic library.
}
```
