package main

import (
	"fmt"

	"github.com/xberg-io/xberg"
)

// Example 1: Basic YAKE configuration
// Uses YAKE algorithm with default parameters and English stopword filtering
func basicYake() error {
	maxKeywords := uint(10)
	language := "en"

	config := xberg.ExtractionConfig{
		Keywords: &xberg.KeywordConfig{
			Algorithm:   xberg.KeywordAlgorithmYake,
			MaxKeywords: &maxKeywords,
			MinScore:    0.0,
			Language:    &language,
			YakeParams:  nil,
			RakeParams:  nil,
		},
	}

	return printKeywords("document.pdf", config)
}

// Example 2: Advanced YAKE with custom parameters
// Fine-tunes YAKE with custom window size for co-occurrence analysis
func advancedYake() error {
	maxKeywords := uint(15)
	windowSize := uint(1)
	language := "en"

	config := xberg.ExtractionConfig{
		Keywords: &xberg.KeywordConfig{
			Algorithm:   xberg.KeywordAlgorithmYake,
			MaxKeywords: &maxKeywords,
			MinScore:    0.1,
			Language:    &language,
			YakeParams: &xberg.YakeParams{
				WindowSize: &windowSize,
			},
			RakeParams: nil,
		},
	}

	return printKeywords("document.pdf", config)
}

// Example 3: RAKE configuration
// Uses RAKE algorithm for rapid keyword extraction with phrase constraints
func rakeConfig() error {
	maxKeywords := uint(10)
	minWordLength := uint(1)
	maxWordsPerPhrase := uint(3)
	language := "en"

	config := xberg.ExtractionConfig{
		Keywords: &xberg.KeywordConfig{
			Algorithm:   xberg.KeywordAlgorithmRake,
			MaxKeywords: &maxKeywords,
			MinScore:    5.0,
			Language:    &language,
			YakeParams:  nil,
			RakeParams: &xberg.RakeParams{
				MinWordLength:     &minWordLength,
				MaxWordsPerPhrase: &maxWordsPerPhrase,
			},
		},
	}

	return printKeywords("document.pdf", config)
}

func printKeywords(uri string, config xberg.ExtractionConfig) error {
	kind := xberg.ExtractInputKindURI
	output, err := xberg.Extract(xberg.ExtractInput{Kind: &kind, URI: &uri}, config)
	if err != nil {
		return fmt.Errorf("extracting keywords from %s: %w", uri, err)
	}
	if len(output.Results) == 0 {
		return fmt.Errorf("no extraction results for %s", uri)
	}

	fmt.Printf("Keywords: %v\n", output.Results[0].ExtractedKeywords)
	return nil
}

func main() {
	if err := basicYake(); err != nil {
		fmt.Println("Error:", err)
	}
}
