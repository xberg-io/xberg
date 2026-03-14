package kreuzberg

import "encoding/json"

var metadataCoreKeys = map[string]struct{}{
	"title":                  {},
	"subject":                {},
	"authors":                {},
	"keywords":               {},
	"language":               {},
	"created_at":             {},
	"modified_at":            {},
	"created_by":             {},
	"modified_by":            {},
	"date":                   {},
	"producer":               {},
	"page_count":             {},
	"pages":                  {},
	"format_type":            {},
	"image_preprocessing":    {},
	"json_schema":            {},
	"error":                  {},
	"category":               {},
	"tags":                   {},
	"document_version":       {},
	"abstract_text":          {},
	"output_format":          {},
	"extraction_duration_ms": {},
}

var formatFieldSets = map[FormatType][]string{
	FormatPDF: {
		"title", "subject", "authors", "keywords", "created_at", "modified_at",
		"created_by", "producer", "page_count", "pdf_version", "is_encrypted",
		"width", "height", "summary",
	},
	FormatExcel:   {"sheet_count", "sheet_names"},
	FormatEmail:   {"from_email", "from_name", "to_emails", "cc_emails", "bcc_emails", "message_id", "attachments"},
	FormatPPTX:    {"title", "author", "description", "summary", "fonts"},
	FormatArchive: {"format", "file_count", "file_list", "total_size", "compressed_size"},
	FormatImage:   {"width", "height", "format", "exif"},
	FormatXML:     {"element_count", "unique_elements"},
	FormatText:    {"line_count", "word_count", "character_count", "headers", "links", "code_blocks"},
	FormatHTML: {
		"title", "description", "keywords", "author", "canonical_url", "base_href",
		"language", "text_direction", "open_graph", "twitter_card", "meta_tags",
		"headers", "links", "images", "structured_data",
	},
	FormatOCR: {"language", "psm", "output_format", "table_count", "table_rows", "table_cols"},
}

func decodeRawString(raw map[string]json.RawMessage, key string) *string {
	value, exists := raw[key]
	if !exists {
		return nil
	}
	var out string
	if err := json.Unmarshal(value, &out); err != nil {
		return nil
	}
	return &out
}

func decodeRawStringSlice(raw map[string]json.RawMessage, key string) []string {
	value, exists := raw[key]
	if !exists {
		return nil
	}
	var out []string
	if err := json.Unmarshal(value, &out); err != nil {
		return nil
	}
	return out
}

func (m *Metadata) decodeCoreFields(raw map[string]json.RawMessage) {
	m.Title = decodeRawString(raw, "title")
	m.Subject = decodeRawString(raw, "subject")
	m.Authors = decodeRawStringSlice(raw, "authors")
	m.Keywords = decodeRawStringSlice(raw, "keywords")
	// If keywords field contains objects (from keyword extraction), try to extract text values
	if m.Keywords == nil {
		if value, exists := raw["keywords"]; exists {
			var keywordObjects []struct {
				Text string `json:"text"`
			}
			if err := json.Unmarshal(value, &keywordObjects); err == nil && len(keywordObjects) > 0 {
				texts := make([]string, 0, len(keywordObjects))
				for _, kw := range keywordObjects {
					if kw.Text != "" {
						texts = append(texts, kw.Text)
					}
				}
				if len(texts) > 0 {
					m.Keywords = texts
				}
			}
		}
	}
	m.Language = decodeRawString(raw, "language")
	m.CreatedAt = decodeRawString(raw, "created_at")
	m.ModifiedAt = decodeRawString(raw, "modified_at")
	m.CreatedBy = decodeRawString(raw, "created_by")
	m.ModifiedBy = decodeRawString(raw, "modified_by")
	m.Date = decodeRawString(raw, "date")
	m.Producer = decodeRawString(raw, "producer")
	if value, exists := raw["page_count"]; exists {
		var pc int
		if err := json.Unmarshal(value, &pc); err == nil {
			m.PageCount = &pc
		}
	}
	m.Category = decodeRawString(raw, "category")
	m.Tags = decodeRawStringSlice(raw, "tags")
	m.DocumentVersion = decodeRawString(raw, "document_version")
	m.AbstractText = decodeRawString(raw, "abstract_text")
	m.OutputFormat = decodeRawString(raw, "output_format")
	if value, exists := raw["extraction_duration_ms"]; exists {
		var dur uint64
		if err := json.Unmarshal(value, &dur); err == nil {
			m.ExtractionDurationMs = &dur
		}
	}
}

