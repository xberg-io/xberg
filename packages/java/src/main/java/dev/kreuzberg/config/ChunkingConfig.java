package dev.kreuzberg.config;

import java.util.HashMap;
import java.util.Map;

/**
 * Chunking configuration for splitting extracted text.
 *
 * @since 4.0.0
 */
public final class ChunkingConfig {
	private final int maxChars;
	private final int maxOverlap;
	private final String preset;
	private final Map<String, Object> embedding;
	private final Boolean enabled;
	private final Map<String, Object> sizing;
	private final Boolean prependHeadingContext;
	private final String chunkerType;
	private final Double topicThreshold;

	private ChunkingConfig(Builder builder) {
		this.maxChars = builder.maxChars;
		this.maxOverlap = builder.maxOverlap;
		this.preset = builder.preset;
		this.embedding = builder.embedding;
		this.enabled = builder.enabled;
		this.sizing = builder.sizing;
		this.prependHeadingContext = builder.prependHeadingContext;
		this.chunkerType = builder.chunkerType;
		this.topicThreshold = builder.topicThreshold;
	}

	public static Builder builder() {
		return new Builder();
	}

	public int getMaxChars() {
		return maxChars;
	}

	public int getMaxOverlap() {
		return maxOverlap;
	}

	public String getPreset() {
		return preset;
	}

	public Map<String, Object> getEmbedding() {
		return embedding;
	}

	public Boolean getEnabled() {
		return enabled;
	}

	public Map<String, Object> getSizing() {
		return sizing;
	}

	public Boolean getPrependHeadingContext() {
		return prependHeadingContext;
	}

	/**
	 * Get the chunker type.
	 *
	 * <p>Set to {@code "semantic"} for topic-aware chunking that works out of
	 * the box with no extra configuration needed.
	 *
	 * @return the chunker type, or null if not set (defaults to "text").
	 *         Supported values: "text", "markdown", "yaml", "semantic".
	 * @since 4.5.4
	 */
	public String getChunkerType() {
		return chunkerType;
	}

	/**
	 * Get the cosine similarity threshold for semantic topic detection.
	 * Optional, defaults to 0.75. Rarely needs tuning.
	 *
	 * @return the topic threshold (0.0-1.0), or null if not set (defaults to 0.75)
	 */
	public Double getTopicThreshold() {
		return topicThreshold;
	}

	public Map<String, Object> toMap() {
		Map<String, Object> map = new HashMap<>();
		map.put("max_chars", maxChars);
		map.put("max_overlap", maxOverlap);
		if (preset != null) {
			map.put("preset", preset);
		}
		if (embedding != null) {
			map.put("embedding", embedding);
		}
		if (enabled != null) {
			map.put("enabled", enabled);
		}
		if (sizing != null) {
			map.put("sizing", sizing);
		}
		if (prependHeadingContext != null) {
			map.put("prepend_heading_context", prependHeadingContext);
		}
		if (chunkerType != null) {
			map.put("chunker_type", chunkerType);
		}
		if (topicThreshold != null) {
			map.put("topic_threshold", topicThreshold);
		}
		return map;
	}

	public static final class Builder {
		private int maxChars = 1000;
		private int maxOverlap = 200;
		private String preset;
		private Map<String, Object> embedding;
		private Boolean enabled;
		private Map<String, Object> sizing;
		private Boolean prependHeadingContext;
		private String chunkerType;
		private Double topicThreshold;

		private Builder() {
		}

		public Builder maxChars(int maxChars) {
			this.maxChars = maxChars;
			return this;
		}

		public Builder maxOverlap(int maxOverlap) {
			this.maxOverlap = maxOverlap;
			return this;
		}

		public Builder preset(String preset) {
			this.preset = preset;
			return this;
		}

		public Builder embedding(Map<String, Object> embedding) {
			this.embedding = embedding;
			return this;
		}

		public Builder enabled(Boolean enabled) {
			this.enabled = enabled;
			return this;
		}

