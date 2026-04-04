using System.Text.RegularExpressions;

namespace Kreuzberg;

/// <summary>
/// Enumeration of error types that can occur during document extraction and processing.
/// </summary>
public enum KreuzbergErrorKind
{
    /// <summary>Unknown or unclassified error.</summary>
    Unknown,
    /// <summary>Input/Output error (file access, network, etc.).</summary>
    Io,
    /// <summary>Validation error (invalid input, configuration, etc.).</summary>
    Validation,
    /// <summary>Error parsing document format.</summary>
    Parsing,
    /// <summary>Error during optical character recognition (OCR) processing.</summary>
    Ocr,
    /// <summary>Error related to caching operations.</summary>
    Cache,
    /// <summary>Error during image processing or manipulation.</summary>
    ImageProcessing,
    /// <summary>Error during JSON serialization or deserialization.</summary>
    Serialization,
    /// <summary>Required external dependency is missing or unavailable.</summary>
    MissingDependency,
    /// <summary>Error in custom plugin execution or registration.</summary>
    Plugin,
    /// <summary>Document format is not supported.</summary>
    UnsupportedFormat,
    /// <summary>Runtime error (lock poisoning, unsupported operation, etc.).</summary>
    Runtime,
    /// <summary>Error during text embedding generation.</summary>
    Embedding,
}

/// <summary>
/// Interface for Kreuzberg exceptions that provides access to the error kind.
/// </summary>
public interface IKreuzbergError
{
    /// <summary>Gets the kind/category of this error.</summary>
    KreuzbergErrorKind Kind { get; }
}

/// <summary>
/// Base exception class for all Kreuzberg-specific errors.
/// </summary>
public class KreuzbergException : Exception, IKreuzbergError
{
    /// <summary>Gets the kind/category of this error.</summary>
    public KreuzbergErrorKind Kind { get; }

    /// <summary>
    /// Initializes a new instance of the KreuzbergException class.
    /// </summary>
    /// <param name="kind">The error kind/category.</param>
    /// <param name="message">The error message. If null or whitespace, defaults to "kreuzberg: unknown error".</param>
    /// <param name="inner">The inner exception that caused this error, if any.</param>
    public KreuzbergException(KreuzbergErrorKind kind, string message, Exception? inner = null)
        : base(string.IsNullOrWhiteSpace(message) ? "kreuzberg: unknown error" : message, inner)
    {
        Kind = kind;
    }
}

/// <summary>
/// Exception thrown when document validation fails due to invalid input or configuration.
/// </summary>
public class KreuzbergValidationException : KreuzbergException
{
    /// <summary>
    /// Initializes a new instance of the KreuzbergValidationException class.
    /// </summary>
    /// <param name="message">The validation error message.</param>
    /// <param name="inner">The inner exception that caused this validation error, if any.</param>
    public KreuzbergValidationException(string message, Exception? inner = null)
        : base(KreuzbergErrorKind.Validation, ErrorMapper.PrefixMessage(message, "Validation error"), inner)
    {
    }
}

/// <summary>
/// Exception thrown when an error occurs while parsing a document format.
/// </summary>
public class KreuzbergParsingException : KreuzbergException
{
    /// <summary>
    /// Initializes a new instance of the KreuzbergParsingException class.
    /// </summary>
    /// <param name="message">The parsing error message.</param>
    /// <param name="inner">The inner exception that caused this parsing error, if any.</param>
    public KreuzbergParsingException(string message, Exception? inner = null)
        : base(KreuzbergErrorKind.Parsing, ErrorMapper.PrefixMessage(message, "Parsing error"), inner)
    {
    }
}

/// <summary>
/// Exception thrown when an error occurs during optical character recognition (OCR) processing.
/// </summary>
public class KreuzbergOcrException : KreuzbergException
{
    /// <summary>
    /// Initializes a new instance of the KreuzbergOcrException class.
    /// </summary>
    /// <param name="message">The OCR error message.</param>
    /// <param name="inner">The inner exception that caused this OCR error, if any.</param>
    public KreuzbergOcrException(string message, Exception? inner = null)
        : base(KreuzbergErrorKind.Ocr, ErrorMapper.PrefixMessage(message, "OCR error"), inner)
    {
    }
}

