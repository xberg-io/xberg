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
  )
    ..files.add(
      http.MultipartFile.fromBytes(
        'file',
        bytes,
        filename: 'document.pdf',
      ),
    )
    ..fields['chunking'] = jsonEncode({
      'max_characters': 800,
      'overlap': 100,
    });

  final streamed = await request.send();
  final response = await http.Response.fromStream(streamed);
  if (response.statusCode >= 400) {
    throw HttpException('Server returned ${response.statusCode}: ${response.body}');
  }

  final result = jsonDecode(response.body) as Map<String, dynamic>;
  final chunks = result['chunks'] as List<dynamic>?;
  if (chunks != null) {
    print('${chunks.length} chunks');
    for (final chunk in chunks) {
      final content = (chunk as Map<String, dynamic>)['content'] as String? ?? '';
      print('  ${content.length} chars');
    }
  }
}
```
