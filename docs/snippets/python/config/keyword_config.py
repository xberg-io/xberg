import asyncio
from xberg import ExtractInput, (
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
            language="en",
            yake_params=None,
            rake_params=None,
        )
    )

    output = await extract(ExtractInput.from_uri("document.pdf"), config)
    result = output.results[0]
    print(f"Keywords: {result.extracted_keywords}")


# Example 2: Advanced YAKE with custom parameters
# Fine-tunes YAKE with custom window size for co-occurrence analysis
async def advanced_yake() -> None:
    config = ExtractionConfig(
        keywords=KeywordConfig(
            algorithm=KeywordAlgorithm.YAKE,
            max_keywords=15,
            min_score=0.1,
            language="en",
            yake_params=YakeParams(
                window_size=1,
            ),
            rake_params=None,
        )
    )

    output = await extract(ExtractInput.from_uri("document.pdf"), config)
    result = output.results[0]
    print(f"Keywords: {result.extracted_keywords}")


# Example 3: RAKE configuration
# Uses RAKE algorithm for rapid keyword extraction with phrase constraints
async def rake_config() -> None:
    config = ExtractionConfig(
        keywords=KeywordConfig(
            algorithm=KeywordAlgorithm.RAKE,
            max_keywords=10,
            min_score=5.0,
            language="en",
            yake_params=None,
            rake_params=RakeParams(
                min_word_length=1,
                max_words_per_phrase=3,
            ),
        )
    )

    output = await extract(ExtractInput.from_uri("document.pdf"), config)
    result = output.results[0]
    print(f"Keywords: {result.extracted_keywords}")


if __name__ == "__main__":
    asyncio.run(basic_yake())
