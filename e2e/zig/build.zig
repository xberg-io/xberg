const std = @import("std");
const builtin = @import("builtin");

pub fn build(b: *std.Build) void {
    const target = b.standardTargetOptions(.{});
    const optimize = b.standardOptimizeOption(.{});
    const test_step = b.step("test", "Run tests");
    const ffi_path = b.option([]const u8, "ffi_path", "Path to directory containing libkreuzberg_ffi") orelse "../../target/release";
    const ffi_include = b.option([]const u8, "ffi_include_path", "Path to directory containing FFI header") orelse "../../crates/kreuzberg-ffi/include";
    const ffi_path_abs = b.pathFromRoot(ffi_path);

    const kreuzberg_module = b.addModule("kreuzberg", .{
        .root_source_file = b.path("../../packages/zig/src/kreuzberg.zig"),
        .target = target,
        .optimize = optimize,
        .link_libc = true,
    });
    kreuzberg_module.addLibraryPath(.{ .cwd_relative = ffi_path });
    kreuzberg_module.addIncludePath(.{ .cwd_relative = ffi_include });
    kreuzberg_module.linkSystemLibrary("kreuzberg_ffi", .{});
    kreuzberg_module.linkSystemLibrary("heif", .{});
    kreuzberg_module.addRPath(.{ .cwd_relative = ffi_path_abs });

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
    async_tests.root_module.addRPath(.{ .cwd_relative = ffi_path_abs });
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
    batch_tests.root_module.addRPath(.{ .cwd_relative = ffi_path_abs });
    const batch_run = b.addRunArtifact(batch_tests);
    batch_run.setCwd(b.path("../../test_documents"));
    batch_run.step.dependOn(&async_run.step);
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
    contract_tests.root_module.addRPath(.{ .cwd_relative = ffi_path_abs });
    const contract_run = b.addRunArtifact(contract_tests);
    contract_run.setCwd(b.path("../../test_documents"));
    contract_run.step.dependOn(&batch_run.step);
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
    detection_tests.root_module.addRPath(.{ .cwd_relative = ffi_path_abs });
    const detection_run = b.addRunArtifact(detection_tests);
    detection_run.setCwd(b.path("../../test_documents"));
    detection_run.step.dependOn(&contract_run.step);
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
    document_extractor_management_tests.root_module.addRPath(.{ .cwd_relative = ffi_path_abs });
    const document_extractor_management_run = b.addRunArtifact(document_extractor_management_tests);
    document_extractor_management_run.setCwd(b.path("../../test_documents"));
    document_extractor_management_run.step.dependOn(&detection_run.step);
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
    embed_async_pending_tests.root_module.addRPath(.{ .cwd_relative = ffi_path_abs });
    const embed_async_pending_run = b.addRunArtifact(embed_async_pending_tests);
    embed_async_pending_run.setCwd(b.path("../../test_documents"));
    embed_async_pending_run.step.dependOn(&document_extractor_management_run.step);
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
    embed_extra_tests.root_module.addRPath(.{ .cwd_relative = ffi_path_abs });
    const embed_extra_run = b.addRunArtifact(embed_extra_tests);
    embed_extra_run.setCwd(b.path("../../test_documents"));
    embed_extra_run.step.dependOn(&embed_async_pending_run.step);
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
    embedding_backend_management_tests.root_module.addRPath(.{ .cwd_relative = ffi_path_abs });
    const embedding_backend_management_run = b.addRunArtifact(embedding_backend_management_tests);
    embedding_backend_management_run.setCwd(b.path("../../test_documents"));
    embedding_backend_management_run.step.dependOn(&embed_extra_run.step);
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
    embeddings_tests.root_module.addRPath(.{ .cwd_relative = ffi_path_abs });
    const embeddings_run = b.addRunArtifact(embeddings_tests);
    embeddings_run.setCwd(b.path("../../test_documents"));
    embeddings_run.step.dependOn(&embedding_backend_management_run.step);
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
    error_tests.root_module.addRPath(.{ .cwd_relative = ffi_path_abs });
    const error_run = b.addRunArtifact(error_tests);
    error_run.setCwd(b.path("../../test_documents"));
    error_run.step.dependOn(&embeddings_run.step);
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
    format_specific_tests.root_module.addRPath(.{ .cwd_relative = ffi_path_abs });
    const format_specific_run = b.addRunArtifact(format_specific_tests);
    format_specific_run.setCwd(b.path("../../test_documents"));
    format_specific_run.step.dependOn(&error_run.step);
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
    mime_utilities_tests.root_module.addRPath(.{ .cwd_relative = ffi_path_abs });
    const mime_utilities_run = b.addRunArtifact(mime_utilities_tests);
    mime_utilities_run.setCwd(b.path("../../test_documents"));
    mime_utilities_run.step.dependOn(&format_specific_run.step);
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
    ocr_backend_management_tests.root_module.addRPath(.{ .cwd_relative = ffi_path_abs });
    const ocr_backend_management_run = b.addRunArtifact(ocr_backend_management_tests);
    ocr_backend_management_run.setCwd(b.path("../../test_documents"));
    ocr_backend_management_run.step.dependOn(&mime_utilities_run.step);
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
    pdf_tests.root_module.addRPath(.{ .cwd_relative = ffi_path_abs });
    const pdf_run = b.addRunArtifact(pdf_tests);
    pdf_run.setCwd(b.path("../../test_documents"));
    pdf_run.step.dependOn(&ocr_backend_management_run.step);
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
    plugin_api_tests.root_module.addRPath(.{ .cwd_relative = ffi_path_abs });
    const plugin_api_run = b.addRunArtifact(plugin_api_tests);
    plugin_api_run.setCwd(b.path("../../test_documents"));
    plugin_api_run.step.dependOn(&pdf_run.step);
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
    post_processor_management_tests.root_module.addRPath(.{ .cwd_relative = ffi_path_abs });
    const post_processor_management_run = b.addRunArtifact(post_processor_management_tests);
    post_processor_management_run.setCwd(b.path("../../test_documents"));
    post_processor_management_run.step.dependOn(&plugin_api_run.step);
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
    registry_tests.root_module.addRPath(.{ .cwd_relative = ffi_path_abs });
    const registry_run = b.addRunArtifact(registry_tests);
    registry_run.setCwd(b.path("../../test_documents"));
    registry_run.step.dependOn(&post_processor_management_run.step);
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
    registry_operations_tests.root_module.addRPath(.{ .cwd_relative = ffi_path_abs });
    const registry_operations_run = b.addRunArtifact(registry_operations_tests);
    registry_operations_run.setCwd(b.path("../../test_documents"));
    registry_operations_run.step.dependOn(&registry_run.step);
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
    renderer_management_tests.root_module.addRPath(.{ .cwd_relative = ffi_path_abs });
    const renderer_management_run = b.addRunArtifact(renderer_management_tests);
    renderer_management_run.setCwd(b.path("../../test_documents"));
    renderer_management_run.step.dependOn(&registry_operations_run.step);
    test_step.dependOn(&renderer_management_run.step);

    const rerank_module = b.createModule(.{
        .root_source_file = b.path("src/rerank_test.zig"),
        .target = target,
        .optimize = optimize,
        .link_libc = true,
    });
    rerank_module.addImport("kreuzberg", kreuzberg_module);
    const rerank_tests = b.addTest(.{
        .name = "rerank_test",
        .root_module = rerank_module,
        .use_llvm = true,
    });
    rerank_tests.root_module.addRPath(.{ .cwd_relative = ffi_path_abs });
    const rerank_run = b.addRunArtifact(rerank_tests);
    rerank_run.setCwd(b.path("../../test_documents"));
    rerank_run.step.dependOn(&renderer_management_run.step);
    test_step.dependOn(&rerank_run.step);

    const rerank_async_pending_module = b.createModule(.{
        .root_source_file = b.path("src/rerank_async_pending_test.zig"),
        .target = target,
        .optimize = optimize,
        .link_libc = true,
    });
    rerank_async_pending_module.addImport("kreuzberg", kreuzberg_module);
    const rerank_async_pending_tests = b.addTest(.{
        .name = "rerank_async_pending_test",
        .root_module = rerank_async_pending_module,
        .use_llvm = true,
    });
    rerank_async_pending_tests.root_module.addRPath(.{ .cwd_relative = ffi_path_abs });
    const rerank_async_pending_run = b.addRunArtifact(rerank_async_pending_tests);
    rerank_async_pending_run.setCwd(b.path("../../test_documents"));
    rerank_async_pending_run.step.dependOn(&rerank_run.step);
    test_step.dependOn(&rerank_async_pending_run.step);

    const reranker_backend_management_module = b.createModule(.{
        .root_source_file = b.path("src/reranker_backend_management_test.zig"),
        .target = target,
        .optimize = optimize,
        .link_libc = true,
    });
    reranker_backend_management_module.addImport("kreuzberg", kreuzberg_module);
    const reranker_backend_management_tests = b.addTest(.{
        .name = "reranker_backend_management_test",
        .root_module = reranker_backend_management_module,
        .use_llvm = true,
    });
    reranker_backend_management_tests.root_module.addRPath(.{ .cwd_relative = ffi_path_abs });
    const reranker_backend_management_run = b.addRunArtifact(reranker_backend_management_tests);
    reranker_backend_management_run.setCwd(b.path("../../test_documents"));
    reranker_backend_management_run.step.dependOn(&rerank_async_pending_run.step);
    test_step.dependOn(&reranker_backend_management_run.step);

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
    smoke_tests.root_module.addRPath(.{ .cwd_relative = ffi_path_abs });
    const smoke_run = b.addRunArtifact(smoke_tests);
    smoke_run.setCwd(b.path("../../test_documents"));
    smoke_run.step.dependOn(&reranker_backend_management_run.step);
    test_step.dependOn(&smoke_run.step);

    const summarization_module = b.createModule(.{
        .root_source_file = b.path("src/summarization_test.zig"),
        .target = target,
        .optimize = optimize,
        .link_libc = true,
    });
    summarization_module.addImport("kreuzberg", kreuzberg_module);
    const summarization_tests = b.addTest(.{
        .name = "summarization_test",
        .root_module = summarization_module,
        .use_llvm = true,
    });
    summarization_tests.root_module.addRPath(.{ .cwd_relative = ffi_path_abs });
    const summarization_run = b.addRunArtifact(summarization_tests);
    summarization_run.setCwd(b.path("../../test_documents"));
    summarization_run.step.dependOn(&smoke_run.step);
    test_step.dependOn(&summarization_run.step);

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
    validator_management_tests.root_module.addRPath(.{ .cwd_relative = ffi_path_abs });
    const validator_management_run = b.addRunArtifact(validator_management_tests);
    validator_management_run.setCwd(b.path("../../test_documents"));
    validator_management_run.step.dependOn(&summarization_run.step);
    test_step.dependOn(&validator_management_run.step);

}
