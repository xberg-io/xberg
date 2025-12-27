package kreuzberg_test

import (
	"encoding/json"
	"testing"

	kreuzberg "github.com/kreuzberg-dev/kreuzberg/packages/go/v4"
)

func TestConfigFromJSON(t *testing.T) {
	tests := []struct {
		name    string
		json    string
		wantErr bool
		check   func(t *testing.T, cfg *kreuzberg.ExtractionConfig)
	}{
		{
			name:    "empty string",
			json:    "",
			wantErr: true,
		},
		{
			name:    "valid minimal config",
			json:    "{}",
			wantErr: false,
			check: func(t *testing.T, cfg *kreuzberg.ExtractionConfig) {
				if cfg == nil {
					t.Fatal("config should not be nil")
				}
			},
		},
		{
			name:    "valid config with use_cache",
			json:    `{"use_cache": true}`,
			wantErr: false,
			check: func(t *testing.T, cfg *kreuzberg.ExtractionConfig) {
				if cfg == nil {
					t.Fatal("config should not be nil")
				}
				if cfg.UseCache == nil || !*cfg.UseCache {
					t.Error("UseCache should be true")
				}
			},
		},
		{
			name:    "valid config with force_ocr",
			json:    `{"force_ocr": true}`,
			wantErr: false,
			check: func(t *testing.T, cfg *kreuzberg.ExtractionConfig) {
				if cfg == nil {
					t.Fatal("config should not be nil")
				}
				if cfg.ForceOCR == nil || !*cfg.ForceOCR {
					t.Error("ForceOCR should be true")
				}
			},
		},
		{
			name:    "config with OCR backend",
			json:    `{"ocr": {"backend": "tesseract"}}`,
			wantErr: false,
			check: func(t *testing.T, cfg *kreuzberg.ExtractionConfig) {
				if cfg == nil {
					t.Fatal("config should not be nil")
				}
				if cfg.OCR == nil || cfg.OCR.Backend != "tesseract" {
					t.Error("OCR backend should be tesseract")
				}
			},
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			cfg, err := kreuzberg.ConfigFromJSON(tt.json)
			if (err != nil) != tt.wantErr {
				t.Errorf("ConfigFromJSON() error = %v, wantErr %v", err, tt.wantErr)
				return
			}
			if !tt.wantErr && tt.check != nil {
				tt.check(t, cfg)
			}
		})
	}
}

func TestIsValidJSON(t *testing.T) {
	tests := []struct {
		name  string
		json  string
		valid bool
	}{
		{
			name:  "empty string",
			json:  "",
			valid: false,
		},
		{
			name:  "valid empty object",
			json:  "{}",
			valid: true,
		},
		{
			name:  "valid boolean",
			json:  `{"use_cache": true}`,
			valid: true,
		},
		{
			name:  "invalid JSON",
			json:  `{invalid}`,
			valid: false,
		},
		{
			name:  "valid nested config",
			json:  `{"ocr": {"backend": "tesseract", "language": "eng"}}`,
			valid: true,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			valid := kreuzberg.IsValidJSON(tt.json)
			if valid != tt.valid {
				t.Errorf("IsValidJSON() = %v, want %v", valid, tt.valid)
			}
		})
	}
}

func TestConfigToJSON(t *testing.T) {
	tests := []struct {
		name    string
		config  *kreuzberg.ExtractionConfig
		wantErr bool
		check   func(t *testing.T, jsonStr string)
	}{
		{
			name:    "nil config",
			config:  nil,
			wantErr: true,
		},
		{
			name:    "empty config",
			config:  &kreuzberg.ExtractionConfig{},
			wantErr: false,
			check: func(t *testing.T, jsonStr string) {
				var m map[string]interface{}
				if err := json.Unmarshal([]byte(jsonStr), &m); err != nil {
					t.Errorf("invalid JSON output: %v", err)
				}
			},
		},
		{
			name: "config with use_cache",
			config: &kreuzberg.ExtractionConfig{
				UseCache: kreuzberg.BoolPtr(true),
			},
			wantErr: false,
			check: func(t *testing.T, jsonStr string) {
				var m map[string]interface{}
				if err := json.Unmarshal([]byte(jsonStr), &m); err != nil {
					t.Errorf("invalid JSON output: %v", err)
				}
				if useCache, ok := m["use_cache"].(bool); !ok || !useCache {
					t.Error("use_cache should be true in JSON")
				}
			},
		},
		{
			name: "config with nested OCR",
			config: &kreuzberg.ExtractionConfig{
				OCR: &kreuzberg.OCRConfig{
					Backend: "tesseract",
				},
			},
			wantErr: false,
			check: func(t *testing.T, jsonStr string) {
				var m map[string]interface{}
				if err := json.Unmarshal([]byte(jsonStr), &m); err != nil {
					t.Errorf("invalid JSON output: %v", err)
				}
				if ocrMap, ok := m["ocr"].(map[string]interface{}); !ok {
					t.Error("ocr should be present in JSON")
				} else if backend, ok := ocrMap["backend"].(string); !ok || backend != "tesseract" {
					t.Error("ocr.backend should be tesseract")
				}
			},
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			jsonStr, err := kreuzberg.ConfigToJSON(tt.config)
			if (err != nil) != tt.wantErr {
				t.Errorf("ConfigToJSON() error = %v, wantErr %v", err, tt.wantErr)
				return
			}
			if !tt.wantErr && tt.check != nil {
				tt.check(t, jsonStr)
			}
		})
	}
}

