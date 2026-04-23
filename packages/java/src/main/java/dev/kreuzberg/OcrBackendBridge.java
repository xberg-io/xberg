package dev.kreuzberg;

import java.lang.foreign.Arena;
import java.lang.foreign.FunctionDescriptor;
import java.lang.foreign.Linker;
import java.lang.foreign.MemorySegment;
import java.lang.foreign.ValueLayout;
import java.lang.invoke.MethodHandles;
import java.lang.invoke.MethodType;
import java.util.List;
import java.util.Map;
import java.util.concurrent.ConcurrentHashMap;
import com.fasterxml.jackson.databind.ObjectMapper;

/**
 * Allocates Panama FFM upcall stubs for an IOcrBackend implementation,
 * assembles the C vtable in native memory, and provides static
 * registerOcrBackend/unregisterOcrBackend helpers.
 */
public final class OcrBackendBridge implements AutoCloseable {

    private static final Linker LINKER = Linker.nativeLinker();
    private static final MethodHandles.Lookup LOOKUP = MethodHandles.lookup();
    private static final ObjectMapper JSON = new ObjectMapper();

    /** Live registry — keeps Arenas and upcall stubs alive past the register call. */
    private static final ConcurrentHashMap<String, OcrBackendBridge> OCR_BACKEND_BRIDGES = new ConcurrentHashMap<>();

    // C vtable: 13 fields (4 plugin methods + 8 trait methods + free_user_data)
    private static final long VTABLE_SIZE = (long) ValueLayout.ADDRESS.byteSize() * 13L;

    private final Arena arena;
    private final MemorySegment vtable;
    private final IOcrBackend impl;

