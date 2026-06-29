package io.xberg;

import com.fasterxml.jackson.core.JsonGenerator;
import com.fasterxml.jackson.databind.JsonSerializer;
import com.fasterxml.jackson.databind.SerializerProvider;
import java.io.IOException;

/**
 * Custom serializer for byte[] that outputs a JSON array of integers.
 * Rust's serde expects bytes to be a JSON array like [1,2,3,...] not a Base64 string.
 */
public class ByteArraySerializer extends JsonSerializer<byte[]> {
    @Override
    public void serialize(final byte[] value, final JsonGenerator gen, final SerializerProvider provider)
            throws IOException {
        gen.writeStartArray();
        for (final byte b : value) {
            // Output as unsigned int (0-255)
            gen.writeNumber(b & 0xFF);
        }
        gen.writeEndArray();
    }
}
