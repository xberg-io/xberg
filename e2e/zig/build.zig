const std = @import("std");

pub fn build(b: *std.Build) void {
    const target = b.standardTargetOptions(.{});
    const optimize = b.standardOptimizeOption(.{});
    const test_step = b.step("test", "Run tests");
    const ffi_path = b.option([]const u8, "ffi_path", "Path to directory containing libkreuzberg_ffi") orelse "../../target/debug";
    const ffi_include = b.option([]const u8, "ffi_include_path", "Path to directory containing FFI header") orelse "../../crates/kreuzberg-ffi/include";

    const kreuzberg_module = b.addModule("kreuzberg", .{
        .root_source_file = b.path("../../packages/zig/src/kreuzberg.zig"),
        .target = target,
        .optimize = optimize,
        .link_libc = true,
    });
    kreuzberg_module.addLibraryPath(.{ .cwd_relative = ffi_path });
    kreuzberg_module.addIncludePath(.{ .cwd_relative = ffi_include });
    kreuzberg_module.linkSystemLibrary("kreuzberg_ffi", .{});

    const async_module = b.createModule(.{
        .root_source_file = b.path("src/async_test.zig"),
        .target = target,
        .optimize = optimize,
        .link_libc = true,
    });
    async_module.addImport("kreuzberg", kreuzberg_module);
    const async_tests = b.addTest(.{
        .name = "async_test",
        .root_module = async_module,
        .use_llvm = true,
    });
    b.installArtifact(async_tests);
    const async_run = b.addRunArtifact(async_tests);
    async_run.setCwd(b.path("../../test_documents"));
    test_step.dependOn(&async_run.step);

    const batch_module = b.createModule(.{
        .root_source_file = b.path("src/batch_test.zig"),
        .target = target,
        .optimize = optimize,
        .link_libc = true,
    });
    batch_module.addImport("kreuzberg", kreuzberg_module);
    const batch_tests = b.addTest(.{
        .name = "batch_test",
        .root_module = batch_module,
        .use_llvm = true,
    });
    b.installArtifact(batch_tests);
    const batch_run = b.addRunArtifact(batch_tests);
    batch_run.setCwd(b.path("../../test_documents"));
    test_step.dependOn(&batch_run.step);

    const contract_module = b.createModule(.{
        .root_source_file = b.path("src/contract_test.zig"),
        .target = target,
        .optimize = optimize,
        .link_libc = true,
    });
    contract_module.addImport("kreuzberg", kreuzberg_module);
    const contract_tests = b.addTest(.{
        .name = "contract_test",
        .root_module = contract_module,
        .use_llvm = true,
    });
    b.installArtifact(contract_tests);
    const contract_run = b.addRunArtifact(contract_tests);
    contract_run.setCwd(b.path("../../test_documents"));
    test_step.dependOn(&contract_run.step);

    const detection_module = b.createModule(.{
        .root_source_file = b.path("src/detection_test.zig"),
        .target = target,
        .optimize = optimize,
        .link_libc = true,
    });
    detection_module.addImport("kreuzberg", kreuzberg_module);
    const detection_tests = b.addTest(.{
        .name = "detection_test",
        .root_module = detection_module,
        .use_llvm = true,
    });
    b.installArtifact(detection_tests);
    const detection_run = b.addRunArtifact(detection_tests);
    detection_run.setCwd(b.path("../../test_documents"));
    test_step.dependOn(&detection_run.step);

    const embeddings_module = b.createModule(.{
        .root_source_file = b.path("src/embeddings_test.zig"),
        .target = target,
        .optimize = optimize,
        .link_libc = true,
    });
    embeddings_module.addImport("kreuzberg", kreuzberg_module);
    const embeddings_tests = b.addTest(.{
        .name = "embeddings_test",
        .root_module = embeddings_module,
        .use_llvm = true,
    });
    b.installArtifact(embeddings_tests);
    const embeddings_run = b.addRunArtifact(embeddings_tests);
    embeddings_run.setCwd(b.path("../../test_documents"));
    test_step.dependOn(&embeddings_run.step);

    const error_module = b.createModule(.{
        .root_source_file = b.path("src/error_test.zig"),
        .target = target,
        .optimize = optimize,
        .link_libc = true,
    });
    error_module.addImport("kreuzberg", kreuzberg_module);
    const error_tests = b.addTest(.{
        .name = "error_test",
        .root_module = error_module,
        .use_llvm = true,
    });
    b.installArtifact(error_tests);
    const error_run = b.addRunArtifact(error_tests);
    error_run.setCwd(b.path("../../test_documents"));
    test_step.dependOn(&error_run.step);

    const format_specific_module = b.createModule(.{
        .root_source_file = b.path("src/format_specific_test.zig"),
        .target = target,
        .optimize = optimize,
        .link_libc = true,
    });
    format_specific_module.addImport("kreuzberg", kreuzberg_module);
    const format_specific_tests = b.addTest(.{
        .name = "format_specific_test",
        .root_module = format_specific_module,
        .use_llvm = true,
    });
    b.installArtifact(format_specific_tests);
    const format_specific_run = b.addRunArtifact(format_specific_tests);
    format_specific_run.setCwd(b.path("../../test_documents"));
    test_step.dependOn(&format_specific_run.step);

    const registry_module = b.createModule(.{
        .root_source_file = b.path("src/registry_test.zig"),
        .target = target,
        .optimize = optimize,
        .link_libc = true,
    });
    registry_module.addImport("kreuzberg", kreuzberg_module);
    const registry_tests = b.addTest(.{
        .name = "registry_test",
        .root_module = registry_module,
        .use_llvm = true,
    });
    b.installArtifact(registry_tests);
    const registry_run = b.addRunArtifact(registry_tests);
    registry_run.setCwd(b.path("../../test_documents"));
    test_step.dependOn(&registry_run.step);

    const registry_operations_module = b.createModule(.{
        .root_source_file = b.path("src/registry_operations_test.zig"),
        .target = target,
        .optimize = optimize,
        .link_libc = true,
    });
    registry_operations_module.addImport("kreuzberg", kreuzberg_module);
    const registry_operations_tests = b.addTest(.{
        .name = "registry_operations_test",
        .root_module = registry_operations_module,
        .use_llvm = true,
    });
    b.installArtifact(registry_operations_tests);
    const registry_operations_run = b.addRunArtifact(registry_operations_tests);
    registry_operations_run.setCwd(b.path("../../test_documents"));
    test_step.dependOn(&registry_operations_run.step);

    const smoke_module = b.createModule(.{
        .root_source_file = b.path("src/smoke_test.zig"),
        .target = target,
        .optimize = optimize,
        .link_libc = true,
    });
    smoke_module.addImport("kreuzberg", kreuzberg_module);
    const smoke_tests = b.addTest(.{
        .name = "smoke_test",
        .root_module = smoke_module,
        .use_llvm = true,
    });
    b.installArtifact(smoke_tests);
    const smoke_run = b.addRunArtifact(smoke_tests);
    smoke_run.setCwd(b.path("../../test_documents"));
    test_step.dependOn(&smoke_run.step);

}