    OcrBackendBridge(final IOcrBackend impl) {
        this.impl = impl;
        this.arena = Arena.ofShared();
        this.vtable = arena.allocate(VTABLE_SIZE);

        try {
            long offset = 0L;

            var stubName = LINKER.upcallStub(LOOKUP.bind(this, "handleName",
                MethodType.methodType(MemorySegment.class, MemorySegment.class)),
                FunctionDescriptor.of(ValueLayout.ADDRESS, ValueLayout.ADDRESS),
                arena);
            vtable.set(ValueLayout.ADDRESS, offset, stubName);
            offset += ValueLayout.ADDRESS.byteSize();

            var stubVersion = LINKER.upcallStub(LOOKUP.bind(this, "handleVersion",
                MethodType.methodType(MemorySegment.class, MemorySegment.class)),
                FunctionDescriptor.of(ValueLayout.ADDRESS, ValueLayout.ADDRESS),
                arena);
            vtable.set(ValueLayout.ADDRESS, offset, stubVersion);
            offset += ValueLayout.ADDRESS.byteSize();

            var stubInitialize = LINKER.upcallStub(LOOKUP.bind(this, "handleInitialize",
                MethodType.methodType(int.class, MemorySegment.class, MemorySegment.class)),
                FunctionDescriptor.of(ValueLayout.JAVA_INT, ValueLayout.ADDRESS, ValueLayout.ADDRESS),
                arena);
            vtable.set(ValueLayout.ADDRESS, offset, stubInitialize);
            offset += ValueLayout.ADDRESS.byteSize();

            var stubShutdown = LINKER.upcallStub(LOOKUP.bind(this, "handleShutdown",
                MethodType.methodType(int.class, MemorySegment.class, MemorySegment.class)),
                FunctionDescriptor.of(ValueLayout.JAVA_INT, ValueLayout.ADDRESS, ValueLayout.ADDRESS),
                arena);
            vtable.set(ValueLayout.ADDRESS, offset, stubShutdown);
            offset += ValueLayout.ADDRESS.byteSize();

            var stubProcessImage = LINKER.upcallStub(LOOKUP.bind(this, "handleProcessImage",
                MethodType.methodType(int.class, MemorySegment.class, MemorySegment.class, MemorySegment.class, MemorySegment.class, MemorySegment.class)),
                FunctionDescriptor.of(ValueLayout.JAVA_INT, ValueLayout.ADDRESS, ValueLayout.ADDRESS, ValueLayout.ADDRESS, ValueLayout.ADDRESS, ValueLayout.ADDRESS),
                arena);
            vtable.set(ValueLayout.ADDRESS, offset, stubProcessImage);
            offset += ValueLayout.ADDRESS.byteSize();

            var stubProcessImageFile = LINKER.upcallStub(LOOKUP.bind(this, "handleProcessImageFile",
                MethodType.methodType(int.class, MemorySegment.class, MemorySegment.class, MemorySegment.class, MemorySegment.class, MemorySegment.class)),
                FunctionDescriptor.of(ValueLayout.JAVA_INT, ValueLayout.ADDRESS, ValueLayout.ADDRESS, ValueLayout.ADDRESS, ValueLayout.ADDRESS, ValueLayout.ADDRESS),
                arena);
            vtable.set(ValueLayout.ADDRESS, offset, stubProcessImageFile);
            offset += ValueLayout.ADDRESS.byteSize();

            var stubSupportsLanguage = LINKER.upcallStub(LOOKUP.bind(this, "handleSupportsLanguage",
                MethodType.methodType(int.class, MemorySegment.class, MemorySegment.class, MemorySegment.class, MemorySegment.class)),
                FunctionDescriptor.of(ValueLayout.JAVA_INT, ValueLayout.ADDRESS, ValueLayout.ADDRESS, ValueLayout.ADDRESS, ValueLayout.ADDRESS),
                arena);
            vtable.set(ValueLayout.ADDRESS, offset, stubSupportsLanguage);
            offset += ValueLayout.ADDRESS.byteSize();

            var stubBackendType = LINKER.upcallStub(LOOKUP.bind(this, "handleBackendType",
                MethodType.methodType(int.class, MemorySegment.class, MemorySegment.class, MemorySegment.class)),
                FunctionDescriptor.of(ValueLayout.JAVA_INT, ValueLayout.ADDRESS, ValueLayout.ADDRESS, ValueLayout.ADDRESS),
                arena);
            vtable.set(ValueLayout.ADDRESS, offset, stubBackendType);
            offset += ValueLayout.ADDRESS.byteSize();

            var stubSupportedLanguages = LINKER.upcallStub(LOOKUP.bind(this, "handleSupportedLanguages",
                MethodType.methodType(int.class, MemorySegment.class, MemorySegment.class, MemorySegment.class)),
                FunctionDescriptor.of(ValueLayout.JAVA_INT, ValueLayout.ADDRESS, ValueLayout.ADDRESS, ValueLayout.ADDRESS),
                arena);
            vtable.set(ValueLayout.ADDRESS, offset, stubSupportedLanguages);
            offset += ValueLayout.ADDRESS.byteSize();

            var stubSupportsTableDetection = LINKER.upcallStub(LOOKUP.bind(this, "handleSupportsTableDetection",
                MethodType.methodType(int.class, MemorySegment.class, MemorySegment.class, MemorySegment.class)),
                FunctionDescriptor.of(ValueLayout.JAVA_INT, ValueLayout.ADDRESS, ValueLayout.ADDRESS, ValueLayout.ADDRESS),
                arena);
            vtable.set(ValueLayout.ADDRESS, offset, stubSupportsTableDetection);
            offset += ValueLayout.ADDRESS.byteSize();

            var stubSupportsDocumentProcessing = LINKER.upcallStub(LOOKUP.bind(this, "handleSupportsDocumentProcessing",
                MethodType.methodType(int.class, MemorySegment.class, MemorySegment.class, MemorySegment.class)),
                FunctionDescriptor.of(ValueLayout.JAVA_INT, ValueLayout.ADDRESS, ValueLayout.ADDRESS, ValueLayout.ADDRESS),
                arena);
            vtable.set(ValueLayout.ADDRESS, offset, stubSupportsDocumentProcessing);
            offset += ValueLayout.ADDRESS.byteSize();

            var stubProcessDocument = LINKER.upcallStub(LOOKUP.bind(this, "handleProcessDocument",
                MethodType.methodType(int.class, MemorySegment.class, MemorySegment.class, MemorySegment.class, MemorySegment.class, MemorySegment.class)),
                FunctionDescriptor.of(ValueLayout.JAVA_INT, ValueLayout.ADDRESS, ValueLayout.ADDRESS, ValueLayout.ADDRESS, ValueLayout.ADDRESS, ValueLayout.ADDRESS),
                arena);
            vtable.set(ValueLayout.ADDRESS, offset, stubProcessDocument);
            offset += ValueLayout.ADDRESS.byteSize();

            vtable.set(ValueLayout.ADDRESS, offset, MemorySegment.NULL);

        } catch (ReflectiveOperationException e) {
            arena.close();
            throw new RuntimeException("Failed to create trait bridge stubs", e);
        }
    }

