<!-- snippet:skip -->
```dart title="Dart"
import 'package:kreuzberg/kreuzberg.dart';

Future<void> main() async {
  // Note: plugin testing patterns assume a custom plugin can be
  // instantiated and exercised in isolation. The Dart binding does not
  // expose registration entry points for custom extractors,
  // post-processors, validators, or embedding backends, so there is no
  // Dart-visible plugin surface to drive from a `package:test` suite.
  //
  // Test custom plugins in Rust with `#[tokio::test]` against the trait
  // implementations directly, then exercise the registered plugin from
  // Dart via `KreuzbergBridge.extractFile` / `extractBytes` and assert
  // on the resulting `ExtractionResult`.
}
```
