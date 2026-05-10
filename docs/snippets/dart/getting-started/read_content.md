```dart title="Dart"
import 'package:kreuzberg/kreuzberg.dart';

Future<void> main() async {
  final result = await KreuzbergBridge.extractFile('document.pdf', null);

  print(result.content);

  for (final table in result.tables) {
    print('Table: $table');
  }

  final chunks = result.chunks;
  if (chunks != null) {
    for (final chunk in chunks) {
      print('Chunk: $chunk');
    }
  }
}
```
