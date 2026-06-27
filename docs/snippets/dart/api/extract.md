```dart title="Dart"
import 'package:xberg/xberg.dart';

final output = await Xberg.extract(
  const ExtractInput(
    kind: ExtractInputKind.uri,
    uri: 'document.pdf',
  ),
);

print(output.results.first.content);
```