func TestConfigGetField(t *testing.T) {
	baseConfig := &kreuzberg.ExtractionConfig{
		UseCache: kreuzberg.BoolPtr(true),
		OCR: &kreuzberg.OCRConfig{
			Backend:  "tesseract",
			Language: kreuzberg.StringPtr("eng"),
		},
	}

	tests := []struct {
		name      string
		config    *kreuzberg.ExtractionConfig
		fieldName string
		wantErr   bool
		check     func(t *testing.T, value interface{})
	}{
		{
			name:      "nil config",
			config:    nil,
			fieldName: "use_cache",
			wantErr:   true,
		},
		{
			name:      "empty field name",
			config:    baseConfig,
			fieldName: "",
			wantErr:   true,
		},
		{
			name:      "valid simple field",
			config:    baseConfig,
			fieldName: "use_cache",
			wantErr:   false,
			check: func(t *testing.T, value interface{}) {
				if boolVal, ok := value.(bool); !ok || !boolVal {
					t.Errorf("expected true, got %v", value)
				}
			},
		},
		{
			name:      "nested field - ocr backend",
			config:    baseConfig,
			fieldName: "ocr",
			wantErr:   false,
			check: func(t *testing.T, value interface{}) {
				if value == nil {
					t.Error("ocr field should not be nil")
				}
			},
		},
		{
			name:      "non-existent field",
			config:    baseConfig,
			fieldName: "nonexistent",
			wantErr:   true,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			value, err := kreuzberg.ConfigGetField(tt.config, tt.fieldName)
			if (err != nil) != tt.wantErr {
				t.Errorf("ConfigGetField() error = %v, wantErr %v", err, tt.wantErr)
				return
			}
			if !tt.wantErr && tt.check != nil {
				tt.check(t, value)
			}
		})
	}
}

func TestConfigMerge(t *testing.T) {
	tests := []struct {
		name       string
		baseConfig *kreuzberg.ExtractionConfig
		override   *kreuzberg.ExtractionConfig
		wantErr    bool
		check      func(t *testing.T, merged *kreuzberg.ExtractionConfig)
	}{
		{
			name:       "nil base config",
			baseConfig: nil,
			override:   &kreuzberg.ExtractionConfig{},
			wantErr:    true,
		},
		{
			name:       "nil override config",
			baseConfig: &kreuzberg.ExtractionConfig{},
			override:   nil,
			wantErr:    true,
		},
		{
			name:       "merge empty into empty",
			baseConfig: &kreuzberg.ExtractionConfig{},
			override:   &kreuzberg.ExtractionConfig{},
			wantErr:    false,
			check: func(t *testing.T, merged *kreuzberg.ExtractionConfig) {
				if merged == nil {
					t.Fatal("merged config should not be nil")
				}
			},
		},
		{
			name: "merge use_cache into base",
			baseConfig: &kreuzberg.ExtractionConfig{
				ForceOCR: kreuzberg.BoolPtr(true),
			},
			override: &kreuzberg.ExtractionConfig{
				UseCache: kreuzberg.BoolPtr(true),
			},
			wantErr: false,
			check: func(t *testing.T, merged *kreuzberg.ExtractionConfig) {
				if merged.UseCache == nil || !*merged.UseCache {
					t.Error("UseCache should be true after merge")
				}
				if merged.ForceOCR == nil || !*merged.ForceOCR {
					t.Error("ForceOCR should remain true after merge")
				}
			},
		},
		{
			name: "override existing field",
			baseConfig: &kreuzberg.ExtractionConfig{
				UseCache: kreuzberg.BoolPtr(false),
			},
			override: &kreuzberg.ExtractionConfig{
				UseCache: kreuzberg.BoolPtr(true),
			},
			wantErr: false,
			check: func(t *testing.T, merged *kreuzberg.ExtractionConfig) {
				if merged.UseCache == nil || !*merged.UseCache {
					t.Error("UseCache should be overridden to true")
				}
			},
		},
		{
			name: "override to default value (use_cache: false -> true)",
			baseConfig: &kreuzberg.ExtractionConfig{
				UseCache: kreuzberg.BoolPtr(false),
			},
			override: &kreuzberg.ExtractionConfig{
				UseCache: kreuzberg.BoolPtr(true),
			},
			wantErr: false,
			check: func(t *testing.T, merged *kreuzberg.ExtractionConfig) {
				if merged.UseCache == nil || !*merged.UseCache {
					t.Error("UseCache should be overridden to true (default value)")
				}
			},
		},
		{
			name: "override to default value (force_ocr: false -> true)",
			baseConfig: &kreuzberg.ExtractionConfig{
				ForceOCR: kreuzberg.BoolPtr(false),
			},
			override: &kreuzberg.ExtractionConfig{
				ForceOCR: kreuzberg.BoolPtr(true),
			},
			wantErr: false,
			check: func(t *testing.T, merged *kreuzberg.ExtractionConfig) {
				if merged.ForceOCR == nil || !*merged.ForceOCR {
					t.Error("ForceOCR should be overridden to true")
				}
			},
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			var baseCopy *kreuzberg.ExtractionConfig
			if tt.baseConfig != nil {
				baseCopy = &kreuzberg.ExtractionConfig{}
				data, _ := json.Marshal(tt.baseConfig)
				json.Unmarshal(data, baseCopy)
			}

			err := kreuzberg.ConfigMerge(baseCopy, tt.override)
			if (err != nil) != tt.wantErr {
				t.Errorf("ConfigMerge() error = %v, wantErr %v", err, tt.wantErr)
				return
			}
			if !tt.wantErr && tt.check != nil {
				tt.check(t, baseCopy)
			}
		})
	}
}

