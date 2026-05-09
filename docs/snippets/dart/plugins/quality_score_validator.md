<!-- snippet:skip -->
```dart title="Dart"
import 'package:kreuzberg/kreuzberg.dart';

Future<void> main() async {
  // Note: the Dart binding does not expose `registerValidator`. A Dart
  // implementation of the `Validator` trait that inspects
  // `metadata.additional["quality_score"]` cannot be plugged into the
  // global validator registry from Dart.
  //
  // Implement the validator in Rust as `Plugin + Validator` and register
  // it via `register_validator` in a Rust shim crate that links kreuzberg
  // before the Dart host process loads the dynamic library.
}
```
