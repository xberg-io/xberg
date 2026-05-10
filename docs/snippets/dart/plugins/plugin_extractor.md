<!-- snippet:skip -->
```dart title="Dart"
import 'package:kreuzberg/kreuzberg.dart';

Future<void> main() async {
  // Note: implementing the Rust `DocumentExtractor` trait from Dart is
  // not feasible through flutter_rust_bridge. No `DocumentExtractor`
  // abstract class or `createDocumentExtractorDartImpl` factory is
  // generated, so there is no way to construct a Dart-side extractor.
  //
  // Authoring a custom extractor must be done in Rust. After implementing
  // `Plugin + DocumentExtractor`, register the extractor in a Rust shim
  // crate that links both `kreuzberg` and the Dart binding crate before
  // the Dart host process loads the dynamic library.
}
```
