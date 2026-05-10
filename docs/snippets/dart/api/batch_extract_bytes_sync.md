```dart title="Dart"
import 'dart:convert';
import 'dart:typed_data';

import 'package:kreuzberg/kreuzberg.dart';

Future<void> main() async {
  final Uint8List first = Uint8List.fromList(utf8.encode('Hello, world!'));
  final Uint8List second = Uint8List.fromList(utf8.encode('<html>test</html>'));

  final items = <BatchBytesItem>[
    BatchBytesItem(content: first, mimeType: 'text/plain'),
    BatchBytesItem(
      content: second,
      mimeType: 'text/html',
      config: const FileExtractionConfig(forceOcr: true),
    ),
  ];

  // Sync semantics — flutter_rust_bridge still returns a Future from Dart.
  final results = await KreuzbergBridge.batchExtractBytesSync(items);

  print('Processed ${results.length} items');
  for (final result in results) {
    print('${result.mimeType}: ${result.content.length} chars');
  }
}
```
