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
 * Allocates Panama FFM upcall stubs for an IValidator implementation, assembles the C vtable in native memory, and
 * provides static registerValidator/unregisterValidator helpers.
 */
@SuppressWarnings("checkstyle:LineLength")
public final class ValidatorBridge implements AutoCloseable {

    private static final Linker LINKER = Linker.nativeLinker();
    private static final MethodHandles.Lookup LOOKUP = MethodHandles.lookup();
    private static final ObjectMapper JSON = new ObjectMapper();

    /** Live registry — keeps Arenas and upcall stubs alive past the register call. */
    private static final ConcurrentHashMap<String, ValidatorBridge> VALIDATOR_BRIDGES = new ConcurrentHashMap<>();

    // C vtable: 8 fields (4 plugin methods + 3 trait methods + free_user_data)
    private static final long VTABLE_SIZE = (long) ValueLayout.ADDRESS.byteSize() * 8L;

    private final Arena arena;
    private final MemorySegment vtable;
    private final IValidator impl;

    ValidatorBridge(final IValidator impl) {
        this.impl = impl;
        this.arena = Arena.ofShared();
        this.vtable = arena.allocate(VTABLE_SIZE);

        try {
            long offset = 0L;

            var stubName = LINKER.upcallStub(
                    LOOKUP.bind(this, "handleName", MethodType.methodType(MemorySegment.class, MemorySegment.class)),
                    FunctionDescriptor.of(ValueLayout.ADDRESS, ValueLayout.ADDRESS), arena);
            vtable.set(ValueLayout.ADDRESS, offset, stubName);
            offset += ValueLayout.ADDRESS.byteSize();

            var stubVersion = LINKER.upcallStub(
                    LOOKUP.bind(this, "handleVersion", MethodType.methodType(MemorySegment.class, MemorySegment.class)),
                    FunctionDescriptor.of(ValueLayout.ADDRESS, ValueLayout.ADDRESS), arena);
            vtable.set(ValueLayout.ADDRESS, offset, stubVersion);
            offset += ValueLayout.ADDRESS.byteSize();

            var stubInitialize = LINKER.upcallStub(
                    LOOKUP.bind(this, "handleInitialize",
                            MethodType.methodType(int.class, MemorySegment.class, MemorySegment.class)),
                    FunctionDescriptor.of(ValueLayout.JAVA_INT, ValueLayout.ADDRESS, ValueLayout.ADDRESS), arena);
            vtable.set(ValueLayout.ADDRESS, offset, stubInitialize);
            offset += ValueLayout.ADDRESS.byteSize();

            var stubShutdown = LINKER.upcallStub(
                    LOOKUP.bind(this, "handleShutdown",
                            MethodType.methodType(int.class, MemorySegment.class, MemorySegment.class)),
                    FunctionDescriptor.of(ValueLayout.JAVA_INT, ValueLayout.ADDRESS, ValueLayout.ADDRESS), arena);
            vtable.set(ValueLayout.ADDRESS, offset, stubShutdown);
            offset += ValueLayout.ADDRESS.byteSize();

            var stubValidate = LINKER.upcallStub(
                    LOOKUP.bind(this, "handleValidate",
                            MethodType.methodType(int.class, MemorySegment.class, MemorySegment.class,
                                    MemorySegment.class, MemorySegment.class)),
                    FunctionDescriptor.of(ValueLayout.JAVA_INT, ValueLayout.ADDRESS, ValueLayout.ADDRESS,
                            ValueLayout.ADDRESS, ValueLayout.ADDRESS),
                    arena);
            vtable.set(ValueLayout.ADDRESS, offset, stubValidate);
            offset += ValueLayout.ADDRESS.byteSize();

            var stubShouldValidate = LINKER.upcallStub(
                    LOOKUP.bind(this, "handleShouldValidate",
                            MethodType.methodType(int.class, MemorySegment.class, MemorySegment.class,
                                    MemorySegment.class, MemorySegment.class, MemorySegment.class)),
                    FunctionDescriptor.of(ValueLayout.JAVA_INT, ValueLayout.ADDRESS, ValueLayout.ADDRESS,
                            ValueLayout.ADDRESS, ValueLayout.ADDRESS, ValueLayout.ADDRESS),
                    arena);
            vtable.set(ValueLayout.ADDRESS, offset, stubShouldValidate);
            offset += ValueLayout.ADDRESS.byteSize();

            var stubPriority = LINKER.upcallStub(
                    LOOKUP.bind(this, "handlePriority",
                            MethodType.methodType(int.class, MemorySegment.class, MemorySegment.class,
                                    MemorySegment.class)),
                    FunctionDescriptor.of(ValueLayout.JAVA_INT, ValueLayout.ADDRESS, ValueLayout.ADDRESS,
                            ValueLayout.ADDRESS),
                    arena);
            vtable.set(ValueLayout.ADDRESS, offset, stubPriority);
            offset += ValueLayout.ADDRESS.byteSize();

            vtable.set(ValueLayout.ADDRESS, offset, MemorySegment.NULL);

        } catch (ReflectiveOperationException e) {
            arena.close();
            throw new RuntimeException("Failed to create trait bridge stubs", e);
        }
    }

