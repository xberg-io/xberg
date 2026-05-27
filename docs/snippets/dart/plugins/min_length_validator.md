<!-- snippet:skip reason="Dart cannot construct the opaque BoxFn closure types required by createValidatorDartImpl — flutter_rust_bridge generates them as RustOpaqueInterface with no Dart-side wrapper. Custom validators must be written in Rust." -->
```dart title="Dart"
import 'package:kreuzberg/kreuzberg.dart';

Future<void> main() async {
  // A Dart implementation of the `Validator` trait that asserts a minimum
  // content length cannot be plugged into the global validator registry.
  // `Kreuzberg.registerValidator(impl)` exists, but its
  // `createValidatorDartImpl` factory takes opaque `BoxFn*` closure
  // arguments whose constructors are not surfaced through
  // flutter_rust_bridge.
  //
  // Implement the validator in Rust and register it via `register_validator`
  // in a Rust shim crate.
}
```
