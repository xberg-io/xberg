package dev.kreuzberg;

import java.lang.foreign.Arena;
import java.lang.foreign.FunctionDescriptor;
import java.lang.foreign.Linker;
import java.lang.foreign.MemoryLayout;
import java.lang.foreign.MemorySegment;
import java.lang.foreign.ValueLayout;
import java.lang.invoke.MethodHandles;
import java.lang.invoke.MethodType;
import java.util.List;
import java.util.concurrent.ConcurrentHashMap;
import com.fasterxml.jackson.databind.ObjectMapper;

/**
 * Allocates Panama FFM upcall stubs for an IOcrBackend implementation,
 * assembles the C vtable in native memory, and provides static
 * registerOcrBackend/unregisterOcrBackend helpers.
 */
@SuppressWarnings("PMD")
public final class OcrBackendBridge implements AutoCloseable {

    private static final Linker LINKER = Linker.nativeLinker();
    private static final MethodHandles.Lookup LOOKUP = MethodHandles.lookup();
    private static final ObjectMapper JSON = new ObjectMapper();

    /** Live registry — keeps Arenas and upcall stubs alive past the register call. */
    private static final ConcurrentHashMap<String, OcrBackendBridge>
            OCR_BACKEND_BRIDGES = new ConcurrentHashMap<>();

    // C vtable: 15 fields (4 plugin methods + 9 trait methods + free_string + free_user_data)
    private static final MemoryLayout VTABLE_LAYOUT = MemoryLayout.structLayout(
            ValueLayout.ADDRESS,
            ValueLayout.ADDRESS,
            ValueLayout.ADDRESS,
            ValueLayout.ADDRESS,
            ValueLayout.ADDRESS,
            ValueLayout.ADDRESS,
            ValueLayout.ADDRESS,
            ValueLayout.ADDRESS,
            ValueLayout.ADDRESS,
            ValueLayout.ADDRESS,
            ValueLayout.ADDRESS,
            ValueLayout.ADDRESS,
            ValueLayout.ADDRESS,
            ValueLayout.ADDRESS,
            ValueLayout.ADDRESS
    );
    private static final long VTABLE_SIZE = VTABLE_LAYOUT.byteSize();

    private final Arena arena;
    private final MemorySegment vtable;
    private final IOcrBackend impl;

    OcrBackendBridge(final IOcrBackend impl) {
        this.impl = impl;
        this.arena = Arena.ofShared();
        this.vtable = arena.allocate(VTABLE_SIZE);
        try {
            initializeStubs();
        } catch (ReflectiveOperationException e) {
            arena.close();
            throw new RuntimeException("Failed to create trait bridge stubs", e);
        }
    }

    private void initializeStubs() throws ReflectiveOperationException {
        long offset = 0L;

        var stubName = LINKER.upcallStub(LOOKUP.bind(this, "handleName",
            MethodType.methodType(int.class, MemorySegment.class, MemorySegment.class, MemorySegment.class)),
            FunctionDescriptor.of(ValueLayout.JAVA_INT, ValueLayout.ADDRESS, ValueLayout.ADDRESS, ValueLayout.ADDRESS),
            arena);
        vtable.set(ValueLayout.ADDRESS, offset, stubName);
        offset += ValueLayout.ADDRESS.byteSize();

        var stubVersion = LINKER.upcallStub(LOOKUP.bind(this, "handleVersion",
            MethodType.methodType(int.class, MemorySegment.class, MemorySegment.class, MemorySegment.class)),
            FunctionDescriptor.of(ValueLayout.JAVA_INT, ValueLayout.ADDRESS, ValueLayout.ADDRESS, ValueLayout.ADDRESS),
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
            MethodType.methodType(
                int.class,
                MemorySegment.class,
                MemorySegment.class,
                long.class,
                MemorySegment.class,
                MemorySegment.class,
                MemorySegment.class
            )),
            FunctionDescriptor.of(
                ValueLayout.JAVA_INT,
                ValueLayout.ADDRESS,
                ValueLayout.ADDRESS,
                ValueLayout.JAVA_LONG,
                ValueLayout.ADDRESS,
                ValueLayout.ADDRESS,
                ValueLayout.ADDRESS
            ),
            arena);
        vtable.set(ValueLayout.ADDRESS, offset, stubProcessImage);
        offset += ValueLayout.ADDRESS.byteSize();

