# Changelog

All notable changes to this package will be documented in this file.

## 5.0.0-rc.24

- **images**: `ExtractedImage.dataBase64` opt-in field. Set `ImageExtractionConfig.includeDataBase64 = true` to receive a Base64-encoded copy of the image bytes alongside the raw `data`. Absent by default; wire-compatible with existing clients.
