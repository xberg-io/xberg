package dev.kreuzberg;

/**
 * Exception thrown when text embedding generation fails.
 *
 * <p>
 * Embedding errors occur during vector generation, such as:
 *
 * <ul>
 * <li>Model loading failures
 * <li>Inference engine errors
 * <li>Invalid input text for the selected model
 * <li>Resource exhaustion during batch processing
 * </ul>
 *
 * @since 4.6.0
 */
public final class EmbeddingException extends KreuzbergException {
	private static final long serialVersionUID = 1L;

	/**
	 * Constructs a new Embedding exception with the specified message.
	 *
	 * @param message
	 *            the detail message explaining why embedding generation failed
	 */
	public EmbeddingException(String message) {
		super(message, ErrorCode.EMBEDDING);
	}

	/**
	 * Constructs a new Embedding exception with the specified message and cause.
	 *
	 * @param message
	 *            the detail message
	 * @param cause
	 *            the cause of the embedding failure
	 */
	public EmbeddingException(String message, Throwable cause) {
		super(message, ErrorCode.EMBEDDING, null, cause);
	}
}
