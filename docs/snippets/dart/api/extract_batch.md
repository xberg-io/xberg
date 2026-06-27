```dart title="Dart"
import 'dart:convert';
import 'package:xberg/xberg.dart';

final output = await Xberg.extractBatch([
  const ExtractInput(
    kind: ExtractInputKind.uri,
    uri: 'document.pdf',
  ),
  ExtractInput(
    kind: ExtractInputKind.bytes,
    bytes: utf8.encode('Hello from memory'),
    mimeType: 'text/plain',
    filename: 'note.txt',
  ),
]);

for (final result in output.results) {
  print(result.content);
}
```
