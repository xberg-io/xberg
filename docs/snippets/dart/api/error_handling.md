```dart title="Dart"
import 'package:xberg/xberg.dart';

Future<void> main() async {
  try {
    final result = await XbergBridge.extract('document.pdf', null);
    print(result.content);
  } on Exception catch (e) {
    // flutter_rust_bridge converts every XbergError variant
    // (Io / UnsupportedFormat / Parsing / MissingDependency, ...)
    // into a Dart exception whose message preserves the original context.
    print('Extraction failed: $e');
  }
}
```
