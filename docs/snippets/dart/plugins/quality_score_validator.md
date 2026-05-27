<!-- snippet:skip reason="Dart cannot construct the opaque BoxFn closure types required by createValidatorDartImpl — flutter_rust_bridge generates them as RustOpaqueInterface with no Dart-side wrapper. Custom validators must be written in Rust." -->
```dart title="Dart"
import 'package:kreuzberg/kreuzberg.dart';

Future<void> main() async {
  // A Dart implementation of the `Validator` trait that inspects
  // `metadata.additional["quality_score"]` cannot be plugged into the global
  // validator registry. The Dart binding exposes
  // `Kreuzberg.registerValidator(impl)` and the `createValidatorDartImpl`
  // factory, but every closure parameter (`validate`, `shouldValidate`,
  // `priority`) is typed as an opaque `BoxFn*` whose constructor is not
  // surfaced through flutter_rust_bridge.
  //
  // Implement the validator in Rust as `Plugin + Validator` and register it
  // via `register_validator` in a Rust shim crate that links kreuzberg
  // before the Dart host process loads the dynamic library.
}
```
