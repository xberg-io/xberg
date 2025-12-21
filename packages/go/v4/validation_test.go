package kreuzberg

import (
	"testing"
)

func TestValidateBinarizationMethodValid(t *testing.T) {
	validMethods := []string{"otsu", "adaptive", "sauvola"}
	for _, method := range validMethods {
		if err := ValidateBinarizationMethod(method); err != nil {
			t.Fatalf("expected valid method %s, got error: %v", method, err)
		}
	}
}

func TestValidateBinarizationMethodInvalid(t *testing.T) {
	if err := ValidateBinarizationMethod("invalid_method"); err == nil {
		t.Fatalf("expected error for invalid binarization method")
	}
}

func TestValidateBinarizationMethodEmpty(t *testing.T) {
	err := ValidateBinarizationMethod("")
	if err == nil {
		t.Fatalf("expected error for empty binarization method")
	}
	if _, ok := err.(*ValidationError); !ok {
		t.Fatalf("expected ValidationError, got %T", err)
	}
}

func TestValidateOCRBackendValid(t *testing.T) {
	validBackends := []string{"tesseract", "easyocr", "paddleocr"}
	for _, backend := range validBackends {
		if err := ValidateOCRBackend(backend); err != nil {
			t.Fatalf("expected valid backend %s, got error: %v", backend, err)
		}
	}
}

func TestValidateOCRBackendInvalid(t *testing.T) {
	if err := ValidateOCRBackend("invalid_backend"); err == nil {
		t.Fatalf("expected error for invalid OCR backend")
	}
}

func TestValidateOCRBackendEmpty(t *testing.T) {
	if err := ValidateOCRBackend(""); err == nil {
		t.Fatalf("expected error for empty OCR backend")
	}
}

func TestValidateLanguageCodeValid(t *testing.T) {
	validCodes := []string{"en", "eng", "de", "deu", "fr", "fra"}
	for _, code := range validCodes {
		if err := ValidateLanguageCode(code); err != nil {
			t.Fatalf("expected valid language code %s, got error: %v", code, err)
		}
	}
}

func TestValidateLanguageCodeInvalid(t *testing.T) {
	if err := ValidateLanguageCode("invalid_code_xxxxxx"); err == nil {
		t.Fatalf("expected error for invalid language code")
	}
}

func TestValidateLanguageCodeEmpty(t *testing.T) {
	if err := ValidateLanguageCode(""); err == nil {
		t.Fatalf("expected error for empty language code")
	}
}

func TestValidateTokenReductionLevelValid(t *testing.T) {
	validLevels := []string{"off", "light", "moderate", "aggressive"}
	for _, level := range validLevels {
		if err := ValidateTokenReductionLevel(level); err != nil {
			t.Fatalf("expected valid token reduction level %s, got error: %v", level, err)
		}
	}
}

func TestValidateTokenReductionLevelInvalid(t *testing.T) {
	if err := ValidateTokenReductionLevel("invalid_level"); err == nil {
		t.Fatalf("expected error for invalid token reduction level")
	}
}

func TestValidateTokenReductionLevelEmpty(t *testing.T) {
	if err := ValidateTokenReductionLevel(""); err == nil {
		t.Fatalf("expected error for empty token reduction level")
	}
}

func TestValidateTesseractPSMValid(t *testing.T) {
	validPSMs := []int{0, 1, 3, 6, 11, 13}
	for _, psm := range validPSMs {
		if err := ValidateTesseractPSM(psm); err != nil {
			t.Fatalf("expected valid PSM %d, got error: %v", psm, err)
		}
	}
}

func TestValidateTesseractPSMInvalid(t *testing.T) {
	if err := ValidateTesseractPSM(14); err == nil {
		t.Fatalf("expected error for PSM value 14 (out of range)")
	}
	if err := ValidateTesseractPSM(-1); err == nil {
		t.Fatalf("expected error for PSM value -1 (negative)")
	}
}

func TestValidateTesseractOEMValid(t *testing.T) {
	validOEMs := []int{0, 1, 2, 3}
	for _, oem := range validOEMs {
		if err := ValidateTesseractOEM(oem); err != nil {
			t.Fatalf("expected valid OEM %d, got error: %v", oem, err)
		}
	}
}

func TestValidateTesseractOEMInvalid(t *testing.T) {
	if err := ValidateTesseractOEM(4); err == nil {
		t.Fatalf("expected error for OEM value 4 (out of range)")
	}
	if err := ValidateTesseractOEM(-1); err == nil {
		t.Fatalf("expected error for OEM value -1 (negative)")
	}
}

