#nullable enable

using System;
using System.Collections.Generic;
using System.Text.Json;
using System.Text.Json.Serialization;

namespace Xberg;

/// <summary>
/// Reads a <c>byte[]</c> from either a JSON array of numbers (as fixtures emit) or a
/// base64 string (System.Text.Json's default). Writes as a number array so the FFI
/// boundary receives the same shape the Rust core expects.
/// </summary>
internal sealed class ByteArrayJsonConverter : JsonConverter<byte[]?>
{
    public override byte[]? Read(ref Utf8JsonReader reader, Type typeToConvert, JsonSerializerOptions options)
    {
        if (reader.TokenType == JsonTokenType.Null)
        {
            return null;
        }

        if (reader.TokenType == JsonTokenType.String)
        {
            return reader.GetBytesFromBase64();
        }

        if (reader.TokenType == JsonTokenType.StartArray)
        {
            var bytes = new List<byte>();
            while (reader.Read() && reader.TokenType != JsonTokenType.EndArray)
            {
                bytes.Add(reader.GetByte());
            }
            return bytes.ToArray();
        }

        throw new JsonException($"Unexpected token {reader.TokenType} when reading byte[]");
    }

    public override void Write(Utf8JsonWriter writer, byte[]? value, JsonSerializerOptions options)
    {
        if (value is null)
        {
            writer.WriteNullValue();
            return;
        }

        writer.WriteStartArray();
        foreach (var b in value)
        {
            writer.WriteNumberValue(b);
        }
        writer.WriteEndArray();
    }
}

/// <summary>
/// Utility for lenient JSON deserialization that ignores unknown properties.
/// </summary>
internal static class JsonLeniency
{
    /// <summary>
    /// Remove unknown properties from a JSON object before deserialization.
    /// This allows JSON with extra fields to be successfully parsed even if the target
    /// type doesn't have those fields.
    /// </summary>
    /// <param name="json">The JSON string to filter</param>
    /// <param name="knownProperties">Set of property names that are known/allowed</param>
    /// <returns>A JSON string with unknown properties removed</returns>
    public static string FilterUnknownProperties(string json, HashSet<string> knownProperties)
    {
        if (string.IsNullOrEmpty(json) || json.Trim() == "{}" || json.Trim() == "[]" || json.Trim() == "null")
        {
            return json;
        }

        try
        {
            using var document = JsonDocument.Parse(json);
            if (document.RootElement.ValueKind != JsonValueKind.Object)
            {
                return json;
            }

            var options = new JsonSerializerOptions { WriteIndented = false };
            using var stream = new System.IO.MemoryStream();
            using var writer = new Utf8JsonWriter(stream);

            writer.WriteStartObject();
            foreach (var property in document.RootElement.EnumerateObject())
            {
                // Only write properties that are known
                if (knownProperties.Contains(property.Name))
                {
                    writer.WritePropertyName(property.Name);
                    property.Value.WriteTo(writer);
                }
            }
            writer.WriteEndObject();
            writer.Flush();

            return System.Text.Encoding.UTF8.GetString(stream.ToArray());
        }
        catch
        {
            // If filtering fails, return the original JSON and let deserialization handle it
            return json;
        }
    }
}