/// <summary>
/// Exception thrown when a caching operation fails.
/// </summary>
public class KreuzbergCacheException : KreuzbergException
{
    /// <summary>
    /// Initializes a new instance of the KreuzbergCacheException class.
    /// </summary>
    /// <param name="message">The cache error message.</param>
    /// <param name="inner">The inner exception that caused this cache error, if any.</param>
    public KreuzbergCacheException(string message, Exception? inner = null)
        : base(KreuzbergErrorKind.Cache, ErrorMapper.PrefixMessage(message, "Cache error"), inner)
    {
    }
}

/// <summary>
/// Exception thrown when an error occurs during image processing or manipulation.
/// </summary>
public class KreuzbergImageProcessingException : KreuzbergException
{
    /// <summary>
    /// Initializes a new instance of the KreuzbergImageProcessingException class.
    /// </summary>
    /// <param name="message">The image processing error message.</param>
    /// <param name="inner">The inner exception that caused this image processing error, if any.</param>
    public KreuzbergImageProcessingException(string message, Exception? inner = null)
        : base(KreuzbergErrorKind.ImageProcessing, ErrorMapper.PrefixMessage(message, "Image processing error"), inner)
    {
    }
}

/// <summary>
/// Exception thrown when JSON serialization or deserialization fails.
/// </summary>
public class KreuzbergSerializationException : KreuzbergException
{
    /// <summary>
    /// Initializes a new instance of the KreuzbergSerializationException class.
    /// </summary>
    /// <param name="message">The serialization error message.</param>
    /// <param name="inner">The inner exception that caused this serialization error, if any.</param>
    public KreuzbergSerializationException(string message, Exception? inner = null)
        : base(KreuzbergErrorKind.Serialization, ErrorMapper.PrefixMessage(message, "Serialization error"), inner)
    {
    }
}

/// <summary>
/// Exception thrown when a required external dependency is missing or unavailable.
/// </summary>
public class KreuzbergMissingDependencyException : KreuzbergException
{
    /// <summary>
    /// Gets the name of the missing dependency.
    /// </summary>
    public string Dependency { get; }

    /// <summary>
    /// Initializes a new instance of the KreuzbergMissingDependencyException class.
    /// </summary>
    /// <param name="dependency">The name of the missing dependency (e.g., "tesseract", "opencv").</param>
    /// <param name="message">The error message describing the missing dependency.</param>
    /// <param name="inner">The inner exception that caused this error, if any.</param>
    public KreuzbergMissingDependencyException(string dependency, string message, Exception? inner = null)
        : base(KreuzbergErrorKind.MissingDependency, ErrorMapper.PrefixMessage(message, $"Missing dependency: {dependency}"), inner)
    {
        Dependency = dependency;
    }
}

/// <summary>
/// Exception thrown when a custom plugin fails to execute or register.
/// </summary>
public class KreuzbergPluginException : KreuzbergException
{
    /// <summary>
    /// Gets the name of the plugin that failed.
    /// </summary>
    public string PluginName { get; }

    /// <summary>
    /// Initializes a new instance of the KreuzbergPluginException class.
    /// </summary>
    /// <param name="pluginName">The name of the plugin that failed.</param>
    /// <param name="message">The error message describing the plugin failure.</param>
    /// <param name="inner">The inner exception that caused this plugin error, if any.</param>
    public KreuzbergPluginException(string pluginName, string message, Exception? inner = null)
        : base(KreuzbergErrorKind.Plugin, ErrorMapper.PrefixMessage(message, "Plugin error"), inner)
    {
        PluginName = pluginName;
    }
}

