package dev.kreuzberg;

import java.lang.foreign.Arena;
import java.lang.foreign.FunctionDescriptor;
import java.lang.foreign.Linker;
import java.lang.foreign.MemorySegment;
import java.lang.foreign.ValueLayout;
import java.lang.invoke.MethodHandles;
import java.lang.invoke.MethodType;
import java.util.concurrent.ConcurrentHashMap;
import com.fasterxml.jackson.databind.ObjectMapper;

/**
 * Allocates Panama FFM upcall stubs for an IRenderer implementation,
 * assembles the C vtable in native memory, and provides static
 * registerRenderer/unregisterRenderer helpers.
 */
public final class RendererBridge implements AutoCloseable {

    private static final Linker LINKER = Linker.nativeLinker();
    private static final MethodHandles.Lookup LOOKUP = MethodHandles.lookup();
    private static final ObjectMapper JSON = new ObjectMapper();

    /** Live registry — keeps Arenas and upcall stubs alive past the register call. */
    private static final ConcurrentHashMap<String, RendererBridge>
            RENDERER_BRIDGES = new ConcurrentHashMap<>();

    // C vtable: 6 fields (4 plugin methods + 1 trait methods + free_user_data)
    private static final long VTABLE_SIZE = (long) ValueLayout.ADDRESS.byteSize() * 6L;

    private final Arena arena;
    private final MemorySegment vtable;
    private final IRenderer impl;

    RendererBridge(final IRenderer impl) {
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

            var stubRender = LINKER.upcallStub(LOOKUP.bind(this, "handleRender",
                MethodType.methodType(int.class, MemorySegment.class, MemorySegment.class, MemorySegment.class, MemorySegment.class)),
                FunctionDescriptor.of(
                    ValueLayout.JAVA_INT,
                    ValueLayout.ADDRESS,
                    ValueLayout.ADDRESS,
                    ValueLayout.ADDRESS,
                    ValueLayout.ADDRESS
                ),
                arena);
            vtable.set(ValueLayout.ADDRESS, offset, stubRender);
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

    private int handleRender(MemorySegment userData, MemorySegment doc_in, MemorySegment outResult, MemorySegment outError) {
        try {
            String doc = doc_in.reinterpret(Long.MAX_VALUE).getString(0);
            String result = impl.render(doc);
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

    /** Register a Renderer implementation via Panama FFM upcall stubs. */
    public static void registerRenderer(final IRenderer impl) throws Exception {
        var bridge = new RendererBridge(impl);
        try {
            try (var nameArena = Arena.ofShared()) {
                var nameCs = nameArena.allocateFrom(impl.name());
                MemorySegment outErr = nameArena.allocate(ValueLayout.ADDRESS);
                int rc = (int) NativeLib.KREUZBERG_REGISTER_RENDERER.invoke(nameCs, bridge.vtableSegment(), MemorySegment.NULL, outErr);
                if (rc != 0) {
                    MemorySegment errPtr = outErr.get(ValueLayout.ADDRESS, 0);
                    String msg = errPtr.equals(MemorySegment.NULL)
                        ? "registration failed (rc=" + rc + ")"
                        : errPtr.reinterpret(Long.MAX_VALUE).getString(0);
                    throw new RuntimeException("registerRenderer: " + msg);
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
        RENDERER_BRIDGES.put(impl.name(), bridge);
    }

    /** Unregister a Renderer implementation by name. */
    public static void unregisterRenderer(String name) throws Exception {
        try {
            try (var nameArena = Arena.ofShared()) {
                var nameCs = nameArena.allocateFrom(name);
                MemorySegment outErr = nameArena.allocate(ValueLayout.ADDRESS);
                int rc = (int) NativeLib.KREUZBERG_UNREGISTER_RENDERER.invoke(nameCs, outErr);
                if (rc != 0) {
                    MemorySegment errPtr = outErr.get(ValueLayout.ADDRESS, 0);
                    String msg = errPtr.equals(MemorySegment.NULL)
                        ? "unregistration failed (rc=" + rc + ")"
                        : errPtr.reinterpret(Long.MAX_VALUE).getString(0);
                    throw new RuntimeException("unregisterRenderer: " + msg);
                }
            }
        } catch (Throwable t) {
            if (t instanceof Exception e) {
                throw e;
            } else {
                throw new RuntimeException("Unexpected error during unregistration", t);
            }
        }
        RendererBridge old = RENDERER_BRIDGES.remove(name);
        if (old != null) { old.close(); }
    }
    /** Clear all registered Renderer implementations. */
    public static void clearAllRenderer() throws Exception {
        try {
            try (var arena = Arena.ofShared()) {
                MemorySegment outErr = arena.allocate(ValueLayout.ADDRESS);
                int rc = (int) NativeLib.KREUZBERG_CLEAR_RENDERER.invoke(outErr);
                if (rc != 0) {
                    MemorySegment errPtr = outErr.get(ValueLayout.ADDRESS, 0);
                    String msg = errPtr.equals(MemorySegment.NULL)
                        ? "clear failed (rc=" + rc + ")"
                        : errPtr.reinterpret(Long.MAX_VALUE).getString(0);
                    throw new RuntimeException("clearAllRenderer: " + msg);
                }
            }
        } catch (Throwable t) {
            if (t instanceof Exception e) {
                throw e;
            } else {
                throw new RuntimeException("Unexpected error during clear", t);
            }
        }
        RENDERER_BRIDGES.values().forEach(RendererBridge::close);
        RENDERER_BRIDGES.clear();
    }
}
