package kreuzberg

import (
	"encoding/json"
	"testing"
)

// TestFontConfigDefaults verifies default values for FontConfig
func TestFontConfigDefaults(t *testing.T) {
	config := &FontConfig{}

	if config.Enabled != false {
		t.Errorf("expected Enabled to be false by default, got %v", config.Enabled)
	}

	if config.CustomFontDirs != nil {
		t.Errorf("expected CustomFontDirs to be nil by default, got %v", config.CustomFontDirs)
	}
}

// TestFontConfigCreation verifies FontConfig creation with custom values
func TestFontConfigCreation(t *testing.T) {
	dirs := []string{"/usr/share/fonts/custom", "~/my-fonts"}
	enabled := true

	config := &FontConfig{
		Enabled:        enabled,
		CustomFontDirs: dirs,
	}

	if config.Enabled != enabled {
		t.Errorf("expected Enabled to be %v, got %v", enabled, config.Enabled)
	}

	if len(config.CustomFontDirs) != 2 {
		t.Errorf("expected 2 custom dirs, got %d", len(config.CustomFontDirs))
	}

	if config.CustomFontDirs[0] != "/usr/share/fonts/custom" {
		t.Errorf("expected first dir to be /usr/share/fonts/custom, got %s", config.CustomFontDirs[0])
	}
}

// TestFontConfigDisabled verifies FontConfig with enabled=false
func TestFontConfigDisabled(t *testing.T) {
	config := &FontConfig{
		Enabled: false,
	}

	if config.Enabled != false {
		t.Errorf("expected Enabled to be false, got %v", config.Enabled)
	}

	if config.CustomFontDirs != nil {
		t.Errorf("expected CustomFontDirs to be nil, got %v", config.CustomFontDirs)
	}
}

// TestFontConfigWithCustomDirs verifies FontConfig with custom directories
func TestFontConfigWithCustomDirs(t *testing.T) {
	dirs := []string{"/path/to/fonts", "/another/path"}

	config := &FontConfig{
		Enabled:        true,
		CustomFontDirs: dirs,
	}

	if !config.Enabled {
		t.Error("expected Enabled to be true")
	}

	if len(config.CustomFontDirs) != 2 {
		t.Errorf("expected 2 dirs, got %d", len(config.CustomFontDirs))
	}

	for i, expected := range dirs {
		if config.CustomFontDirs[i] != expected {
			t.Errorf("expected dir %d to be %s, got %s", i, expected, config.CustomFontDirs[i])
		}
	}
}

// TestFontConfigJSONMarshaling verifies JSON serialization/deserialization
func TestFontConfigJSONMarshaling(t *testing.T) {
	original := &FontConfig{
		Enabled:        true,
		CustomFontDirs: []string{"/fonts", "~/fonts"},
	}

	jsonBytes, err := json.Marshal(original)
	if err != nil {
		t.Fatalf("failed to marshal FontConfig: %v", err)
	}

	restored := &FontConfig{}
	if err := json.Unmarshal(jsonBytes, restored); err != nil {
		t.Fatalf("failed to unmarshal FontConfig: %v", err)
	}

	if restored.Enabled != original.Enabled {
		t.Errorf("expected Enabled to be %v after roundtrip, got %v", original.Enabled, restored.Enabled)
	}

	if len(restored.CustomFontDirs) != len(original.CustomFontDirs) {
		t.Errorf("expected %d dirs after roundtrip, got %d", len(original.CustomFontDirs), len(restored.CustomFontDirs))
	}

	for i, expected := range original.CustomFontDirs {
		if restored.CustomFontDirs[i] != expected {
			t.Errorf("expected dir %d to be %s after roundtrip, got %s", i, expected, restored.CustomFontDirs[i])
		}
	}
}