func (m *Metadata) decodeStructuredFields(raw map[string]json.RawMessage) {
	if value, ok := raw["pages"]; ok {
		var pages PageStructure
		if err := json.Unmarshal(value, &pages); err == nil {
			m.Pages = &pages
		}
	}
	if value, ok := raw["image_preprocessing"]; ok {
		var meta ImagePreprocessingMetadata
		if err := json.Unmarshal(value, &meta); err == nil {
			m.ImagePreprocessing = &meta
		}
	}
	if value, ok := raw["json_schema"]; ok {
		m.JSONSchema = value
	}
	if value, ok := raw["error"]; ok {
		var errMeta ErrorMetadata
		if err := json.Unmarshal(value, &errMeta); err == nil {
			m.Error = &errMeta
		}
	}
	if value, ok := raw["format_type"]; ok {
		var format string
		if err := json.Unmarshal(value, &format); err == nil {
			m.Format.Type = FormatType(format)
		}
	}
}

// UnmarshalJSON ensures Metadata captures flattened format unions and additional custom fields.
func (m *Metadata) UnmarshalJSON(data []byte) error {
	raw := map[string]json.RawMessage{}
	if err := json.Unmarshal(data, &raw); err != nil {
		return err
	}

	m.decodeCoreFields(raw)
	m.decodeStructuredFields(raw)

	if err := m.decodeFormat(data); err != nil {
		return err
	}

	recognized := map[string]struct{}{}
	for key := range metadataCoreKeys {
		recognized[key] = struct{}{}
	}
	for _, field := range formatFieldSets[m.Format.Type] {
		recognized[field] = struct{}{}
	}

	m.Additional = make(map[string]json.RawMessage)
	for key, value := range raw {
		if _, ok := recognized[key]; ok {
			continue
		}
		m.Additional[key] = value
	}
	if len(m.Additional) == 0 {
		m.Additional = nil
	}

	return nil
}

// MarshalJSON reserializes Metadata back into the flattened JSON structure that
// the Rust core produces so round-tripping preserves the original payload.
func (m Metadata) MarshalJSON() ([]byte, error) {
	out := make(map[string]any)

	if m.Title != nil {
		out["title"] = *m.Title
	}
	if m.Subject != nil {
		out["subject"] = *m.Subject
	}
	if len(m.Authors) > 0 {
		out["authors"] = m.Authors
	}
	if len(m.Keywords) > 0 {
		out["keywords"] = m.Keywords
	}
	if m.Language != nil {
		out["language"] = *m.Language
	}
	if m.CreatedAt != nil {
		out["created_at"] = *m.CreatedAt
	}
	if m.ModifiedAt != nil {
		out["modified_at"] = *m.ModifiedAt
	}
	if m.CreatedBy != nil {
		out["created_by"] = *m.CreatedBy
	}
	if m.ModifiedBy != nil {
		out["modified_by"] = *m.ModifiedBy
	}
	if m.Date != nil {
		out["date"] = *m.Date
	}
	if m.Producer != nil {
		out["producer"] = *m.Producer
	}
	if m.PageCount != nil {
		out["page_count"] = *m.PageCount
	}
	if m.Pages != nil {
		out["pages"] = m.Pages
	}
	if m.ImagePreprocessing != nil {
		out["image_preprocessing"] = m.ImagePreprocessing
	}
	if m.JSONSchema != nil {
		out["json_schema"] = json.RawMessage(m.JSONSchema)
	}
	if m.Error != nil {
		out["error"] = m.Error
	}
	if m.Category != nil {
		out["category"] = *m.Category
	}
	if len(m.Tags) > 0 {
		out["tags"] = m.Tags
	}
	if m.DocumentVersion != nil {
		out["document_version"] = *m.DocumentVersion
	}
	if m.AbstractText != nil {
		out["abstract_text"] = *m.AbstractText
	}
	if m.OutputFormat != nil {
		out["output_format"] = *m.OutputFormat
	}
	if m.ExtractionDurationMs != nil {
		out["extraction_duration_ms"] = *m.ExtractionDurationMs
	}

	formatFields, err := m.encodeFormat()
	if err != nil {
		return nil, err
	}
	for key, value := range formatFields {
		out[key] = value
	}

	for key, value := range m.Additional {
		out[key] = json.RawMessage(value)
	}

	return json.Marshal(out)
}