func TestValidateOutputFormatValid(t *testing.T) {
	validFormats := []string{"text", "markdown"}
	for _, format := range validFormats {
		if err := ValidateOutputFormat(format); err != nil {
			t.Fatalf("expected valid format %s, got error: %v", format, err)
		}
	}
}

func TestValidateOutputFormatInvalid(t *testing.T) {
	if err := ValidateOutputFormat("invalid_format"); err == nil {
		t.Fatalf("expected error for invalid output format")
	}
}

func TestValidateOutputFormatEmpty(t *testing.T) {
	if err := ValidateOutputFormat(""); err == nil {
		t.Fatalf("expected error for empty output format")
	}
}

func TestValidateConfidenceValid(t *testing.T) {
	validConfidences := []float64{0.0, 0.5, 1.0}
	for _, conf := range validConfidences {
		if err := ValidateConfidence(conf); err != nil {
			t.Fatalf("expected valid confidence %.2f, got error: %v", conf, err)
		}
	}
}

func TestValidateConfidenceInvalid(t *testing.T) {
	invalidConfidences := []float64{-0.1, 1.1, 2.0}
	for _, conf := range invalidConfidences {
		if err := ValidateConfidence(conf); err == nil {
			t.Fatalf("expected error for invalid confidence %.2f", conf)
		}
	}
}

func TestValidateDPIValid(t *testing.T) {
	validDPIs := []int{72, 150, 300, 600}
	for _, dpi := range validDPIs {
		if err := ValidateDPI(dpi); err != nil {
			t.Fatalf("expected valid DPI %d, got error: %v", dpi, err)
		}
	}
}

func TestValidateDPIInvalid(t *testing.T) {
	invalidDPIs := []int{0, -1, -100}
	for _, dpi := range invalidDPIs {
		if err := ValidateDPI(dpi); err == nil {
			t.Fatalf("expected error for invalid DPI %d", dpi)
		}
	}
}

func TestValidateChunkingParamsValid(t *testing.T) {
	validParams := [][2]int{
		{100, 0},
		{1000, 100},
		{5000, 500},
	}
	for _, params := range validParams {
		if err := ValidateChunkingParams(params[0], params[1]); err != nil {
			t.Fatalf("expected valid chunking params (max_chars=%d, max_overlap=%d), got error: %v", params[0], params[1], err)
		}
	}
}

func TestValidateChunkingParamsInvalid(t *testing.T) {
	testCases := []struct {
		maxChars   int
		maxOverlap int
		expectErr  bool
	}{
		{0, 0, true},
		{-1, 0, true},
		{100, 100, true},
		{100, 200, true},
		{100, -1, true},
	}
	for _, tc := range testCases {
		if err := ValidateChunkingParams(tc.maxChars, tc.maxOverlap); err == nil && tc.expectErr {
			t.Fatalf("expected error for chunking params (max_chars=%d, max_overlap=%d)", tc.maxChars, tc.maxOverlap)
		}
	}
}

func TestGetValidBinarizationMethods(t *testing.T) {
	methods, err := GetValidBinarizationMethods()
	if err != nil {
		t.Fatalf("failed to get valid binarization methods: %v", err)
	}
	if len(methods) == 0 {
		t.Fatalf("expected non-empty binarization methods list")
	}
	if methods[0] == "" {
		t.Fatalf("expected non-empty method name in list")
	}
}

func TestGetValidLanguageCodes(t *testing.T) {
	codes, err := GetValidLanguageCodes()
	if err != nil {
		t.Fatalf("failed to get valid language codes: %v", err)
	}
	if len(codes) == 0 {
		t.Fatalf("expected non-empty language codes list")
	}
	if codes[0] == "" {
		t.Fatalf("expected non-empty language code in list")
	}
}

func TestGetValidOCRBackends(t *testing.T) {
	backends, err := GetValidOCRBackends()
	if err != nil {
		t.Fatalf("failed to get valid OCR backends: %v", err)
	}
	if len(backends) == 0 {
		t.Fatalf("expected non-empty OCR backends list")
	}
	if backends[0] == "" {
		t.Fatalf("expected non-empty backend name in list")
	}
}

func TestGetValidTokenReductionLevels(t *testing.T) {
	levels, err := GetValidTokenReductionLevels()
	if err != nil {
		t.Fatalf("failed to get valid token reduction levels: %v", err)
	}
	if len(levels) == 0 {
		t.Fatalf("expected non-empty token reduction levels list")
	}
	if levels[0] == "" {
		t.Fatalf("expected non-empty level name in list")
	}
}
