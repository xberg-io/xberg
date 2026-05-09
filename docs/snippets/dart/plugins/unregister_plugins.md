<!-- snippet:skip -->
```dart title="Dart"
import 'package:kreuzberg/kreuzberg.dart';

Future<void> main() async {
  // Note: the Dart binding does not expose per-plugin unregistration
  // (`registry.remove(name)`). flutter_rust_bridge surfaces only the
  // bulk-clear entry points: `clearOcrBackends`, `clearPostProcessors`,
  // and `clearValidators`. To remove a single plugin, clear the relevant
  // registry and re-register the plugins you want to keep from the Rust
  // core (or from a Rust shim crate that links kreuzberg).
}
```
