```dart title="Dart"
import 'package:kreuzberg/kreuzberg.dart';

Future<void> main() async {
  final items = <BatchFileItem>[
    const BatchFileItem(path: 'doc1.pdf'),
    BatchFileItem(
      path: 'scan.pdf',
      config: FileExtractionConfig(forceOcr: true),
    ),
  ];

  // Sync semantics — flutter_rust_bridge still returns a Future from Dart.
  final results = await KreuzbergBridge.batchExtractFilesSync(items);

  print('Processed ${results.length} files');
  for (final result in results) {
    print('${result.mimeType}: ${result.content.length} chars');
  }
}
```