/// <summary>
/// Exception thrown when a document format is not supported.
/// </summary>
public class KreuzbergUnsupportedFormatException : KreuzbergException
{
    /// <summary>
    /// Gets the format that is not supported.
    /// </summary>
    public string Format { get; }

    /// <summary>
    /// Initializes a new instance of the KreuzbergUnsupportedFormatException class.
    /// </summary>
    /// <param name="format">The unsupported document format (e.g., "application/x-custom").</param>
    /// <param name="message">The error message describing why the format is unsupported.</param>
    /// <param name="inner">The inner exception that caused this error, if any.</param>
    public KreuzbergUnsupportedFormatException(string format, string message, Exception? inner = null)
        : base(KreuzbergErrorKind.UnsupportedFormat, ErrorMapper.PrefixMessage(message, $"Unsupported format: {format}"), inner)
    {
        Format = format;
    }
}

/// <summary>
/// Exception thrown when an input/output error occurs (file access, network, etc.).
/// Inherits from IOException for compatibility with standard .NET I/O exception handling.
/// </summary>
public class KreuzbergIOException : IOException, IKreuzbergError
{
    /// <summary>Gets the error kind for I/O errors.</summary>
    public KreuzbergErrorKind Kind => KreuzbergErrorKind.Io;

    /// <summary>
    /// Initializes a new instance of the KreuzbergIOException class.
    /// </summary>
    /// <param name="message">The I/O error message.</param>
    /// <param name="inner">The inner exception that caused this I/O error, if any.</param>
    public KreuzbergIOException(string message, Exception? inner = null)
        : base(ErrorMapper.PrefixMessage(message, "IO error"), inner)
    {
    }
}

/// <summary>
/// Exception thrown for runtime errors such as lock poisoning or unsupported operations.
/// </summary>
public class KreuzbergRuntimeException : Exception, IKreuzbergError
{
    /// <summary>Gets the error kind for runtime errors.</summary>
    public KreuzbergErrorKind Kind => KreuzbergErrorKind.Runtime;

    /// <summary>
    /// Initializes a new instance of the KreuzbergRuntimeException class.
    /// </summary>
    /// <param name="message">The runtime error message.</param>
    /// <param name="inner">The inner exception that caused this runtime error, if any.</param>
    public KreuzbergRuntimeException(string message, Exception? inner = null)
        : base(ErrorMapper.PrefixMessage(message, "Runtime error"), inner)
    {
    }
}

/// <summary>
/// Exception thrown when text embedding generation fails.
/// </summary>
public class KreuzbergEmbeddingException : KreuzbergException
{
    /// <summary>
    /// Initializes a new instance of the KreuzbergEmbeddingException class.
    /// </summary>
    /// <param name="message">The embedding error message.</param>
    /// <param name="inner">The inner exception that caused this error, if any.</param>
    public KreuzbergEmbeddingException(string message, Exception? inner = null)
        : base(KreuzbergErrorKind.Embedding, ErrorMapper.PrefixMessage(message, "Embedding error"), inner)
    {
    }
}

/// <summary>
/// Internal utility class for mapping native Kreuzberg errors to .NET exceptions.
/// This class parses error messages from the Rust FFI layer and creates appropriate exception types.
/// </summary>
internal static class ErrorMapper
{
    /// <summary>
    /// Creates an exception for an unknown/unclassified error.
    /// </summary>
    /// <param name="message">The error message.</param>
    /// <returns>A KreuzbergRuntimeException with the provided message.</returns>
    internal static Exception Unknown(string message)
    {
        return new KreuzbergRuntimeException(message);
    }

