const std = @import("std");

pub fn build(b: *std.Build) void {
    const target = b.standardTargetOptions(.{});
    const optimize = b.standardOptimizeOption(.{});
    const test_step = b.step("test", "Run tests");
    const ffi_path = b.option([]const u8, "ffi_path", "Path to directory containing libkreuzberg_ffi") orelse "../../target/release";
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
    const batch_run = b.addRunArtifact(batch_tests);
    batch_run.setCwd(b.path("../../test_documents"));
    test_step.dependOn(&batch_run.step);

    const code_module = b.createModule(.{
        .root_source_file = b.path("src/code_test.zig"),
        .target = target,
        .optimize = optimize,
        .link_libc = true,
    });
    code_module.addImport("kreuzberg", kreuzberg_module);
    const code_tests = b.addTest(.{
        .name = "code_test",
        .root_module = code_module,
        .use_llvm = true,
    });
    const code_run = b.addRunArtifact(code_tests);
    code_run.setCwd(b.path("../../test_documents"));
    test_step.dependOn(&code_run.step);

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
    const detection_run = b.addRunArtifact(detection_tests);
    detection_run.setCwd(b.path("../../test_documents"));
    test_step.dependOn(&detection_run.step);

    const document_extractor_management_module = b.createModule(.{
        .root_source_file = b.path("src/document_extractor_management_test.zig"),
        .target = target,
        .optimize = optimize,
        .link_libc = true,
    });
    document_extractor_management_module.addImport("kreuzberg", kreuzberg_module);
    const document_extractor_management_tests = b.addTest(.{
        .name = "document_extractor_management_test",
        .root_module = document_extractor_management_module,
        .use_llvm = true,
    });
    const document_extractor_management_run = b.addRunArtifact(document_extractor_management_tests);
    document_extractor_management_run.setCwd(b.path("../../test_documents"));
    test_step.dependOn(&document_extractor_management_run.step);

    const embed_async_pending_module = b.createModule(.{
        .root_source_file = b.path("src/embed_async_pending_test.zig"),
        .target = target,
        .optimize = optimize,
        .link_libc = true,
    });
    embed_async_pending_module.addImport("kreuzberg", kreuzberg_module);
    const embed_async_pending_tests = b.addTest(.{
        .name = "embed_async_pending_test",
        .root_module = embed_async_pending_module,
        .use_llvm = true,
    });
    const embed_async_pending_run = b.addRunArtifact(embed_async_pending_tests);
    embed_async_pending_run.setCwd(b.path("../../test_documents"));
    test_step.dependOn(&embed_async_pending_run.step);

    const embed_extra_module = b.createModule(.{
        .root_source_file = b.path("src/embed_extra_test.zig"),
        .target = target,
        .optimize = optimize,
        .link_libc = true,
    });
    embed_extra_module.addImport("kreuzberg", kreuzberg_module);
    const embed_extra_tests = b.addTest(.{
        .name = "embed_extra_test",
        .root_module = embed_extra_module,
        .use_llvm = true,
    });
    const embed_extra_run = b.addRunArtifact(embed_extra_tests);
    embed_extra_run.setCwd(b.path("../../test_documents"));
    test_step.dependOn(&embed_extra_run.step);

    const embedding_backend_management_module = b.createModule(.{
        .root_source_file = b.path("src/embedding_backend_management_test.zig"),
        .target = target,
        .optimize = optimize,
        .link_libc = true,
    });
    embedding_backend_management_module.addImport("kreuzberg", kreuzberg_module);
    const embedding_backend_management_tests = b.addTest(.{
        .name = "embedding_backend_management_test",
        .root_module = embedding_backend_management_module,
        .use_llvm = true,
    });
    const embedding_backend_management_run = b.addRunArtifact(embedding_backend_management_tests);
    embedding_backend_management_run.setCwd(b.path("../../test_documents"));
    test_step.dependOn(&embedding_backend_management_run.step);

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
    const format_specific_run = b.addRunArtifact(format_specific_tests);
    format_specific_run.setCwd(b.path("../../test_documents"));
    test_step.dependOn(&format_specific_run.step);

    const mime_utilities_module = b.createModule(.{
        .root_source_file = b.path("src/mime_utilities_test.zig"),
        .target = target,
        .optimize = optimize,
        .link_libc = true,
    });
    mime_utilities_module.addImport("kreuzberg", kreuzberg_module);
    const mime_utilities_tests = b.addTest(.{
        .name = "mime_utilities_test",
        .root_module = mime_utilities_module,
        .use_llvm = true,
    });
    const mime_utilities_run = b.addRunArtifact(mime_utilities_tests);
    mime_utilities_run.setCwd(b.path("../../test_documents"));
    test_step.dependOn(&mime_utilities_run.step);

    const ocr_backend_management_module = b.createModule(.{
        .root_source_file = b.path("src/ocr_backend_management_test.zig"),
        .target = target,
        .optimize = optimize,
        .link_libc = true,
    });
    ocr_backend_management_module.addImport("kreuzberg", kreuzberg_module);
    const ocr_backend_management_tests = b.addTest(.{
        .name = "ocr_backend_management_test",
        .root_module = ocr_backend_management_module,
        .use_llvm = true,
    });
    const ocr_backend_management_run = b.addRunArtifact(ocr_backend_management_tests);
    ocr_backend_management_run.setCwd(b.path("../../test_documents"));
    test_step.dependOn(&ocr_backend_management_run.step);

    const pdf_module = b.createModule(.{
        .root_source_file = b.path("src/pdf_test.zig"),
        .target = target,
        .optimize = optimize,
        .link_libc = true,
    });
    pdf_module.addImport("kreuzberg", kreuzberg_module);
    const pdf_tests = b.addTest(.{
        .name = "pdf_test",
        .root_module = pdf_module,
        .use_llvm = true,
    });
    const pdf_run = b.addRunArtifact(pdf_tests);
    pdf_run.setCwd(b.path("../../test_documents"));
    test_step.dependOn(&pdf_run.step);

    const plugin_api_module = b.createModule(.{
        .root_source_file = b.path("src/plugin_api_test.zig"),
        .target = target,
        .optimize = optimize,
        .link_libc = true,
    });
    plugin_api_module.addImport("kreuzberg", kreuzberg_module);
    const plugin_api_tests = b.addTest(.{
        .name = "plugin_api_test",
        .root_module = plugin_api_module,
        .use_llvm = true,
    });
    const plugin_api_run = b.addRunArtifact(plugin_api_tests);
    plugin_api_run.setCwd(b.path("../../test_documents"));
    test_step.dependOn(&plugin_api_run.step);

    const post_processor_management_module = b.createModule(.{
        .root_source_file = b.path("src/post_processor_management_test.zig"),
        .target = target,
        .optimize = optimize,
        .link_libc = true,
    });
    post_processor_management_module.addImport("kreuzberg", kreuzberg_module);
    const post_processor_management_tests = b.addTest(.{
        .name = "post_processor_management_test",
        .root_module = post_processor_management_module,
        .use_llvm = true,
    });
    const post_processor_management_run = b.addRunArtifact(post_processor_management_tests);
    post_processor_management_run.setCwd(b.path("../../test_documents"));
    test_step.dependOn(&post_processor_management_run.step);

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
    const registry_operations_run = b.addRunArtifact(registry_operations_tests);
    registry_operations_run.setCwd(b.path("../../test_documents"));
    test_step.dependOn(&registry_operations_run.step);

    const renderer_management_module = b.createModule(.{
        .root_source_file = b.path("src/renderer_management_test.zig"),
        .target = target,
        .optimize = optimize,
        .link_libc = true,
    });
    renderer_management_module.addImport("kreuzberg", kreuzberg_module);
    const renderer_management_tests = b.addTest(.{
        .name = "renderer_management_test",
        .root_module = renderer_management_module,
        .use_llvm = true,
    });
    const renderer_management_run = b.addRunArtifact(renderer_management_tests);
    renderer_management_run.setCwd(b.path("../../test_documents"));
    test_step.dependOn(&renderer_management_run.step);

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
    const smoke_run = b.addRunArtifact(smoke_tests);
    smoke_run.setCwd(b.path("../../test_documents"));
    test_step.dependOn(&smoke_run.step);

    const validator_management_module = b.createModule(.{
        .root_source_file = b.path("src/validator_management_test.zig"),
        .target = target,
        .optimize = optimize,
        .link_libc = true,
    });
    validator_management_module.addImport("kreuzberg", kreuzberg_module);
    const validator_management_tests = b.addTest(.{
        .name = "validator_management_test",
        .root_module = validator_management_module,
        .use_llvm = true,
    });
    const validator_management_run = b.addRunArtifact(validator_management_tests);
    validator_management_run.setCwd(b.path("../../test_documents"));
    test_step.dependOn(&validator_management_run.step);

}
