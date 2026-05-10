```dart title="Dart"
import 'package:kreuzberg/kreuzberg.dart';

Future<void> main() async {
  final result = await KreuzbergBridge.extractFile('document.pdf', null);

  print(result.content);
  print('MIME type: ${result.mimeType}');
  print('Tables: ${result.tables.length}');
}
```
