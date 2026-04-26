const std = @import("std");

pub fn build(b: *std.Build) void {
    const target = b.standardTargetOptions(.{});
    const optimize = b.standardOptimizeOption(.{});
    const test_step = b.step("test", "Run tests");

    const archive_tests = b.addTest(.{
        .root_source_file = b.path("src/archive_test.zig"),
        .target = target,
        .optimize = optimize,
    });
    const archive_run = b.addRunArtifact(archive_tests);
    test_step.dependOn(&archive_run.step);

    const async_tests = b.addTest(.{
        .root_source_file = b.path("src/async_test.zig"),
        .target = target,
        .optimize = optimize,
    });
    const async_run = b.addRunArtifact(async_tests);
    test_step.dependOn(&async_run.step);

    const batch_tests = b.addTest(.{
        .root_source_file = b.path("src/batch_test.zig"),
        .target = target,
        .optimize = optimize,
    });
    const batch_run = b.addRunArtifact(batch_tests);
    test_step.dependOn(&batch_run.step);

    const cache_operations_tests = b.addTest(.{
        .root_source_file = b.path("src/cache_operations_test.zig"),
        .target = target,
        .optimize = optimize,
    });
    const cache_operations_run = b.addRunArtifact(cache_operations_tests);
    test_step.dependOn(&cache_operations_run.step);

    const cache_ops_tests = b.addTest(.{
        .root_source_file = b.path("src/cache_ops_test.zig"),
        .target = target,
        .optimize = optimize,
    });
    const cache_ops_run = b.addRunArtifact(cache_ops_tests);
    test_step.dependOn(&cache_ops_run.step);

    const chunking_tests = b.addTest(.{
        .root_source_file = b.path("src/chunking_test.zig"),
        .target = target,
        .optimize = optimize,
    });
    const chunking_run = b.addRunArtifact(chunking_tests);
    test_step.dependOn(&chunking_run.step);

    const configuration_tests = b.addTest(.{
        .root_source_file = b.path("src/configuration_test.zig"),
        .target = target,
        .optimize = optimize,
    });
    const configuration_run = b.addRunArtifact(configuration_tests);
    test_step.dependOn(&configuration_run.step);

    const contract_tests = b.addTest(.{
        .root_source_file = b.path("src/contract_test.zig"),
        .target = target,
        .optimize = optimize,
    });
    const contract_run = b.addRunArtifact(contract_tests);
    test_step.dependOn(&contract_run.step);

    const detection_tests = b.addTest(.{
        .root_source_file = b.path("src/detection_test.zig"),
        .target = target,
        .optimize = optimize,
    });
    const detection_run = b.addRunArtifact(detection_tests);
    test_step.dependOn(&detection_run.step);

    const document_tests = b.addTest(.{
        .root_source_file = b.path("src/document_test.zig"),
        .target = target,
        .optimize = optimize,
    });
    const document_run = b.addRunArtifact(document_tests);
    test_step.dependOn(&document_run.step);

    const document_extractor_management_tests = b.addTest(.{
        .root_source_file = b.path("src/document_extractor_management_test.zig"),
        .target = target,
        .optimize = optimize,
    });
    const document_extractor_management_run = b.addRunArtifact(document_extractor_management_tests);
    test_step.dependOn(&document_extractor_management_run.step);

    const email_tests = b.addTest(.{
        .root_source_file = b.path("src/email_test.zig"),
        .target = target,
        .optimize = optimize,
    });
    const email_run = b.addRunArtifact(email_tests);
    test_step.dependOn(&email_run.step);

    const embed_extra_tests = b.addTest(.{
        .root_source_file = b.path("src/embed_extra_test.zig"),
        .target = target,
        .optimize = optimize,
    });
    const embed_extra_run = b.addRunArtifact(embed_extra_tests);
    test_step.dependOn(&embed_extra_run.step);

    const embedding_backend_management_tests = b.addTest(.{
        .root_source_file = b.path("src/embedding_backend_management_test.zig"),
        .target = target,
        .optimize = optimize,
    });
    const embedding_backend_management_run = b.addRunArtifact(embedding_backend_management_tests);
    test_step.dependOn(&embedding_backend_management_run.step);

    const embeddings_tests = b.addTest(.{
        .root_source_file = b.path("src/embeddings_test.zig"),
        .target = target,
        .optimize = optimize,
    });
    const embeddings_run = b.addRunArtifact(embeddings_tests);
    test_step.dependOn(&embeddings_run.step);

    const error_tests = b.addTest(.{
        .root_source_file = b.path("src/error_test.zig"),
        .target = target,
        .optimize = optimize,
    });
    const error_run = b.addRunArtifact(error_tests);
    test_step.dependOn(&error_run.step);

    const extraction_tests = b.addTest(.{
        .root_source_file = b.path("src/extraction_test.zig"),
        .target = target,
        .optimize = optimize,
    });
    const extraction_run = b.addRunArtifact(extraction_tests);
    test_step.dependOn(&extraction_run.step);

    const format_specific_tests = b.addTest(.{
        .root_source_file = b.path("src/format_specific_test.zig"),
        .target = target,
        .optimize = optimize,
    });
    const format_specific_run = b.addRunArtifact(format_specific_tests);
    test_step.dependOn(&format_specific_run.step);

    const hash_tests = b.addTest(.{
        .root_source_file = b.path("src/hash_test.zig"),
        .target = target,
        .optimize = optimize,
    });
    const hash_run = b.addRunArtifact(hash_tests);
    test_step.dependOn(&hash_run.step);

    const html_tests = b.addTest(.{
        .root_source_file = b.path("src/html_test.zig"),
        .target = target,
        .optimize = optimize,
    });
    const html_run = b.addRunArtifact(html_tests);
    test_step.dependOn(&html_run.step);

    const image_tests = b.addTest(.{
        .root_source_file = b.path("src/image_test.zig"),
        .target = target,
        .optimize = optimize,
    });
    const image_run = b.addRunArtifact(image_tests);
    test_step.dependOn(&image_run.step);

    const image_operations_tests = b.addTest(.{
        .root_source_file = b.path("src/image_operations_test.zig"),
        .target = target,
        .optimize = optimize,
    });
    const image_operations_run = b.addRunArtifact(image_operations_tests);
    test_step.dependOn(&image_operations_run.step);

    const image_ops_tests = b.addTest(.{
        .root_source_file = b.path("src/image_ops_test.zig"),
        .target = target,
        .optimize = optimize,
    });
    const image_ops_run = b.addRunArtifact(image_ops_tests);
    test_step.dependOn(&image_ops_run.step);

    const language_tests = b.addTest(.{
        .root_source_file = b.path("src/language_test.zig"),
        .target = target,
        .optimize = optimize,
    });
    const language_run = b.addRunArtifact(language_tests);
    test_step.dependOn(&language_run.step);

    const markup_tests = b.addTest(.{
        .root_source_file = b.path("src/markup_test.zig"),
        .target = target,
        .optimize = optimize,
    });
    const markup_run = b.addRunArtifact(markup_tests);
    test_step.dependOn(&markup_run.step);

    const mime_utilities_tests = b.addTest(.{
        .root_source_file = b.path("src/mime_utilities_test.zig"),
        .target = target,
        .optimize = optimize,
    });
    const mime_utilities_run = b.addRunArtifact(mime_utilities_tests);
    test_step.dependOn(&mime_utilities_run.step);

    const ocr_backend_management_tests = b.addTest(.{
        .root_source_file = b.path("src/ocr_backend_management_test.zig"),
        .target = target,
        .optimize = optimize,
    });
    const ocr_backend_management_run = b.addRunArtifact(ocr_backend_management_tests);
    test_step.dependOn(&ocr_backend_management_run.step);

    const office_tests = b.addTest(.{
        .root_source_file = b.path("src/office_test.zig"),
        .target = target,
        .optimize = optimize,
    });
    const office_run = b.addRunArtifact(office_tests);
    test_step.dependOn(&office_run.step);

    const parsing_tests = b.addTest(.{
        .root_source_file = b.path("src/parsing_test.zig"),
        .target = target,
        .optimize = optimize,
    });
    const parsing_run = b.addRunArtifact(parsing_tests);
    test_step.dependOn(&parsing_run.step);

    const pdf_tests = b.addTest(.{
        .root_source_file = b.path("src/pdf_test.zig"),
        .target = target,
        .optimize = optimize,
    });
    const pdf_run = b.addRunArtifact(pdf_tests);
    test_step.dependOn(&pdf_run.step);

    const post_processor_management_tests = b.addTest(.{
        .root_source_file = b.path("src/post_processor_management_test.zig"),
        .target = target,
        .optimize = optimize,
    });
    const post_processor_management_run = b.addRunArtifact(post_processor_management_tests);
    test_step.dependOn(&post_processor_management_run.step);

    const registry_tests = b.addTest(.{
        .root_source_file = b.path("src/registry_test.zig"),
        .target = target,
        .optimize = optimize,
    });
    const registry_run = b.addRunArtifact(registry_tests);
    test_step.dependOn(&registry_run.step);

    const registry_operations_tests = b.addTest(.{
        .root_source_file = b.path("src/registry_operations_test.zig"),
        .target = target,
        .optimize = optimize,
    });
    const registry_operations_run = b.addRunArtifact(registry_operations_tests);
    test_step.dependOn(&registry_operations_run.step);

    const render_tests = b.addTest(.{
        .root_source_file = b.path("src/render_test.zig"),
        .target = target,
        .optimize = optimize,
    });
    const render_run = b.addRunArtifact(render_tests);
    test_step.dependOn(&render_run.step);

    const rendering_tests = b.addTest(.{
        .root_source_file = b.path("src/rendering_test.zig"),
        .target = target,
        .optimize = optimize,
    });
    const rendering_run = b.addRunArtifact(rendering_tests);
    test_step.dependOn(&rendering_run.step);

    const serialization_tests = b.addTest(.{
        .root_source_file = b.path("src/serialization_test.zig"),
        .target = target,
        .optimize = optimize,
    });
    const serialization_run = b.addRunArtifact(serialization_tests);
    test_step.dependOn(&serialization_run.step);

    const smoke_tests = b.addTest(.{
        .root_source_file = b.path("src/smoke_test.zig"),
        .target = target,
        .optimize = optimize,
    });
    const smoke_run = b.addRunArtifact(smoke_tests);
    test_step.dependOn(&smoke_run.step);

    const string_utils_tests = b.addTest(.{
        .root_source_file = b.path("src/string_utils_test.zig"),
        .target = target,
        .optimize = optimize,
    });
    const string_utils_run = b.addRunArtifact(string_utils_tests);
    test_step.dependOn(&string_utils_run.step);

    const structured_tests = b.addTest(.{
        .root_source_file = b.path("src/structured_test.zig"),
        .target = target,
        .optimize = optimize,
    });
    const structured_run = b.addRunArtifact(structured_tests);
    test_step.dependOn(&structured_run.step);

    const table_tests = b.addTest(.{
        .root_source_file = b.path("src/table_test.zig"),
        .target = target,
        .optimize = optimize,
    });
    const table_run = b.addRunArtifact(table_tests);
    test_step.dependOn(&table_run.step);

    const table_operations_tests = b.addTest(.{
        .root_source_file = b.path("src/table_operations_test.zig"),
        .target = target,
        .optimize = optimize,
    });
    const table_operations_run = b.addRunArtifact(table_operations_tests);
    test_step.dependOn(&table_operations_run.step);

    const table_ops_tests = b.addTest(.{
        .root_source_file = b.path("src/table_ops_test.zig"),
        .target = target,
        .optimize = optimize,
    });
    const table_ops_run = b.addRunArtifact(table_ops_tests);
    test_step.dependOn(&table_ops_run.step);

    const text_processing_tests = b.addTest(.{
        .root_source_file = b.path("src/text_processing_test.zig"),
        .target = target,
        .optimize = optimize,
    });
    const text_processing_run = b.addRunArtifact(text_processing_tests);
    test_step.dependOn(&text_processing_run.step);

    const text_utils_tests = b.addTest(.{
        .root_source_file = b.path("src/text_utils_test.zig"),
        .target = target,
        .optimize = optimize,
    });
    const text_utils_run = b.addRunArtifact(text_utils_tests);
    test_step.dependOn(&text_utils_run.step);

    const token_reduction_tests = b.addTest(.{
        .root_source_file = b.path("src/token_reduction_test.zig"),
        .target = target,
        .optimize = optimize,
    });
    const token_reduction_run = b.addRunArtifact(token_reduction_tests);
    test_step.dependOn(&token_reduction_run.step);

    const uri_tests = b.addTest(.{
        .root_source_file = b.path("src/uri_test.zig"),
        .target = target,
        .optimize = optimize,
    });
    const uri_run = b.addRunArtifact(uri_tests);
    test_step.dependOn(&uri_run.step);

    const validate_tests = b.addTest(.{
        .root_source_file = b.path("src/validate_test.zig"),
        .target = target,
        .optimize = optimize,
    });
    const validate_run = b.addRunArtifact(validate_tests);
    test_step.dependOn(&validate_run.step);

    const validation_tests = b.addTest(.{
        .root_source_file = b.path("src/validation_test.zig"),
        .target = target,
        .optimize = optimize,
    });
    const validation_run = b.addRunArtifact(validation_tests);
    test_step.dependOn(&validation_run.step);

    const validator_management_tests = b.addTest(.{
        .root_source_file = b.path("src/validator_management_test.zig"),
        .target = target,
        .optimize = optimize,
    });
    const validator_management_run = b.addRunArtifact(validator_management_tests);
    test_step.dependOn(&validator_management_run.step);

    const xml_tests = b.addTest(.{
        .root_source_file = b.path("src/xml_test.zig"),
        .target = target,
        .optimize = optimize,
    });
    const xml_run = b.addRunArtifact(xml_tests);
    test_step.dependOn(&xml_run.step);

}
