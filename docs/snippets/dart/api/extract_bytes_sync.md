```dart title="Dart"
import 'dart:io';
import 'dart:typed_data';

import 'package:kreuzberg/kreuzberg.dart';

Future<void> main() async {
  final Uint8List bytes = await File('document.pdf').readAsBytes();
  // Sync semantics — flutter_rust_bridge surfaces every call as a Future,
  // so even the *Sync entrypoints must be awaited from Dart.
  final result = await KreuzbergBridge.extractBytesSync(bytes, 'application/pdf');

  print(result.content);
  print('MIME type: ${result.mimeType}');
}
```