        var stubProcessImageFile = LINKER.upcallStub(LOOKUP.bind(this, "handleProcessImageFile",
            MethodType.methodType(
                int.class,
                MemorySegment.class,
                MemorySegment.class,
                MemorySegment.class,
                MemorySegment.class,
                MemorySegment.class
            )),
            FunctionDescriptor.of(
                ValueLayout.JAVA_INT,
                ValueLayout.ADDRESS,
                ValueLayout.ADDRESS,
                ValueLayout.ADDRESS,
                ValueLayout.ADDRESS,
                ValueLayout.ADDRESS
            ),
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

        var stubEmitsStructuredMarkdown = LINKER.upcallStub(LOOKUP.bind(this, "handleEmitsStructuredMarkdown",
            MethodType.methodType(int.class, MemorySegment.class, MemorySegment.class, MemorySegment.class)),
            FunctionDescriptor.of(ValueLayout.JAVA_INT, ValueLayout.ADDRESS, ValueLayout.ADDRESS, ValueLayout.ADDRESS),
            arena);
        vtable.set(ValueLayout.ADDRESS, offset, stubEmitsStructuredMarkdown);
        offset += ValueLayout.ADDRESS.byteSize();

        var stubProcessDocument = LINKER.upcallStub(LOOKUP.bind(this, "handleProcessDocument",
            MethodType.methodType(
                int.class,
                MemorySegment.class,
                MemorySegment.class,
                MemorySegment.class,
                MemorySegment.class,
                MemorySegment.class
            )),
            FunctionDescriptor.of(
                ValueLayout.JAVA_INT,
                ValueLayout.ADDRESS,
                ValueLayout.ADDRESS,
                ValueLayout.ADDRESS,
                ValueLayout.ADDRESS,
                ValueLayout.ADDRESS
            ),
            arena);
        vtable.set(ValueLayout.ADDRESS, offset, stubProcessDocument);
        offset += ValueLayout.ADDRESS.byteSize();

        var stubFreeString = LINKER.upcallStub(LOOKUP.bind(this, "freeString",
            MethodType.methodType(void.class, MemorySegment.class)),
            FunctionDescriptor.ofVoid(ValueLayout.ADDRESS),
            arena);
        vtable.set(ValueLayout.ADDRESS, offset, stubFreeString);
        offset += ValueLayout.ADDRESS.byteSize();

        var stubFreeUserData = LINKER.upcallStub(LOOKUP.bind(this, "freeUserData",
            MethodType.methodType(void.class, MemorySegment.class)),
            FunctionDescriptor.ofVoid(ValueLayout.ADDRESS),
            arena);
        vtable.set(ValueLayout.ADDRESS, offset, stubFreeUserData);
        offset += ValueLayout.ADDRESS.byteSize();

    }

    MemorySegment vtableSegment() { return vtable; }

    private int handleName(MemorySegment userData, MemorySegment outName, MemorySegment outError) {
        try {
            outName.set(ValueLayout.ADDRESS, 0, arena.allocateFrom(impl.name()));
            return 0;
        } catch (Throwable e) { return 1; }
    }

    private int handleVersion(MemorySegment userData, MemorySegment outVersion, MemorySegment outError) {
        try {
            outVersion.set(ValueLayout.ADDRESS, 0, arena.allocateFrom(impl.version()));
            return 0;
        } catch (Throwable e) { return 1; }
    }

    private int handleInitialize(MemorySegment userData, MemorySegment outError) {
        try {
            impl.initialize();
            return 0;
        } catch (Throwable e) { return 1; }
    }

    private int handleShutdown(MemorySegment userData, MemorySegment outError) {
        try {
            impl.shutdown();
            return 0;
        } catch (Throwable e) { return 1; }
    }

