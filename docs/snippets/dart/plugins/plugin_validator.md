<!-- snippet:skip -->
```dart title="Dart"
import 'package:kreuzberg/kreuzberg.dart';

Future<void> main() async {
  // Note: while flutter_rust_bridge surfaces a `Validator` abstract class
  // and a `createValidatorDartImpl` factory, the Dart binding does not
  // expose `registerValidator`. Without a registration entry point, a
  // Dart-side `ValidatorDartImpl` cannot be plugged into the global
  // validator registry.
  //
  // Custom validators must be written in Rust and registered via a Rust
  // shim crate that links kreuzberg before the Dart host process loads
  // the dynamic library.
}
```
