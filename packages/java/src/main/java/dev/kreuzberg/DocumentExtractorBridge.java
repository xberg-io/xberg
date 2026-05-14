package dev.kreuzberg;

import java.lang.foreign.Arena;
import java.lang.foreign.FunctionDescriptor;
import java.lang.foreign.Linker;
import java.lang.foreign.MemorySegment;
import java.lang.foreign.ValueLayout;
import java.lang.invoke.MethodHandles;
import java.lang.invoke.MethodType;
import java.util.List;
import java.util.concurrent.ConcurrentHashMap;
import com.fasterxml.jackson.databind.ObjectMapper;

/**
 * Allocates Panama FFM upcall stubs for an IDocumentExtractor implementation,
 * assembles the C vtable in native memory, and provides static
 * registerDocumentExtractor/unregisterDocumentExtractor helpers.
 */
public final class DocumentExtractorBridge implements AutoCloseable {

    private static final Linker LINKER = Linker.nativeLinker();
    private static final MethodHandles.Lookup LOOKUP = MethodHandles.lookup();
    private static final ObjectMapper JSON = new ObjectMapper();

    /** Live registry — keeps Arenas and upcall stubs alive past the register call. */
    private static final ConcurrentHashMap<String, DocumentExtractorBridge>
            DOCUMENT_EXTRACTOR_BRIDGES = new ConcurrentHashMap<>();

    // C vtable: 11 fields (4 plugin methods + 6 trait methods + free_user_data)
    private static final long VTABLE_SIZE = (long) ValueLayout.ADDRESS.byteSize() * 11L;

    private final Arena arena;
    private final MemorySegment vtable;
    private final IDocumentExtractor impl;