// UnmarshalJSON implements lenient JSON unmarshaling for PdfMetadata.
// When keyword extraction is enabled, the flattened JSON may contain keyword
// objects ([{text, score, ...}]) in the "keywords" field instead of simple
// strings, which causes standard decoding to fail. This method falls back to
// field-by-field decoding to recover all other fields.
func (p *PdfMetadata) UnmarshalJSON(data []byte) error {
	type Alias PdfMetadata
	var alias Alias
	if err := json.Unmarshal(data, &alias); err == nil {
		*p = PdfMetadata(alias)
		return nil
	}

	// Standard decode failed; decode field-by-field, skipping type mismatches.
	raw := map[string]json.RawMessage{}
	if err := json.Unmarshal(data, &raw); err != nil {
		return err
	}

	tryUnmarshal := func(key string, target any) {
		if v, ok := raw[key]; ok {
			if err := json.Unmarshal(v, target); err != nil {
				// Intentionally ignore type-mismatch errors in fallback decoding.
				_ = err
			}
		}
	}

	tryUnmarshal("title", &p.Title)
	tryUnmarshal("subject", &p.Subject)
	tryUnmarshal("authors", &p.Authors)
	tryUnmarshal("keywords", &p.Keywords)
	tryUnmarshal("created_at", &p.CreatedAt)
	tryUnmarshal("modified_at", &p.ModifiedAt)
	tryUnmarshal("created_by", &p.CreatedBy)
	tryUnmarshal("producer", &p.Producer)
	tryUnmarshal("page_count", &p.PageCount)
	tryUnmarshal("pdf_version", &p.PDFVersion)
	tryUnmarshal("is_encrypted", &p.IsEncrypted)
	tryUnmarshal("width", &p.Width)
	tryUnmarshal("height", &p.Height)
	tryUnmarshal("summary", &p.Summary)

	return nil
}

func (m *Metadata) decodeFormat(data []byte) error {
	switch m.Format.Type {
	case FormatPDF:
		var meta PdfMetadata
		if err := json.Unmarshal(data, &meta); err != nil {
			return err
		}
		m.Format.Pdf = &meta
	case FormatExcel:
		var meta ExcelMetadata
		if err := json.Unmarshal(data, &meta); err != nil {
			return err
		}
		m.Format.Excel = &meta
	case FormatEmail:
		var meta EmailMetadata
		if err := json.Unmarshal(data, &meta); err != nil {
			return err
		}
		m.Format.Email = &meta
	case FormatPPTX:
		var meta PptxMetadata
		if err := json.Unmarshal(data, &meta); err != nil {
			return err
		}
		m.Format.Pptx = &meta
	case FormatArchive:
		var meta ArchiveMetadata
		if err := json.Unmarshal(data, &meta); err != nil {
			return err
		}
		m.Format.Archive = &meta
	case FormatImage:
		var meta ImageMetadata
		if err := json.Unmarshal(data, &meta); err != nil {
			return err
		}
		m.Format.Image = &meta
	case FormatXML:
		var meta XMLMetadata
		if err := json.Unmarshal(data, &meta); err != nil {
			return err
		}
		m.Format.XML = &meta
	case FormatText:
		var meta TextMetadata
		if err := json.Unmarshal(data, &meta); err != nil {
			return err
		}
		m.Format.Text = &meta
	case FormatHTML:
		var meta HtmlMetadata
		if err := json.Unmarshal(data, &meta); err != nil {
			return err
		}
		m.Format.HTML = &meta
	case FormatOCR:
		var meta OcrMetadata
		if err := json.Unmarshal(data, &meta); err != nil {
			return err
		}
		m.Format.OCR = &meta
	default:
		m.Format.Type = FormatUnknown
	}
	return nil
}

func (m Metadata) encodeFormat() (map[string]json.RawMessage, error) {
	result := make(map[string]json.RawMessage)
	if m.Format.Type == FormatUnknown || m.Format.Type == "" {
		return result, nil
	}

	typeRaw, err := json.Marshal(m.Format.Type)
	if err != nil {
		return nil, err
	}
	result["format_type"] = json.RawMessage(typeRaw)

	var payload any
	switch m.Format.Type {
	case FormatPDF:
		payload = m.Format.Pdf
	case FormatExcel:
		payload = m.Format.Excel
	case FormatEmail:
		payload = m.Format.Email
	case FormatPPTX:
		payload = m.Format.Pptx
	case FormatArchive:
		payload = m.Format.Archive
	case FormatImage:
		payload = m.Format.Image
	case FormatXML:
		payload = m.Format.XML
	case FormatText:
		payload = m.Format.Text
	case FormatHTML:
		payload = m.Format.HTML
	case FormatOCR:
		payload = m.Format.OCR
	}

	if payload == nil {
		return result, nil
	}

	fields, err := encodeStructToRaw(payload)
	if err != nil {
		return nil, err
	}
	for key, value := range fields {
		result[key] = value
	}
	return result, nil
}

func encodeStructToRaw(value any) (map[string]json.RawMessage, error) {
	raw, err := json.Marshal(value)
	if err != nil {
		return nil, err
	}
	result := make(map[string]json.RawMessage)
	if err := json.Unmarshal(raw, &result); err != nil {
		return nil, err
	}
	return result, nil
}
