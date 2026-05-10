```dart title="Dart"
import 'package:kreuzberg/kreuzberg.dart';

Future<void> main() async {
  final config = ExtractionConfig(
    useCache: true,
    enableQualityProcessing: true,
    forceOcr: false,
    disableOcr: false,
    htmlOutput: const HtmlOutputConfig(
      theme: HtmlTheme.gitHub,
      classPrefix: 'kb-',
      embedCss: true,
    ),
    resultFormat: ResultFormat.unified,
    outputFormat: OutputFormat.html(),
    includeDocumentStructure: false,
    maxArchiveDepth: 3,
  );

  final result = await KreuzbergBridge.extractFile('document.pdf', null, config);
  print(result.content);
}
```
