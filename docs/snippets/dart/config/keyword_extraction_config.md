```dart title="Dart"
import 'package:flutter_rust_bridge/flutter_rust_bridge.dart' show Int64List;

import 'package:xberg/xberg.dart';

Future<void> main() async {
  final config = ExtractionConfig(
    useCache: true,
    enableQualityProcessing: true,
    forceOcr: false,
    disableOcr: false,
    keywords: KeywordConfig(
      algorithm: KeywordAlgorithm.yake,
      maxKeywords: 10,
      minScore: 0.1,
      ngramRange: Int64List.fromList(<int>[1, 3]),
      language: 'en',
    ),
    resultFormat: ResultFormat.unified,
    outputFormat: OutputFormat.plain(),
    includeDocumentStructure: false,
    useLayoutForMarkdown: false,
    maxArchiveDepth: 3,
  );

  final result = await XbergBridge.extract('document.pdf', null, config);
  print('Keywords: ${result.extractedKeywords}');
}
```