		/**
		 * Set chunk sizing to token-based using a HuggingFace tokenizer model.
		 */
		public Builder sizingTokenizer(String model) {
			Map<String, Object> s = new HashMap<>();
			s.put("type", "tokenizer");
			s.put("model", model);
			this.sizing = s;
			return this;
		}

		/**
		 * Set chunk sizing to character-based (default).
		 */
		public Builder sizingCharacters() {
			Map<String, Object> s = new HashMap<>();
			s.put("type", "characters");
			this.sizing = s;
			return this;
		}

		/**
		 * Prepend heading context to each chunk for improved retrieval.
		 */
		public Builder prependHeadingContext(Boolean prependHeadingContext) {
			this.prependHeadingContext = prependHeadingContext;
			return this;
		}

		/**
		 * Set the chunker type.
		 *
		 * <p>Set to {@code "semantic"} for topic-aware chunking that works out of
		 * the box with sensible defaults (max_chars=1000, overlap=200,
		 * topic_threshold=0.75). No other parameters needed.
		 *
		 * @param chunkerType the chunker type ("text", "markdown", "yaml", or "semantic")
		 * @return this builder for chaining
		 * @since 4.5.4
		 */
		public Builder chunkerType(String chunkerType) {
			this.chunkerType = chunkerType;
			return this;
		}

		/**
		 * Set the cosine similarity threshold for semantic topic detection.
		 * Optional, defaults to 0.75. Rarely needs tuning.
		 *
		 * @param topicThreshold threshold value (0.0-1.0), optional, defaults to 0.75
		 * @return this builder for chaining
		 */
		public Builder topicThreshold(Double topicThreshold) {
			this.topicThreshold = topicThreshold;
			return this;
		}

		public ChunkingConfig build() {
			return new ChunkingConfig(this);
		}
	}

	static ChunkingConfig fromMap(Map<String, Object> map) {
		if (map == null) {
			return null;
		}
		Builder builder = builder();
		Object maxCharsValue = map.get("max_chars");
		if (maxCharsValue instanceof Number) {
			builder.maxChars(((Number) maxCharsValue).intValue());
		}
		Object maxOverlapValue = map.get("max_overlap");
		if (maxOverlapValue instanceof Number) {
			builder.maxOverlap(((Number) maxOverlapValue).intValue());
		}
		Object presetValue = map.get("preset");
		if (presetValue instanceof String) {
			builder.preset((String) presetValue);
		}
		@SuppressWarnings("unchecked")
		Map<String, Object> embeddingMap = map.get("embedding") instanceof Map
				? (Map<String, Object>) map.get("embedding")
				: null;
		if (embeddingMap != null && !embeddingMap.isEmpty()) {
			builder.embedding(new HashMap<>(embeddingMap));
		}
		if (map.containsKey("enabled")) {
			Object enabledValue = map.get("enabled");
			if (enabledValue instanceof Boolean) {
				builder.enabled((Boolean) enabledValue);
			}
		}
		@SuppressWarnings("unchecked")
		Map<String, Object> sizingMap = map.get("sizing") instanceof Map
				? (Map<String, Object>) map.get("sizing")
				: null;
		if (sizingMap != null && !sizingMap.isEmpty()) {
			builder.sizing = new HashMap<>(sizingMap);
		}
		if (map.containsKey("prepend_heading_context")) {
			Object prependHeadingContextValue = map.get("prepend_heading_context");
			if (prependHeadingContextValue instanceof Boolean) {
				builder.prependHeadingContext((Boolean) prependHeadingContextValue);
			}
		}
		Object chunkerTypeValue = map.get("chunker_type");
		if (chunkerTypeValue instanceof String) {
			builder.chunkerType((String) chunkerTypeValue);
		}
		Object topicThresholdValue = map.get("topic_threshold");
		if (topicThresholdValue instanceof Number) {
			builder.topicThreshold(((Number) topicThresholdValue).doubleValue());
		}
		return builder.build();
	}
}
