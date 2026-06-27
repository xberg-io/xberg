package main

import (
	"fmt"
	"xberg"
)

// Example 1: Basic YAKE configuration
// Uses YAKE algorithm with default parameters and English stopword filtering
func basicYake() error {
	config := &xberg.ExtractionConfig{
		Keywords: &xberg.KeywordConfig{
			Algorithm:   "yake",
			MaxKeywords: 10,
			MinScore:    0.0,
			NgramRange:  [2]int{1, 3},
			Language:    "en",
			YakeParams:  nil,
			RakeParams:  nil,
		},
	}

	result, err := xberg.ExtractSync("document.pdf", config)
	if err != nil {
		return err
	}

	fmt.Printf("Keywords: %v\n", result.Keywords)
	return nil
}

// Example 2: Advanced YAKE with custom parameters
// Fine-tunes YAKE with custom window size for co-occurrence analysis
func advancedYake() error {
	config := &xberg.ExtractionConfig{
		Keywords: &xberg.KeywordConfig{
			Algorithm:   "yake",
			MaxKeywords: 15,
			MinScore:    0.1,
			NgramRange:  [2]int{1, 2},
			Language:    "en",
			YakeParams: &xberg.YakeParams{
				WindowSize: 1,
			},
			RakeParams: nil,
		},
	}

	result, err := xberg.ExtractSync("document.pdf", config)
	if err != nil {
		return err
	}

	fmt.Printf("Keywords: %v\n", result.Keywords)
	return nil
}

// Example 3: RAKE configuration
// Uses RAKE algorithm for rapid keyword extraction with phrase constraints
func rakeConfig() error {
	config := &xberg.ExtractionConfig{
		Keywords: &xberg.KeywordConfig{
			Algorithm:   "rake",
			MaxKeywords: 10,
			MinScore:    5.0,
			NgramRange:  [2]int{1, 3},
			Language:    "en",
			YakeParams:  nil,
			RakeParams: &xberg.RakeParams{
				MinWordLength:      1,
				MaxWordsPerPhrase:  3,
			},
		},
	}

	result, err := xberg.ExtractSync("document.pdf", config)
	if err != nil {
		return err
	}

	fmt.Printf("Keywords: %v\n", result.Keywords)
	return nil
}

func main() {
	if err := basicYake(); err != nil {
		fmt.Println("Error:", err)
	}
}
