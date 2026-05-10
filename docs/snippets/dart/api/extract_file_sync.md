```dart title="Dart"
import 'package:kreuzberg/kreuzberg.dart';

Future<void> main() async {
  // Sync semantics — flutter_rust_bridge surfaces every call as a Future,
  // so even the *Sync entrypoints must be awaited from Dart.
  final result = await KreuzbergBridge.extractFileSync('document.pdf', null);

  print(result.content);
  print('MIME type: ${result.mimeType}');
  print('Tables: ${result.tables.length}');
}
```
