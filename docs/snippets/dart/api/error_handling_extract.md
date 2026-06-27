```dart title="Dart"
import 'package:xberg/xberg.dart';

Future<void> main() async {
  final config = ExtractionConfig(
    useCache: true,
    enableQualityProcessing: true,
    forceOcr: false,
    disableOcr: false,
    resultFormat: ResultFormat.unified,
    outputFormat: OutputFormat.markdown(),
    includeDocumentStructure: false,
    maxArchiveDepth: 3,
    useLayoutForMarkdown: false,
  );

  try {
    final result = await XbergBridge.extract('document.pdf', null, config);
    print('Extracted ${result.content.length} chars');
    print('MIME: ${result.mimeType}');
    if (result.detectedLanguages != null) {
      print('Languages: ${result.detectedLanguages}');
    }
  } on Exception catch (e) {
    final message = e.toString();
    if (message.contains('UnsupportedFormat')) {
      print('Unsupported format: $message');
    } else if (message.contains('MissingDependency')) {
      print('Install the required dependency: $message');
    } else if (message.contains('Parsing')) {
      print('Corrupt or invalid document: $message');
    } else {
      print('Extraction failed: $message');
    }
  }
}
```
