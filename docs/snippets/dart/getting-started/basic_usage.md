```dart title="Dart"
import 'package:kreuzberg/kreuzberg.dart';

Future<void> main() async {
  // Default ExtractionConfig — flutter_rust_bridge surfaces every call
  // as a Future, so even non-async-flavored entrypoints must be awaited.
  final result = await KreuzbergBridge.extractFile('document.pdf', null);

  print(result.content);
  print('MIME type: ${result.mimeType}');
}
```
