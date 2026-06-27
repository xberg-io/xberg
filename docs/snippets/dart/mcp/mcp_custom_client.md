<!-- snippet:syntax-only -->

```dart title="Dart"
import 'dart:convert';
import 'dart:io';

Future<void> main() async {
  final process = await Process.start('xberg', <String>['mcp']);

  final request = <String, Object?>{
    'method': 'tools/call',
    'params': <String, Object?>{
      'name': 'extract',
      'arguments': <String, Object?>{
        'path': 'document.pdf',
        'async': true,
      },
    },
  };

  process.stdin.writeln(jsonEncode(request));
  await process.stdin.flush();
  await process.stdin.close();

  final line = await process.stdout
      .transform(utf8.decoder)
      .transform(const LineSplitter())
      .first;
  print(line);

  await process.exitCode;
}
```
