```dart title="Dart"
import 'dart:io';
import 'dart:typed_data';

import 'package:kreuzberg/kreuzberg.dart';

Future<void> main() async {
  final Uint8List bytes = await File('document.pdf').readAsBytes();
  final result = await KreuzbergBridge.extractBytes(bytes, 'application/pdf');

  print(result.content);
  print('MIME type: ${result.mimeType}');
}
```
