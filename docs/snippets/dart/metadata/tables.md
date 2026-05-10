```dart title="Dart"
import 'package:kreuzberg/kreuzberg.dart';

Future<void> main() async {
  final result = await KreuzbergBridge.extractFile('document.pdf', null);

  for (final table in result.tables) {
    print('Table on page ${table.pageNumber} with ${table.cells.length} rows');
    print(table.markdown);

    for (final row in table.cells) {
      print(row);
    }

    if (table.boundingBox != null) {
      print('Bounding box: ${table.boundingBox}');
    }
  }
}
```
