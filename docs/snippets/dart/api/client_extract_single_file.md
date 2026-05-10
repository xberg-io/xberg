```dart title="Dart"
import 'dart:convert';
import 'dart:io';

import 'package:http/http.dart' as http;

Future<void> main() async {
  final file = File('document.pdf');
  final bytes = await file.readAsBytes();

  final request = http.MultipartRequest(
    'POST',
    Uri.parse('http://localhost:8000/extract'),
  )..files.add(
      http.MultipartFile.fromBytes(
        'file',
        bytes,
        filename: 'document.pdf',
      ),
    );

  final streamed = await request.send();
  final response = await http.Response.fromStream(streamed);
  if (response.statusCode >= 400) {
    throw HttpException('Server returned ${response.statusCode}: ${response.body}');
  }

  final result = jsonDecode(response.body) as Map<String, dynamic>;
  print(result['content'] ?? '');
}
```