    MemorySegment vtableSegment() {
        return vtable;
    }

    private MemorySegment handleName(MemorySegment userData) {
        try {
            return arena.allocateFrom(impl.name());
        } catch (Throwable e) {
            return MemorySegment.NULL;
        }
    }

    private MemorySegment handleVersion(MemorySegment userData) {
        try {
            return arena.allocateFrom(impl.version());
        } catch (Throwable e) {
            return MemorySegment.NULL;
        }
    }

    private int handleInitialize(MemorySegment userData, MemorySegment outError) {
        try {
            impl.initialize();
            return 0;
        } catch (Throwable e) {
            writeError(outError, e);
            return 1;
        }
    }

    private int handleShutdown(MemorySegment userData, MemorySegment outError) {
        try {
            impl.shutdown();
            return 0;
        } catch (Throwable e) {
            writeError(outError, e);
            return 1;
        }
    }

    private int handleValidate(MemorySegment userData, MemorySegment result_in, MemorySegment config_in,
            MemorySegment outError) {
        try {
            String result_json = result_in.reinterpret(Long.MAX_VALUE).getString(0);
            ExtractionResult result = JSON.readValue(result_json, ExtractionResult.class);
            String config_json = config_in.reinterpret(Long.MAX_VALUE).getString(0);
            ExtractionConfig config = JSON.readValue(config_json, ExtractionConfig.class);
            impl.validate(result, config);
            return 0;
        } catch (Throwable e) {
            writeError(outError, e);
            return 1;
        }
    }

    private int handleShouldValidate(MemorySegment userData, MemorySegment _result_in, MemorySegment _config_in,
            MemorySegment outResult, MemorySegment outError) {
        try {
            String _result_json = _result_in.reinterpret(Long.MAX_VALUE).getString(0);
            ExtractionResult _result = JSON.readValue(_result_json, ExtractionResult.class);
            String _config_json = _config_in.reinterpret(Long.MAX_VALUE).getString(0);
            ExtractionConfig _config = JSON.readValue(_config_json, ExtractionConfig.class);
            boolean result = impl.should_validate(_result, _config);
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

    private void writeError(MemorySegment outError, Throwable e) {
        try {
            outError.set(ValueLayout.ADDRESS, 0,
                    arena.allocateFrom(e.getClass().getSimpleName() + ": " + e.getMessage()));
        } catch (Throwable ignored) {
            /* swallow */ }
    }

    @Override
    public void close() {
        arena.close();
    }

    /** Register a Validator implementation via Panama FFM upcall stubs. */
    public static void registerValidator(final IValidator impl) throws Exception {
        var bridge = new ValidatorBridge(impl);
        try {
            try (var nameArena = Arena.ofConfined()) {
                var nameCs = nameArena.allocateFrom(impl.name());
                MemorySegment outErr = nameArena.allocate(ValueLayout.ADDRESS);
                int rc = (int) NativeLib.KREUZBERG_REGISTER_VALIDATOR.invoke(nameCs, bridge.vtableSegment(),
                        MemorySegment.NULL, outErr);
                if (rc != 0) {
                    MemorySegment errPtr = outErr.get(ValueLayout.ADDRESS, 0);
                    String msg = errPtr.equals(MemorySegment.NULL)
                            ? "registration failed (rc=" + rc + ")"
                            : errPtr.reinterpret(Long.MAX_VALUE).getString(0);
                    throw new RuntimeException("registerValidator: " + msg);
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
        VALIDATOR_BRIDGES.put(impl.name(), bridge);
    }

    /** Unregister a Validator implementation by name. */
    public static void unregisterValidator(String name) throws Exception {
        try {
            try (var nameArena = Arena.ofConfined()) {
                var nameCs = nameArena.allocateFrom(name);
                MemorySegment outErr = nameArena.allocate(ValueLayout.ADDRESS);
                int rc = (int) NativeLib.KREUZBERG_UNREGISTER_VALIDATOR.invoke(nameCs, outErr);
                if (rc != 0) {
                    MemorySegment errPtr = outErr.get(ValueLayout.ADDRESS, 0);
                    String msg = errPtr.equals(MemorySegment.NULL)
                            ? "unregistration failed (rc=" + rc + ")"
                            : errPtr.reinterpret(Long.MAX_VALUE).getString(0);
                    throw new RuntimeException("unregisterValidator: " + msg);
                }
            }
        } catch (Throwable t) {
            if (t instanceof Exception e) {
                throw e;
            } else {
                throw new RuntimeException("Unexpected error during unregistration", t);
            }
        }
        ValidatorBridge old = VALIDATOR_BRIDGES.remove(name);
        if (old != null) {
            old.close();
        }
    }
}
