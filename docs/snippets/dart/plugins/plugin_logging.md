<!-- snippet:skip -->
```dart title="Dart"
import 'package:kreuzberg/kreuzberg.dart';

Future<void> main() async {
  // Note: plugin-side logging hooks (`Plugin::initialize`,
  // `Plugin::shutdown`, and per-method `tracing::info!` calls) live on
  // the Rust trait implementations. The Dart binding does not expose a
  // `Plugin` abstract class or registration entry points for custom
  // plugins, so there is no Dart-visible surface to attach logging to.
  //
  // Implement plugins in Rust using the `tracing` or `log` crate and
  // register them via the corresponding `register_*` Rust API in a
  // shim crate that links kreuzberg before the Dart host process loads
  // the dynamic library.
}
```