    MemorySegment vtableSegment() { return vtable; }

    private MemorySegment handleName(MemorySegment userData) {
        try {
            return arena.allocateFrom(impl.name());
        } catch (Throwable e) { return MemorySegment.NULL; }
    }

    private MemorySegment handleVersion(MemorySegment userData) {
        try {
            return arena.allocateFrom(impl.version());
        } catch (Throwable e) { return MemorySegment.NULL; }
    }

    private int handleInitialize(MemorySegment userData, MemorySegment outError) {
        try { impl.initialize(); return 0; }
        catch (Throwable e) { writeError(outError, e); return 1; }
    }

    private int handleShutdown(MemorySegment userData, MemorySegment outError) {
        try { impl.shutdown(); return 0; }
        catch (Throwable e) { writeError(outError, e); return 1; }
    }

    private int handleProcessImage(MemorySegment userData, MemorySegment image_bytes_in, MemorySegment config_in, MemorySegment outResult, MemorySegment outError) {
        try {
            byte[] image_bytes = image_bytes_in.reinterpret(Long.MAX_VALUE).toArray(ValueLayout.JAVA_BYTE);
            String config_json = config_in.reinterpret(Long.MAX_VALUE).getString(0);
            OcrConfig config = JSON.readValue(config_json, OcrConfig.class);
            ExtractionResult result = impl.process_image(image_bytes, config);
            String json = JSON.writeValueAsString(result);
            MemorySegment jsonCs = arena.allocateFrom(json);
            outResult.set(ValueLayout.ADDRESS, 0, jsonCs);
            return 0;
        } catch (Throwable e) {
            writeError(outError, e);
            return 1;
        }
    }

    private int handleProcessImageFile(MemorySegment userData, MemorySegment path_in, MemorySegment config_in, MemorySegment outResult, MemorySegment outError) {
        try {
            java.nio.file.Path path = java.nio.file.Paths.get(path_in.reinterpret(Long.MAX_VALUE).getString(0));
            String config_json = config_in.reinterpret(Long.MAX_VALUE).getString(0);
            OcrConfig config = JSON.readValue(config_json, OcrConfig.class);
            ExtractionResult result = impl.process_image_file(path, config);
            String json = JSON.writeValueAsString(result);
            MemorySegment jsonCs = arena.allocateFrom(json);
            outResult.set(ValueLayout.ADDRESS, 0, jsonCs);
            return 0;
        } catch (Throwable e) {
            writeError(outError, e);
            return 1;
        }
    }

    private int handleSupportsLanguage(MemorySegment userData, MemorySegment lang_in, MemorySegment outResult, MemorySegment outError) {
        try {
            String lang = lang_in.reinterpret(Long.MAX_VALUE).getString(0);
            boolean result = impl.supports_language(lang);
            String json = JSON.writeValueAsString(result);
            MemorySegment jsonCs = arena.allocateFrom(json);
            outResult.set(ValueLayout.ADDRESS, 0, jsonCs);
            return 0;
        } catch (Throwable e) {
            writeError(outError, e);
            return 1;
        }
    }

    private int handleBackendType(MemorySegment userData, MemorySegment outResult, MemorySegment outError) {
        try {
            OcrBackendType result = impl.backend_type();
            String json = JSON.writeValueAsString(result);
            MemorySegment jsonCs = arena.allocateFrom(json);
            outResult.set(ValueLayout.ADDRESS, 0, jsonCs);
            return 0;
        } catch (Throwable e) {
            writeError(outError, e);
            return 1;
        }
    }

    private int handleSupportedLanguages(MemorySegment userData, MemorySegment outResult, MemorySegment outError) {
        try {
            List<String> result = impl.supported_languages();
            String json = JSON.writeValueAsString(result);
            MemorySegment jsonCs = arena.allocateFrom(json);
            outResult.set(ValueLayout.ADDRESS, 0, jsonCs);
            return 0;
        } catch (Throwable e) {
            writeError(outError, e);
            return 1;
        }
    }

