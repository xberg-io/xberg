```go title="Go"
package main

import "github.com/kreuzberg-dev/kreuzberg/packages/go/v5"

func main() {
	enabled := true
	includeBbox := true
	kClusters := uint(6)
	kClustersAdvanced := uint(12)
	threshold := float32(0.8)

	// Basic hierarchy configuration
	config := kreuzberg.ExtractionConfig{
		PdfOptions: &kreuzberg.PdfConfig{
			ExtractImages: true,
			Hierarchy: &kreuzberg.HierarchyConfig{
				Enabled:              &enabled,
				KClusters:            &kClusters,
				IncludeBbox:          &includeBbox,
				OcrCoverageThreshold: &threshold,
			},
		},
	}

	// Advanced hierarchy configuration with more clusters
	advancedConfig := kreuzberg.ExtractionConfig{
		PdfOptions: &kreuzberg.PdfConfig{
			ExtractImages: true,
			Hierarchy: &kreuzberg.HierarchyConfig{
				Enabled:              &enabled,
				KClusters:            &kClustersAdvanced,
				IncludeBbox:          &includeBbox,
				OcrCoverageThreshold: &threshold,
			},
		},
	}

	_ = config
	_ = advancedConfig
}
```
