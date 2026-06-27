import asyncio
from xberg import (
    ExtractionConfig,
    KeywordConfig,
    KeywordAlgorithm,
    YakeParams,
    RakeParams,
    extract,
)


# Example 1: Basic YAKE configuration
# Uses YAKE algorithm with default parameters and English stopword filtering
async def basic_yake() -> None:
    config = ExtractionConfig(
        keywords=KeywordConfig(
            algorithm=KeywordAlgorithm.YAKE,
            max_keywords=10,
            min_score=0.0,
            ngram_range=(1, 3),
            language="en",
            yake_params=None,
            rake_params=None,
        )
    )

    result = await extract("document.pdf", config=config)
    print(f"Keywords: {result.keywords}")


# Example 2: Advanced YAKE with custom parameters
# Fine-tunes YAKE with custom window size for co-occurrence analysis
async def advanced_yake() -> None:
    config = ExtractionConfig(
        keywords=KeywordConfig(
            algorithm=KeywordAlgorithm.YAKE,
            max_keywords=15,
            min_score=0.1,
            ngram_range=(1, 2),
            language="en",
            yake_params=YakeParams(
                window_size=1,
            ),
            rake_params=None,
        )
    )

    result = await extract("document.pdf", config=config)
    print(f"Keywords: {result.keywords}")


# Example 3: RAKE configuration
# Uses RAKE algorithm for rapid keyword extraction with phrase constraints
async def rake_config() -> None:
    config = ExtractionConfig(
        keywords=KeywordConfig(
            algorithm=KeywordAlgorithm.RAKE,
            max_keywords=10,
            min_score=5.0,
            ngram_range=(1, 3),
            language="en",
            yake_params=None,
            rake_params=RakeParams(
                min_word_length=1,
                max_words_per_phrase=3,
            ),
        )
    )

    result = await extract("document.pdf", config=config)
    print(f"Keywords: {result.keywords}")


if __name__ == "__main__":
    asyncio.run(basic_yake())