func TestResultGetPageCount(t *testing.T) {
	tests := []struct {
		name      string
		result    *kreuzberg.ExtractionResult
		wantErr   bool
		wantCount int
	}{
		{
			name:      "nil result",
			result:    nil,
			wantErr:   false,
			wantCount: 0,
		},
		{
			name: "result with no pages",
			result: &kreuzberg.ExtractionResult{
				Content: "test",
			},
			wantErr:   false,
			wantCount: 0,
		},
		{
			name: "result with page structure",
			result: &kreuzberg.ExtractionResult{
				Content: "test",
				Metadata: kreuzberg.Metadata{
					PageStructure: &kreuzberg.PageStructure{
						TotalCount: 42,
					},
				},
			},
			wantErr:   false,
			wantCount: 42,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			if tt.result == nil {
				t.Skip("skipping nil result test")
			}

			count, err := tt.result.GetPageCount()
			if (err != nil) != tt.wantErr {
				t.Errorf("GetPageCount() error = %v, wantErr %v", err, tt.wantErr)
				return
			}
			if count != tt.wantCount {
				t.Errorf("GetPageCount() = %d, want %d", count, tt.wantCount)
			}
		})
	}
}

func TestResultGetChunkCount(t *testing.T) {
	tests := []struct {
		name      string
		result    *kreuzberg.ExtractionResult
		wantErr   bool
		wantCount int
	}{
		{
			name: "result with no chunks",
			result: &kreuzberg.ExtractionResult{
				Content: "test",
			},
			wantErr:   false,
			wantCount: 0,
		},
		{
			name: "result with chunks",
			result: &kreuzberg.ExtractionResult{
				Content: "test",
				Chunks: []kreuzberg.Chunk{
					{
						Content: "chunk1",
						Metadata: kreuzberg.ChunkMetadata{
							ChunkIndex:  0,
							TotalChunks: 3,
						},
					},
					{
						Content: "chunk2",
						Metadata: kreuzberg.ChunkMetadata{
							ChunkIndex:  1,
							TotalChunks: 3,
						},
					},
					{
						Content: "chunk3",
						Metadata: kreuzberg.ChunkMetadata{
							ChunkIndex:  2,
							TotalChunks: 3,
						},
					},
				},
			},
			wantErr:   false,
			wantCount: 3,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			count, err := tt.result.GetChunkCount()
			if (err != nil) != tt.wantErr {
				t.Errorf("GetChunkCount() error = %v, wantErr %v", err, tt.wantErr)
				return
			}
			if count != tt.wantCount {
				t.Errorf("GetChunkCount() = %d, want %d", count, tt.wantCount)
			}
		})
	}
}