    DocumentExtractorBridge(final IDocumentExtractor impl) {
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

            var stubExtractBytes = LINKER.upcallStub(LOOKUP.bind(this, "handleExtractBytes",
                MethodType.methodType(
                    int.class,
                    MemorySegment.class,
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
                    ValueLayout.ADDRESS,
                    ValueLayout.ADDRESS
                ),
                arena);
            vtable.set(ValueLayout.ADDRESS, offset, stubExtractBytes);
            offset += ValueLayout.ADDRESS.byteSize();

            var stubExtractFile = LINKER.upcallStub(LOOKUP.bind(this, "handleExtractFile",
                MethodType.methodType(
                    int.class,
                    MemorySegment.class,
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
                    ValueLayout.ADDRESS,
                    ValueLayout.ADDRESS
                ),
                arena);
            vtable.set(ValueLayout.ADDRESS, offset, stubExtractFile);
            offset += ValueLayout.ADDRESS.byteSize();

            var stubSupportedMimeTypes = LINKER.upcallStub(LOOKUP.bind(this, "handleSupportedMimeTypes",
                MethodType.methodType(int.class, MemorySegment.class, MemorySegment.class, MemorySegment.class)),
                FunctionDescriptor.of(ValueLayout.JAVA_INT, ValueLayout.ADDRESS, ValueLayout.ADDRESS, ValueLayout.ADDRESS),
                arena);
            vtable.set(ValueLayout.ADDRESS, offset, stubSupportedMimeTypes);
            offset += ValueLayout.ADDRESS.byteSize();

            var stubPriority = LINKER.upcallStub(LOOKUP.bind(this, "handlePriority",
                MethodType.methodType(int.class, MemorySegment.class, MemorySegment.class, MemorySegment.class)),
                FunctionDescriptor.of(ValueLayout.JAVA_INT, ValueLayout.ADDRESS, ValueLayout.ADDRESS, ValueLayout.ADDRESS),
                arena);
            vtable.set(ValueLayout.ADDRESS, offset, stubPriority);
            offset += ValueLayout.ADDRESS.byteSize();

            var stubCanHandle = LINKER.upcallStub(LOOKUP.bind(this, "handleCanHandle",
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
            vtable.set(ValueLayout.ADDRESS, offset, stubCanHandle);
            offset += ValueLayout.ADDRESS.byteSize();

            var stubAsSyncExtractor = LINKER.upcallStub(LOOKUP.bind(this, "handleAsSyncExtractor",
                MethodType.methodType(int.class, MemorySegment.class, MemorySegment.class, MemorySegment.class)),
                FunctionDescriptor.of(ValueLayout.JAVA_INT, ValueLayout.ADDRESS, ValueLayout.ADDRESS, ValueLayout.ADDRESS),
                arena);
            vtable.set(ValueLayout.ADDRESS, offset, stubAsSyncExtractor);
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

    private int handleExtractBytes(
        MemorySegment userData,
        MemorySegment content_in,
        MemorySegment mime_type_in,
        MemorySegment config_in,
        MemorySegment outResult,
        MemorySegment outError
    ) {
        try {
            byte[] content = content_in.reinterpret(Long.MAX_VALUE).toArray(ValueLayout.JAVA_BYTE);
            String mime_type = mime_type_in.reinterpret(Long.MAX_VALUE).getString(0);
            String config_json = config_in.reinterpret(Long.MAX_VALUE).getString(0);
            ExtractionConfig config = JSON.readValue(config_json, ExtractionConfig.class);
            String result = impl.extract_bytes(content, mime_type, config);
            String json = JSON.writeValueAsString(result);
            MemorySegment jsonCs = arena.allocateFrom(json);
            outResult.set(ValueLayout.ADDRESS, 0, jsonCs);
            return 0;
        } catch (Throwable e) {
            writeError(outError, e);
            return 1;
        }
    }

    private int handleExtractFile(
        MemorySegment userData,
        MemorySegment path_in,
        MemorySegment mime_type_in,
        MemorySegment config_in,
        MemorySegment outResult,
        MemorySegment outError
    ) {
        try {
            java.nio.file.Path path = java.nio.file.Paths.get(path_in.reinterpret(Long.MAX_VALUE).getString(0));
            String mime_type = mime_type_in.reinterpret(Long.MAX_VALUE).getString(0);
            String config_json = config_in.reinterpret(Long.MAX_VALUE).getString(0);
            ExtractionConfig config = JSON.readValue(config_json, ExtractionConfig.class);
            String result = impl.extract_file(path, mime_type, config);
            String json = JSON.writeValueAsString(result);
            MemorySegment jsonCs = arena.allocateFrom(json);
            outResult.set(ValueLayout.ADDRESS, 0, jsonCs);
            return 0;
        } catch (Throwable e) {
            writeError(outError, e);
            return 1;
        }
    }

    private int handleSupportedMimeTypes(MemorySegment userData, MemorySegment outResult, MemorySegment outError) {
        try {
            List<String> result = impl.supported_mime_types();
            String json = JSON.writeValueAsString(result);
            MemorySegment jsonCs = arena.allocateFrom(json);
            outResult.set(ValueLayout.ADDRESS, 0, jsonCs);
            return 0;
        } catch (Throwable e) {
            writeError(outError, e);
            return 1;
        }
    }

    private int handlePriority(MemorySegment userData, MemorySegment outResult, MemorySegment outError) {
        try {
            int result = impl.priority();
            String json = JSON.writeValueAsString(result);
            MemorySegment jsonCs = arena.allocateFrom(json);
            outResult.set(ValueLayout.ADDRESS, 0, jsonCs);
            return 0;
        } catch (Throwable e) {
            writeError(outError, e);
            return 1;
        }
    }

    private int handleCanHandle(
        MemorySegment userData,
        MemorySegment _path_in,
        MemorySegment _mime_type_in,
        MemorySegment outResult,
        MemorySegment outError
    ) {
        try {
            java.nio.file.Path _path = java.nio.file.Paths.get(_path_in.reinterpret(Long.MAX_VALUE).getString(0));
            String _mime_type = _mime_type_in.reinterpret(Long.MAX_VALUE).getString(0);
            boolean result = impl.can_handle(_path, _mime_type);
            String json = JSON.writeValueAsString(result);
            MemorySegment jsonCs = arena.allocateFrom(json);
            outResult.set(ValueLayout.ADDRESS, 0, jsonCs);
            return 0;
        } catch (Throwable e) {
            writeError(outError, e);
            return 1;
        }
    }

    private int handleAsSyncExtractor(MemorySegment userData, MemorySegment outResult, MemorySegment outError) {
        try {
            String result = impl.as_sync_extractor();
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

    /** Register a DocumentExtractor implementation via Panama FFM upcall stubs. */
    public static void registerDocumentExtractor(final IDocumentExtractor impl) throws Exception {
        var bridge = new DocumentExtractorBridge(impl);
        try {
            try (var nameArena = Arena.ofShared()) {
                var nameCs = nameArena.allocateFrom(impl.name());
                MemorySegment outErr = nameArena.allocate(ValueLayout.ADDRESS);
                int rc = (int) NativeLib.KREUZBERG_REGISTER_DOCUMENT_EXTRACTOR.invoke(
                    nameCs,
                    bridge.vtableSegment(),
                    MemorySegment.NULL,
                    outErr
                );
                if (rc != 0) {
                    MemorySegment errPtr = outErr.get(ValueLayout.ADDRESS, 0);
                    String msg = errPtr.equals(MemorySegment.NULL)
                        ? "registration failed (rc=" + rc + ")"
                        : errPtr.reinterpret(Long.MAX_VALUE).getString(0);
                    throw new RuntimeException("registerDocumentExtractor: " + msg);
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
        DOCUMENT_EXTRACTOR_BRIDGES.put(impl.name(), bridge);
    }

    /** Unregister a DocumentExtractor implementation by name. */
    public static void unregisterDocumentExtractor(String name) throws Exception {
        try {
            try (var nameArena = Arena.ofShared()) {
                var nameCs = nameArena.allocateFrom(name);
                MemorySegment outErr = nameArena.allocate(ValueLayout.ADDRESS);
                int rc = (int) NativeLib.KREUZBERG_UNREGISTER_DOCUMENT_EXTRACTOR.invoke(nameCs, outErr);
                if (rc != 0) {
                    MemorySegment errPtr = outErr.get(ValueLayout.ADDRESS, 0);
                    String msg = errPtr.equals(MemorySegment.NULL)
                        ? "unregistration failed (rc=" + rc + ")"
                        : errPtr.reinterpret(Long.MAX_VALUE).getString(0);
                    throw new RuntimeException("unregisterDocumentExtractor: " + msg);
                }
            }
        } catch (Throwable t) {
            if (t instanceof Exception e) {
                throw e;
            } else {
                throw new RuntimeException("Unexpected error during unregistration", t);
            }
        }
        DocumentExtractorBridge old = DOCUMENT_EXTRACTOR_BRIDGES.remove(name);
        if (old != null) { old.close(); }
    }
    /** Clear all registered DocumentExtractor implementations. */
    public static void clearAllDocumentExtractor() throws Exception {
        try {
            try (var arena = Arena.ofShared()) {
                MemorySegment outErr = arena.allocate(ValueLayout.ADDRESS);
                int rc = (int) NativeLib.KREUZBERG_CLEAR_DOCUMENT_EXTRACTOR.invoke(outErr);
                if (rc != 0) {
                    MemorySegment errPtr = outErr.get(ValueLayout.ADDRESS, 0);
                    String msg = errPtr.equals(MemorySegment.NULL)
                        ? "clear failed (rc=" + rc + ")"
                        : errPtr.reinterpret(Long.MAX_VALUE).getString(0);
                    throw new RuntimeException("clearAllDocumentExtractor: " + msg);
                }
            }
        } catch (Throwable t) {
            if (t instanceof Exception e) {
                throw e;
            } else {
                throw new RuntimeException("Unexpected error during clear", t);
            }
        }
        DOCUMENT_EXTRACTOR_BRIDGES.values().forEach(DocumentExtractorBridge::close);
        DOCUMENT_EXTRACTOR_BRIDGES.clear();
    }
}
