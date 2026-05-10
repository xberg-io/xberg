```dart title="Dart"
import 'dart:typed_data';

import 'package:kreuzberg/kreuzberg.dart';

Future<void> main() async {
  final config = ExtractionConfig(
    useCache: true,
    enableQualityProcessing: true,
    forceOcr: false,
    disableOcr: false,
    keywords: KeywordConfig(
      algorithm: KeywordAlgorithm.yake,
      maxKeywords: 10,
      minScore: 0.3,
      ngramRange: Int64List.fromList(<int>[1, 3]),
    ),
    resultFormat: ResultFormat.unified,
    outputFormat: OutputFormat.plain(),
    includeDocumentStructure: false,
    maxArchiveDepth: 3,
  );

  final result = await KreuzbergBridge.extractFile('research_paper.pdf', null, config);
  final keywords = result.extractedKeywords;
  if (keywords != null) {
    for (final keyword in keywords) {
      print('${keyword.text} (score: ${keyword.score})');
    }
  }
}
```