func TestResultGetDetectedLanguage(t *testing.T) {
	tests := []struct {
		name         string
		result       *kreuzberg.ExtractionResult
		wantErr      bool
		wantLanguage string
	}{
		{
			name: "result with no language",
			result: &kreuzberg.ExtractionResult{
				Content: "test",
			},
			wantErr:      false,
			wantLanguage: "",
		},
		{
			name: "result with metadata language",
			result: &kreuzberg.ExtractionResult{
				Content: "test",
				Metadata: kreuzberg.Metadata{
					Language: kreuzberg.StringPtr("en"),
				},
			},
			wantErr:      false,
			wantLanguage: "en",
		},
		{
			name: "result with detected languages",
			result: &kreuzberg.ExtractionResult{
				Content:           "test",
				DetectedLanguages: []string{"de", "fr"},
			},
			wantErr:      false,
			wantLanguage: "de",
		},
		{
			name: "metadata language takes precedence",
			result: &kreuzberg.ExtractionResult{
				Content: "test",
				Metadata: kreuzberg.Metadata{
					Language: kreuzberg.StringPtr("en"),
				},
				DetectedLanguages: []string{"de", "fr"},
			},
			wantErr:      false,
			wantLanguage: "en",
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			lang, err := tt.result.GetDetectedLanguage()
			if (err != nil) != tt.wantErr {
				t.Errorf("GetDetectedLanguage() error = %v, wantErr %v", err, tt.wantErr)
				return
			}
			if lang != tt.wantLanguage {
				t.Errorf("GetDetectedLanguage() = %q, want %q", lang, tt.wantLanguage)
			}
		})
	}
}

func TestResultGetMetadataField(t *testing.T) {
	tests := []struct {
		name      string
		result    *kreuzberg.ExtractionResult
		fieldName string
		wantErr   bool
		check     func(t *testing.T, field *kreuzberg.MetadataField)
	}{
		{
			name: "empty field name",
			result: &kreuzberg.ExtractionResult{
				Content: "test",
			},
			fieldName: "",
			wantErr:   true,
		},
		{
			name: "language field exists",
			result: &kreuzberg.ExtractionResult{
				Content: "test",
				Metadata: kreuzberg.Metadata{
					Language: kreuzberg.StringPtr("en"),
				},
			},
			fieldName: "language",
			wantErr:   false,
			check: func(t *testing.T, field *kreuzberg.MetadataField) {
				if field.IsNull {
					t.Error("language field should not be null")
				}
				if field.Name != "language" {
					t.Errorf("field name should be 'language', got %q", field.Name)
				}
			},
		},
		{
			name: "language field missing",
			result: &kreuzberg.ExtractionResult{
				Content:  "test",
				Metadata: kreuzberg.Metadata{},
			},
			fieldName: "language",
			wantErr:   false,
			check: func(t *testing.T, field *kreuzberg.MetadataField) {
				if !field.IsNull {
					t.Error("language field should be null when missing")
				}
			},
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			field, err := tt.result.GetMetadataField(tt.fieldName)
			if (err != nil) != tt.wantErr {
				t.Errorf("GetMetadataField() error = %v, wantErr %v", err, tt.wantErr)
				return
			}
			if !tt.wantErr && tt.check != nil {
				tt.check(t, field)
			}
		})
	}
}

func TestResultToJSON(t *testing.T) {
	result := &kreuzberg.ExtractionResult{
		Content:  "test content",
		MimeType: "text/plain",
		Success:  true,
	}

	jsonStr, err := kreuzberg.ResultToJSON(result)
	if err != nil {
		t.Fatalf("ResultToJSON() error = %v", err)
	}

	var parsed kreuzberg.ExtractionResult
	if err := json.Unmarshal([]byte(jsonStr), &parsed); err != nil {
		t.Fatalf("failed to parse JSON: %v", err)
	}

	if parsed.Content != result.Content {
		t.Errorf("Content mismatch: %q != %q", parsed.Content, result.Content)
	}
	if parsed.MimeType != result.MimeType {
		t.Errorf("MimeType mismatch: %q != %q", parsed.MimeType, result.MimeType)
	}
}

func TestResultFromJSON(t *testing.T) {
	jsonStr := `{
		"content": "test content",
		"mime_type": "text/plain",
		"metadata": {},
		"tables": [],
		"success": true
	}`

	result, err := kreuzberg.ResultFromJSON(jsonStr)
	if err != nil {
		t.Fatalf("ResultFromJSON() error = %v", err)
	}

	if result.Content != "test content" {
		t.Errorf("Content should be 'test content', got %q", result.Content)
	}
	if result.MimeType != "text/plain" {
		t.Errorf("MimeType should be 'text/plain', got %q", result.MimeType)
	}
	if !result.Success {
		t.Error("Success should be true")
	}
}
