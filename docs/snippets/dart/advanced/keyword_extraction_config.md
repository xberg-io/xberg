```dart title="Dart"
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
      minScore: 0.3,
      language: 'en',
    ),
    resultFormat: ResultFormat.unified,
    outputFormat: OutputFormat.plain(),
    includeDocumentStructure: false,
    useLayoutForMarkdown: false,
    maxArchiveDepth: 3,
  );

  final output = await XbergBridge.extract(
    const ExtractInput(kind: ExtractInputKind.uri, uri: 'document.pdf'),
    config: config,
  );
  final result = output.results.first;
  print('Keywords: ${result.extractedKeywords}');
}
```
