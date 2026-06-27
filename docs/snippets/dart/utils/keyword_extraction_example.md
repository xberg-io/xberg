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
    ),
    resultFormat: ResultFormat.unified,
    outputFormat: OutputFormat.plain(),
    includeDocumentStructure: false,
    useLayoutForMarkdown: false,
    maxArchiveDepth: 3,
  );

  final output = await XbergBridge.extract(
    const ExtractInput(kind: ExtractInputKind.uri, uri: 'research_paper.pdf'),
    config: config,
  );
  final result = output.results.first;
  final keywords = result.extractedKeywords;
  if (keywords != null) {
    for (final keyword in keywords) {
      print('${keyword.text} (score: ${keyword.score})');
    }
  }
}
```