    private int handleSupportsTableDetection(MemorySegment userData, MemorySegment outResult, MemorySegment outError) {
        try {
            boolean result = impl.supports_table_detection();
            String json = JSON.writeValueAsString(result);
            MemorySegment jsonCs = arena.allocateFrom(json);
            outResult.set(ValueLayout.ADDRESS, 0, jsonCs);
            return 0;
        } catch (Throwable e) {
            writeError(outError, e);
            return 1;
        }
    }

    private int handleSupportsDocumentProcessing(MemorySegment userData, MemorySegment outResult, MemorySegment outError) {
        try {
            boolean result = impl.supports_document_processing();
            String json = JSON.writeValueAsString(result);
            MemorySegment jsonCs = arena.allocateFrom(json);
            outResult.set(ValueLayout.ADDRESS, 0, jsonCs);
            return 0;
        } catch (Throwable e) {
            writeError(outError, e);
            return 1;
        }
    }

    private int handleProcessDocument(MemorySegment userData, MemorySegment _path_in, MemorySegment _config_in, MemorySegment outResult, MemorySegment outError) {
        try {
            java.nio.file.Path _path = java.nio.file.Paths.get(_path_in.reinterpret(Long.MAX_VALUE).getString(0));
            String _config_json = _config_in.reinterpret(Long.MAX_VALUE).getString(0);
            OcrConfig _config = JSON.readValue(_config_json, OcrConfig.class);
            ExtractionResult result = impl.process_document(_path, _config);
            String json = JSON.writeValueAsString(result);
            MemorySegment jsonCs = arena.allocateFrom(json);
            outResult.set(ValueLayout.ADDRESS, 0, jsonCs);
            return 0;
        } catch (Throwable e) {
            writeError(outError, e);
            return 1;
        }
    }

    private void writeError(MemorySegment outError, Throwable e) {
        try { outError.set(ValueLayout.ADDRESS, 0, arena.allocateFrom(e.getClass().getSimpleName() + ": " + e.getMessage())); }
        catch (Throwable ignored) { /* swallow */ }
    }

    @Override
    public void close() { arena.close(); }

    /** Register a OcrBackend implementation via Panama FFM upcall stubs. */
    public static void registerOcrBackend(final IOcrBackend impl) throws Exception {
        var bridge = new OcrBackendBridge(impl);
        try {
            try (var nameArena = Arena.ofConfined()) {
                var nameCs = nameArena.allocateFrom(impl.name());
                MemorySegment outErr = nameArena.allocate(ValueLayout.ADDRESS);
                int rc = (int) NativeLib.KREUZBERG_REGISTER_OCR_BACKEND.invoke(nameCs, bridge.vtableSegment(), MemorySegment.NULL, outErr);
                if (rc != 0) {
                    MemorySegment errPtr = outErr.get(ValueLayout.ADDRESS, 0);
                    String msg = errPtr.equals(MemorySegment.NULL) ? "registration failed (rc=" + rc + ")" : errPtr.reinterpret(Long.MAX_VALUE).getString(0);
                    throw new RuntimeException("registerOcrBackend: " + msg);
                }
            }
        } catch (Throwable t) {
            bridge.close();
            if (t instanceof Exception e) {
                throw e;
            } else {
                throw new RuntimeException("Unexpected error during registration", t);
            }
        }
        OCR_BACKEND_BRIDGES.put(impl.name(), bridge);
    }

    /** Unregister a OcrBackend implementation by name. */
    public static void unregisterOcrBackend(String name) throws Exception {
        try {
            try (var nameArena = Arena.ofConfined()) {
                var nameCs = nameArena.allocateFrom(name);
                MemorySegment outErr = nameArena.allocate(ValueLayout.ADDRESS);
                int rc = (int) NativeLib.KREUZBERG_UNREGISTER_OCR_BACKEND.invoke(nameCs, outErr);
                if (rc != 0) {
                    MemorySegment errPtr = outErr.get(ValueLayout.ADDRESS, 0);
                    String msg = errPtr.equals(MemorySegment.NULL) ? "unregistration failed (rc=" + rc + ")" : errPtr.reinterpret(Long.MAX_VALUE).getString(0);
                    throw new RuntimeException("unregisterOcrBackend: " + msg);
                }
            }
        } catch (Throwable t) {
            if (t instanceof Exception e) {
                throw e;
            } else {
                throw new RuntimeException("Unexpected error during unregistration", t);
            }
        }
        OcrBackendBridge old = OCR_BACKEND_BRIDGES.remove(name);
        if (old != null) { old.close(); }
    }
}
