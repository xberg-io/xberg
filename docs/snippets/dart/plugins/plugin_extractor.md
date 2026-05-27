<!-- snippet:skip reason="DocumentExtractor trait has no createDocumentExtractorDartImpl factory in the generated Dart binding; custom extractors must be written and registered in Rust." -->
```dart title="Dart"
import 'package:kreuzberg/kreuzberg.dart';

Future<void> main() async {
  // Custom document extractors cannot be implemented in Dart. While the
  // traits.dart file includes the DocumentExtractor abstract class,
  // flutter_rust_bridge does not generate a createDocumentExtractorDartImpl
  // factory function, so there is no way to bridge Dart closures into the
  // extractor registry.
  //
  // Implement custom extractors in Rust and register them via a Rust shim
  // crate that links kreuzberg before the Dart host loads the dynamic library.
}
```