    private int handleProcessImage(
        MemorySegment userData,
        MemorySegment image_bytes_in,
        long image_bytesLen,
        MemorySegment config_in,
        MemorySegment outResult,
        MemorySegment outError
    ) {
        try {
            byte[] image_bytes = image_bytes_in.reinterpret(image_bytesLen).toArray(ValueLayout.JAVA_BYTE);
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

    private int handleProcessImageFile(
        MemorySegment userData,
        MemorySegment path_in,
        MemorySegment config_in,
        MemorySegment outResult,
        MemorySegment outError
    ) {
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
            String result = impl.backend_type();
            MemorySegment jsonCs = arena.allocateFrom(result);
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

    private int handleEmitsStructuredMarkdown(MemorySegment userData, MemorySegment outResult, MemorySegment outError) {
        try {
            boolean result = impl.emits_structured_markdown();
            String json = JSON.writeValueAsString(result);
            MemorySegment jsonCs = arena.allocateFrom(json);
            outResult.set(ValueLayout.ADDRESS, 0, jsonCs);
            return 0;
        } catch (Throwable e) {
            writeError(outError, e);
            return 1;
        }
    }

    private int handleProcessDocument(
        MemorySegment userData,
        MemorySegment _path_in,
        MemorySegment _config_in,
        MemorySegment outResult,
        MemorySegment outError
    ) {
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

    private void freeString(MemorySegment ptr) {
        // Strings returned by Java callbacks are arena-owned and released when this bridge closes.
    }

    private void freeUserData(MemorySegment userData) {
        // User data is Java-side state (the impl object), not freed by Rust on drop.
    }

    /** Read a NUL-terminated native C string safely without unbounded reinterpret. */
    private static String readNativeString(MemorySegment ptr) {
        return ptr.reinterpret(4096).getString(0);
    }

    @Override
    public void close() { arena.close(); }

    /** Register a OcrBackend implementation via Panama FFM upcall stubs. */
    public static void registerOcrBackend(final IOcrBackend impl) throws Exception {
        var bridge = new OcrBackendBridge(impl);
        try {
            try (var nameArena = Arena.ofShared()) {
                var nameCs = nameArena.allocateFrom(impl.name());
                MemorySegment outErr = nameArena.allocate(ValueLayout.ADDRESS);
                int rc = (int) NativeLib.KREUZBERG_REGISTER_OCR_BACKEND.invoke(nameCs, bridge.vtableSegment(), MemorySegment.NULL, outErr);
                if (rc != 0) {
                    MemorySegment errPtr = outErr.get(ValueLayout.ADDRESS, 0);
                    String msg = errPtr.equals(MemorySegment.NULL) ? "registration failed (rc=" + rc + ")" : readNativeString(errPtr);
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
            try (var nameArena = Arena.ofShared()) {
                var nameCs = nameArena.allocateFrom(name);
                MemorySegment outErr = nameArena.allocate(ValueLayout.ADDRESS);
                int rc = (int) NativeLib.KREUZBERG_UNREGISTER_OCR_BACKEND.invoke(nameCs, outErr);
                if (rc != 0) {
                    MemorySegment errPtr = outErr.get(ValueLayout.ADDRESS, 0);
                    String msg = errPtr.equals(MemorySegment.NULL)
                        ? "unregistration failed (rc=" + rc + ")"
                        : errPtr.reinterpret(Long.MAX_VALUE).getString(0);
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
    /** Clear all registered OcrBackend implementations. */
    public static void clearOcrBackends() throws Exception {
        try {
            try (var arena = Arena.ofShared()) {
                MemorySegment outErr = arena.allocate(ValueLayout.ADDRESS);
                int rc = (int) NativeLib.KREUZBERG_CLEAR_OCR_BACKEND.invoke(outErr);
                if (rc != 0) {
                    MemorySegment errPtr = outErr.get(ValueLayout.ADDRESS, 0);
                    String msg = errPtr.equals(MemorySegment.NULL)
                        ? "clear failed (rc=" + rc + ")"
                        : errPtr.reinterpret(Long.MAX_VALUE).getString(0);
                    throw new RuntimeException("clearOcrBackends: " + msg);
                }
            }
        } catch (Throwable t) {
            if (t instanceof Exception e) {
                throw e;
            } else {
                throw new RuntimeException("Unexpected error during clear", t);
            }
        }
        OCR_BACKEND_BRIDGES.values().forEach(OcrBackendBridge::close);
        OCR_BACKEND_BRIDGES.clear();
    }
}
