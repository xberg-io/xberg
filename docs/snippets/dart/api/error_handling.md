```dart title="Dart"
import 'package:kreuzberg/kreuzberg.dart';

Future<void> main() async {
  try {
    final result = await KreuzbergBridge.extractFile('document.pdf', null);
    print(result.content);
  } on Exception catch (e) {
    // flutter_rust_bridge converts every KreuzbergError variant
    // (Io / UnsupportedFormat / Parsing / MissingDependency, ...)
    // into a Dart exception whose message preserves the original context.
    print('Extraction failed: $e');
  }
}
```