    /// <summary>
    /// Parses a native error message and returns the appropriate exception type.
    /// Examines the error message prefix to determine the error category.
    /// </summary>
    /// <param name="error">The error message from the native Kreuzberg library.</param>
    /// <returns>An exception of the appropriate type based on the error message prefix.</returns>
    internal static Exception FromNativeError(string? error)
    {
        var trimmed = string.IsNullOrWhiteSpace(error) ? "Unknown error" : error.Trim();

        if (trimmed.StartsWith("Validation error:", StringComparison.OrdinalIgnoreCase))
        {
            return new KreuzbergValidationException(trimmed);
        }

        if (trimmed.StartsWith("Parsing error:", StringComparison.OrdinalIgnoreCase))
        {
            return new KreuzbergParsingException(trimmed);
        }

        if (trimmed.StartsWith("OCR error:", StringComparison.OrdinalIgnoreCase))
        {
            return new KreuzbergOcrException(trimmed);
        }

        if (trimmed.StartsWith("Cache error:", StringComparison.OrdinalIgnoreCase))
        {
            return new KreuzbergCacheException(trimmed);
        }

        if (trimmed.StartsWith("Image processing error:", StringComparison.OrdinalIgnoreCase))
        {
            return new KreuzbergImageProcessingException(trimmed);
        }

        if (trimmed.StartsWith("Serialization error:", StringComparison.OrdinalIgnoreCase))
        {
            return new KreuzbergSerializationException(trimmed);
        }

        if (trimmed.StartsWith("Missing dependency:", StringComparison.OrdinalIgnoreCase))
        {
            var dependency = trimmed["Missing dependency:".Length..].Trim();
            return new KreuzbergMissingDependencyException(dependency, trimmed);
        }

        if (trimmed.StartsWith("Plugin error", StringComparison.OrdinalIgnoreCase))
        {
            var name = ParsePluginName(trimmed);
            return new KreuzbergPluginException(name, trimmed);
        }

        if (trimmed.StartsWith("Unsupported format:", StringComparison.OrdinalIgnoreCase))
        {
            var format = trimmed["Unsupported format:".Length..].Trim();
            return new KreuzbergUnsupportedFormatException(format, trimmed);
        }

        if (trimmed.StartsWith("IO error:", StringComparison.OrdinalIgnoreCase))
        {
            return new KreuzbergIOException(trimmed);
        }

        if (trimmed.StartsWith("Lock poisoned:", StringComparison.OrdinalIgnoreCase) ||
            trimmed.StartsWith("Unsupported operation:", StringComparison.OrdinalIgnoreCase))
        {
            return new KreuzbergRuntimeException(trimmed);
        }

        if (trimmed.StartsWith("Embedding error:", StringComparison.OrdinalIgnoreCase))
        {
            return new KreuzbergEmbeddingException(trimmed);
        }

        return new KreuzbergRuntimeException(trimmed);
    }

    /// <summary>
    /// Extracts the plugin name from an error message using regex pattern matching.
    /// </summary>
    /// <param name="message">The full error message containing plugin name.</param>
    /// <returns>The extracted plugin name, or "unknown" if parsing fails.</returns>
    private static string ParsePluginName(string message)
    {
        var match = Regex.Match(message, @"Plugin error in ([^:]+)");
        if (match.Success)
        {
            return match.Groups[1].Value;
        }
        return "unknown";
    }

    /// <summary>
    /// Prefixes an error message with a category prefix, handling edge cases.
    /// If the message already has the prefix or starts with "kreuzberg:", returns as-is.
    /// Otherwise, combines the prefix with the message.
    /// </summary>
    /// <param name="message">The original error message.</param>
    /// <param name="prefix">The prefix to add (e.g., "Validation error", "Parsing error").</param>
    /// <returns>The prefixed error message in the format "kreuzberg: {prefix}: {message}".</returns>
    internal static string PrefixMessage(string? message, string prefix)
    {
        var trimmed = string.IsNullOrWhiteSpace(message) ? string.Empty : message.Trim();
        if (trimmed.StartsWith(prefix, StringComparison.OrdinalIgnoreCase) || trimmed.StartsWith("kreuzberg:", StringComparison.OrdinalIgnoreCase))
        {
            return trimmed;
        }

        var baseMessage = string.IsNullOrWhiteSpace(trimmed) ? $"{prefix}: unknown error" : $"{prefix}: {trimmed}";
        return $"kreuzberg: {baseMessage}";
    }
}
