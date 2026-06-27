<!-- snippet:skip reason="Testing Dart plugins via package:test is not practical because test closure capture varies by test framework; test via integration after registration or via Rust unit tests." -->
```dart title="Dart"
import 'package:xberg/xberg.dart';

Future<void> main() async {
  // Plugin testing with Dart is different from Rust. Dart plugins cannot be
  // unit-tested in isolation because the registration mechanism uses closures
  // captured in the plugin factory, and test framework async contexts vary.
  //
  // Recommended approaches:
  // 1. Test core plugin logic directly in unit tests with mock data
  // 2. Write integration tests that register the plugin and exercise it via
  //    XbergBridge.extract or other extraction methods
  // 3. For complex plugins, implement in Rust and test with #[tokio::test]
}
```