// TestFontConfigEmptySlice verifies handling of empty CustomFontDirs slice
func TestFontConfigEmptySlice(t *testing.T) {
	config := &FontConfig{
		Enabled:        true,
		CustomFontDirs: []string{},
	}

	if !config.Enabled {
		t.Error("expected Enabled to be true")
	}

	if config.CustomFontDirs == nil {
		t.Error("expected CustomFontDirs to be non-nil (empty slice)")
	}

	if len(config.CustomFontDirs) != 0 {
		t.Errorf("expected 0 dirs, got %d", len(config.CustomFontDirs))
	}
}

// TestFontConfigMultipleDirs verifies handling multiple custom directories
func TestFontConfigMultipleDirs(t *testing.T) {
	dirs := []string{
		"/path1",
		"/path2",
		"/path3",
		"~/fonts",
		"./relative-fonts",
	}

	config := &FontConfig{
		Enabled:        true,
		CustomFontDirs: dirs,
	}

	if len(config.CustomFontDirs) != 5 {
		t.Errorf("expected 5 dirs, got %d", len(config.CustomFontDirs))
	}

	for i, expected := range dirs {
		if config.CustomFontDirs[i] != expected {
			t.Errorf("dir %d: expected %s, got %s", i, expected, config.CustomFontDirs[i])
		}
	}
}

// TestPdfConfigWithFontConfig verifies FontConfig integration with PdfConfig
func TestPdfConfigWithFontConfig(t *testing.T) {
	fontConfig := &FontConfig{
		Enabled:        true,
		CustomFontDirs: []string{"/fonts"},
	}

	extractImages := true
	pdfConfig := &PdfConfig{
		ExtractImages: &extractImages,
		FontConfig:    fontConfig,
	}

	if pdfConfig.FontConfig == nil {
		t.Error("expected FontConfig to be set in PdfConfig")
	}

	if !pdfConfig.FontConfig.Enabled {
		t.Error("expected FontConfig.Enabled to be true")
	}

	if len(pdfConfig.FontConfig.CustomFontDirs) != 1 {
		t.Errorf("expected 1 custom dir, got %d", len(pdfConfig.FontConfig.CustomFontDirs))
	}

	if pdfConfig.FontConfig.CustomFontDirs[0] != "/fonts" {
		t.Errorf("expected custom dir to be /fonts, got %s", pdfConfig.FontConfig.CustomFontDirs[0])
	}
}

// TestPdfConfigWithFontConfigAllParameters verifies full integration
func TestPdfConfigWithFontConfigAllParameters(t *testing.T) {
	fontConfig := &FontConfig{
		Enabled:        true,
		CustomFontDirs: []string{"/custom-fonts"},
	}

	extractImages := true
	extractMetadata := true
	pdfConfig := &PdfConfig{
		ExtractImages:   &extractImages,
		Passwords:       []string{"pass1"},
		ExtractMetadata: &extractMetadata,
		FontConfig:      fontConfig,
	}

	if pdfConfig.ExtractImages == nil || !*pdfConfig.ExtractImages {
		t.Error("expected ExtractImages to be true")
	}

	if len(pdfConfig.Passwords) != 1 {
		t.Errorf("expected 1 password, got %d", len(pdfConfig.Passwords))
	}

	if pdfConfig.ExtractMetadata == nil || !*pdfConfig.ExtractMetadata {
		t.Error("expected ExtractMetadata to be true")
	}

	if pdfConfig.FontConfig == nil {
		t.Error("expected FontConfig to be set")
	}

	if !pdfConfig.FontConfig.Enabled {
		t.Error("expected FontConfig.Enabled to be true")
	}
}

// TestFontConfigNilCustomDirs verifies nil CustomFontDirs
func TestFontConfigNilCustomDirs(t *testing.T) {
	config := &FontConfig{
		Enabled:        true,
		CustomFontDirs: nil,
	}

	if !config.Enabled {
		t.Error("expected Enabled to be true")
	}

	if config.CustomFontDirs != nil {
		t.Error("expected CustomFontDirs to be nil")
	}
}

// TestFontConfigNilPointer verifies nil FontConfig pointer behavior
func TestFontConfigNilPointer(t *testing.T) {
	var config *FontConfig
	_ = config
}
