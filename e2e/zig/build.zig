const std = @import("std");
const builtin = @import("builtin");

pub fn build(b: *std.Build) void {
    const target = b.standardTargetOptions(.{});
    const optimize = b.standardOptimizeOption(.{});
    const test_step = b.step("test", "Run tests");
    const ffi_path = b.option([]const u8, "ffi_path", "Path to directory containing libxberg_ffi") orelse "../../target/release";
    const ffi_include = b.option([]const u8, "ffi_include_path", "Path to directory containing FFI header") orelse "../../crates/xberg-ffi/include";
    const ffi_path_abs = b.pathFromRoot(ffi_path);

    const xberg_module = b.addModule("xberg", .{
        .root_source_file = b.path("../../packages/zig/src/xberg.zig"),
        .target = target,
        .optimize = optimize,
        .link_libc = true,
    });
    xberg_module.addLibraryPath(.{ .cwd_relative = ffi_path });
    xberg_module.addIncludePath(.{ .cwd_relative = ffi_include });
    xberg_module.linkSystemLibrary("xberg_ffi", .{});
    xberg_module.linkSystemLibrary("heif", .{});
    xberg_module.addRPath(.{ .cwd_relative = ffi_path_abs });

    const _alloc = b.allocator;
    var mock_server_url: ?[]const u8 = b.graph.environ_map.get("MOCK_SERVER_URL");
    var mock_servers_json: ?[]const u8 = null;
    var mock_servers_map = std.StringHashMap([]const u8).init(_alloc);
    if (mock_server_url == null) {
        const _bin = b.pathFromRoot("../rust/target/release/mock-server");
        const _fixtures = b.pathFromRoot("../../fixtures");
        var _threaded = std.Io.Threaded.init(_alloc, .{});
        const _io = _threaded.io();
        const _spawned = std.process.spawn(_io, .{
            .argv = &.{ _bin, _fixtures },
            .stdin = .pipe,
            .stdout = .pipe,
            .stderr = .inherit,
        });
        if (_spawned) |_child| {
            // The child is intentionally not awaited: it lives for the duration
            // of the `zig build` process, which spans test execution.
            const _stdout = _child.stdout.?;
            var _buf: [65536]u8 = undefined;
            var _file_reader = _stdout.readerStreaming(_io, &_buf);
            const _r = &_file_reader.interface;
            // Read startup lines: MOCK_SERVER_URL= then MOCK_SERVERS= (always
            // emitted, possibly `{}`). Cap the loop so a misbehaving server
            // cannot block the build indefinitely.
            var _saw_url = false;
            var _i: usize = 0;
            while (_i < 64) : (_i += 1) {
                const _line_raw = _r.takeDelimiterExclusive('\n') catch break;
                const _line = std.mem.trim(u8, _line_raw, " \r\t");
                if (std.mem.startsWith(u8, _line, "MOCK_SERVER_URL=")) {
                    mock_server_url = _alloc.dupe(u8, _line["MOCK_SERVER_URL=".len..]) catch null;
                    _saw_url = true;
                } else if (std.mem.startsWith(u8, _line, "MOCK_SERVERS=")) {
                    const _json = _line["MOCK_SERVERS=".len..];
                    mock_servers_json = _alloc.dupe(u8, _json) catch null;
                    if (std.json.parseFromSlice(std.json.Value, _alloc, _json, .{})) |_parsed| {
                        if (_parsed.value == .object) {
                            var _entries = _parsed.value.object.iterator();
                            while (_entries.next()) |_entry| {
                                if (_entry.value_ptr.* == .string) {
                                    const _key = std.fmt.allocPrint(_alloc, "MOCK_SERVER_{s}", .{_entry.key_ptr.*}) catch continue;
                                    for (_key) |*_c| _c.* = std.ascii.toUpper(_c.*);
                                    const _val = _alloc.dupe(u8, _entry.value_ptr.*.string) catch continue;
                                    mock_servers_map.put(_key, _val) catch {};
                                }
                            }
                        }
                    } else |_| {}
                    break;
                } else if (_saw_url) {
                    break;
                }
            }
        } else |_| {
            // Binary not built — leave mock_server_url null so tests surface a
            // clear connection error rather than a build failure.
        }
    }

    const batch_module = b.createModule(.{
        .root_source_file = b.path("src/batch_test.zig"),
        .target = target,
        .optimize = optimize,
        .link_libc = true,
    });
    batch_module.addImport("xberg", xberg_module);
    const batch_tests = b.addTest(.{
        .name = "batch_test",
        .root_module = batch_module,
        .use_llvm = true,
    });
    batch_tests.root_module.addRPath(.{ .cwd_relative = ffi_path_abs });
    const batch_run = b.addRunArtifact(batch_tests);
    batch_run.setCwd(b.path("../../test_documents"));
    batch_run.setEnvironmentVariable("CRAWLBERG_ALLOW_PRIVATE_NETWORK", "true");
    if (mock_server_url) |_url| {
        batch_run.setEnvironmentVariable("MOCK_SERVER_URL", _url);
    }
    if (mock_servers_json) |_json| {
        batch_run.setEnvironmentVariable("MOCK_SERVERS", _json);
    }
    {
        var _it = mock_servers_map.iterator();
        while (_it.next()) |_entry| {
            batch_run.setEnvironmentVariable(_entry.key_ptr.*, _entry.value_ptr.*);
        }
    }
    test_step.dependOn(&batch_run.step);

    const contract_module = b.createModule(.{
        .root_source_file = b.path("src/contract_test.zig"),
        .target = target,
        .optimize = optimize,
        .link_libc = true,
    });
    contract_module.addImport("xberg", xberg_module);
    const contract_tests = b.addTest(.{
        .name = "contract_test",
        .root_module = contract_module,
        .use_llvm = true,
    });
    contract_tests.root_module.addRPath(.{ .cwd_relative = ffi_path_abs });
    const contract_run = b.addRunArtifact(contract_tests);
    contract_run.setCwd(b.path("../../test_documents"));
    contract_run.setEnvironmentVariable("CRAWLBERG_ALLOW_PRIVATE_NETWORK", "true");
    if (mock_server_url) |_url| {
        contract_run.setEnvironmentVariable("MOCK_SERVER_URL", _url);
    }
    if (mock_servers_json) |_json| {
        contract_run.setEnvironmentVariable("MOCK_SERVERS", _json);
    }
    {
        var _it = mock_servers_map.iterator();
        while (_it.next()) |_entry| {
            contract_run.setEnvironmentVariable(_entry.key_ptr.*, _entry.value_ptr.*);
        }
    }
    contract_run.step.dependOn(&batch_run.step);
    test_step.dependOn(&contract_run.step);

    const embedding_backend_management_module = b.createModule(.{
        .root_source_file = b.path("src/embedding_backend_management_test.zig"),
        .target = target,
        .optimize = optimize,
        .link_libc = true,
    });
    embedding_backend_management_module.addImport("xberg", xberg_module);
    const embedding_backend_management_tests = b.addTest(.{
        .name = "embedding_backend_management_test",
        .root_module = embedding_backend_management_module,
        .use_llvm = true,
    });
    embedding_backend_management_tests.root_module.addRPath(.{ .cwd_relative = ffi_path_abs });
    const embedding_backend_management_run = b.addRunArtifact(embedding_backend_management_tests);
    embedding_backend_management_run.setCwd(b.path("../../test_documents"));
    embedding_backend_management_run.setEnvironmentVariable("CRAWLBERG_ALLOW_PRIVATE_NETWORK", "true");
    if (mock_server_url) |_url| {
        embedding_backend_management_run.setEnvironmentVariable("MOCK_SERVER_URL", _url);
    }
    if (mock_servers_json) |_json| {
        embedding_backend_management_run.setEnvironmentVariable("MOCK_SERVERS", _json);
    }
    {
        var _it = mock_servers_map.iterator();
        while (_it.next()) |_entry| {
            embedding_backend_management_run.setEnvironmentVariable(_entry.key_ptr.*, _entry.value_ptr.*);
        }
    }
    embedding_backend_management_run.step.dependOn(&contract_run.step);
    test_step.dependOn(&embedding_backend_management_run.step);

    const error_module = b.createModule(.{
        .root_source_file = b.path("src/error_test.zig"),
        .target = target,
        .optimize = optimize,
        .link_libc = true,
    });
    error_module.addImport("xberg", xberg_module);
    const error_tests = b.addTest(.{
        .name = "error_test",
        .root_module = error_module,
        .use_llvm = true,
    });
    error_tests.root_module.addRPath(.{ .cwd_relative = ffi_path_abs });
    const error_run = b.addRunArtifact(error_tests);
    error_run.setCwd(b.path("../../test_documents"));
    error_run.setEnvironmentVariable("CRAWLBERG_ALLOW_PRIVATE_NETWORK", "true");
    if (mock_server_url) |_url| {
        error_run.setEnvironmentVariable("MOCK_SERVER_URL", _url);
    }
    if (mock_servers_json) |_json| {
        error_run.setEnvironmentVariable("MOCK_SERVERS", _json);
    }
    {
        var _it = mock_servers_map.iterator();
        while (_it.next()) |_entry| {
            error_run.setEnvironmentVariable(_entry.key_ptr.*, _entry.value_ptr.*);
        }
    }
    error_run.step.dependOn(&embedding_backend_management_run.step);
    test_step.dependOn(&error_run.step);

    const extract_module = b.createModule(.{
        .root_source_file = b.path("src/extract_test.zig"),
        .target = target,
        .optimize = optimize,
        .link_libc = true,
    });
    extract_module.addImport("xberg", xberg_module);
    const extract_tests = b.addTest(.{
        .name = "extract_test",
        .root_module = extract_module,
        .use_llvm = true,
    });
    extract_tests.root_module.addRPath(.{ .cwd_relative = ffi_path_abs });
    const extract_run = b.addRunArtifact(extract_tests);
    extract_run.setCwd(b.path("../../test_documents"));
    extract_run.setEnvironmentVariable("CRAWLBERG_ALLOW_PRIVATE_NETWORK", "true");
    if (mock_server_url) |_url| {
        extract_run.setEnvironmentVariable("MOCK_SERVER_URL", _url);
    }
    if (mock_servers_json) |_json| {
        extract_run.setEnvironmentVariable("MOCK_SERVERS", _json);
    }
    {
        var _it = mock_servers_map.iterator();
        while (_it.next()) |_entry| {
            extract_run.setEnvironmentVariable(_entry.key_ptr.*, _entry.value_ptr.*);
        }
    }
    extract_run.step.dependOn(&error_run.step);
    test_step.dependOn(&extract_run.step);

    const format_specific_module = b.createModule(.{
        .root_source_file = b.path("src/format_specific_test.zig"),
        .target = target,
        .optimize = optimize,
        .link_libc = true,
    });
    format_specific_module.addImport("xberg", xberg_module);
    const format_specific_tests = b.addTest(.{
        .name = "format_specific_test",
        .root_module = format_specific_module,
        .use_llvm = true,
    });
    format_specific_tests.root_module.addRPath(.{ .cwd_relative = ffi_path_abs });
    const format_specific_run = b.addRunArtifact(format_specific_tests);
    format_specific_run.setCwd(b.path("../../test_documents"));
    format_specific_run.setEnvironmentVariable("CRAWLBERG_ALLOW_PRIVATE_NETWORK", "true");
    if (mock_server_url) |_url| {
        format_specific_run.setEnvironmentVariable("MOCK_SERVER_URL", _url);
    }
    if (mock_servers_json) |_json| {
        format_specific_run.setEnvironmentVariable("MOCK_SERVERS", _json);
    }
    {
        var _it = mock_servers_map.iterator();
        while (_it.next()) |_entry| {
            format_specific_run.setEnvironmentVariable(_entry.key_ptr.*, _entry.value_ptr.*);
        }
    }
    format_specific_run.step.dependOn(&extract_run.step);
    test_step.dependOn(&format_specific_run.step);

    const ocr_backend_management_module = b.createModule(.{
        .root_source_file = b.path("src/ocr_backend_management_test.zig"),
        .target = target,
        .optimize = optimize,
        .link_libc = true,
    });
    ocr_backend_management_module.addImport("xberg", xberg_module);
    const ocr_backend_management_tests = b.addTest(.{
        .name = "ocr_backend_management_test",
        .root_module = ocr_backend_management_module,
        .use_llvm = true,
    });
    ocr_backend_management_tests.root_module.addRPath(.{ .cwd_relative = ffi_path_abs });
    const ocr_backend_management_run = b.addRunArtifact(ocr_backend_management_tests);
    ocr_backend_management_run.setCwd(b.path("../../test_documents"));
    ocr_backend_management_run.setEnvironmentVariable("CRAWLBERG_ALLOW_PRIVATE_NETWORK", "true");
    if (mock_server_url) |_url| {
        ocr_backend_management_run.setEnvironmentVariable("MOCK_SERVER_URL", _url);
    }
    if (mock_servers_json) |_json| {
        ocr_backend_management_run.setEnvironmentVariable("MOCK_SERVERS", _json);
    }
    {
        var _it = mock_servers_map.iterator();
        while (_it.next()) |_entry| {
            ocr_backend_management_run.setEnvironmentVariable(_entry.key_ptr.*, _entry.value_ptr.*);
        }
    }
    ocr_backend_management_run.step.dependOn(&format_specific_run.step);
    test_step.dependOn(&ocr_backend_management_run.step);

    const plugin_api_module = b.createModule(.{
        .root_source_file = b.path("src/plugin_api_test.zig"),
        .target = target,
        .optimize = optimize,
        .link_libc = true,
    });
    plugin_api_module.addImport("xberg", xberg_module);
    const plugin_api_tests = b.addTest(.{
        .name = "plugin_api_test",
        .root_module = plugin_api_module,
        .use_llvm = true,
    });
    plugin_api_tests.root_module.addRPath(.{ .cwd_relative = ffi_path_abs });
    const plugin_api_run = b.addRunArtifact(plugin_api_tests);
    plugin_api_run.setCwd(b.path("../../test_documents"));
    plugin_api_run.setEnvironmentVariable("CRAWLBERG_ALLOW_PRIVATE_NETWORK", "true");
    if (mock_server_url) |_url| {
        plugin_api_run.setEnvironmentVariable("MOCK_SERVER_URL", _url);
    }
    if (mock_servers_json) |_json| {
        plugin_api_run.setEnvironmentVariable("MOCK_SERVERS", _json);
    }
    {
        var _it = mock_servers_map.iterator();
        while (_it.next()) |_entry| {
            plugin_api_run.setEnvironmentVariable(_entry.key_ptr.*, _entry.value_ptr.*);
        }
    }
    plugin_api_run.step.dependOn(&ocr_backend_management_run.step);
    test_step.dependOn(&plugin_api_run.step);

    const post_processor_management_module = b.createModule(.{
        .root_source_file = b.path("src/post_processor_management_test.zig"),
        .target = target,
        .optimize = optimize,
        .link_libc = true,
    });
    post_processor_management_module.addImport("xberg", xberg_module);
    const post_processor_management_tests = b.addTest(.{
        .name = "post_processor_management_test",
        .root_module = post_processor_management_module,
        .use_llvm = true,
    });
    post_processor_management_tests.root_module.addRPath(.{ .cwd_relative = ffi_path_abs });
    const post_processor_management_run = b.addRunArtifact(post_processor_management_tests);
    post_processor_management_run.setCwd(b.path("../../test_documents"));
    post_processor_management_run.setEnvironmentVariable("CRAWLBERG_ALLOW_PRIVATE_NETWORK", "true");
    if (mock_server_url) |_url| {
        post_processor_management_run.setEnvironmentVariable("MOCK_SERVER_URL", _url);
    }
    if (mock_servers_json) |_json| {
        post_processor_management_run.setEnvironmentVariable("MOCK_SERVERS", _json);
    }
    {
        var _it = mock_servers_map.iterator();
        while (_it.next()) |_entry| {
            post_processor_management_run.setEnvironmentVariable(_entry.key_ptr.*, _entry.value_ptr.*);
        }
    }
    post_processor_management_run.step.dependOn(&plugin_api_run.step);
    test_step.dependOn(&post_processor_management_run.step);

    const registry_module = b.createModule(.{
        .root_source_file = b.path("src/registry_test.zig"),
        .target = target,
        .optimize = optimize,
        .link_libc = true,
    });
    registry_module.addImport("xberg", xberg_module);
    const registry_tests = b.addTest(.{
        .name = "registry_test",
        .root_module = registry_module,
        .use_llvm = true,
    });
    registry_tests.root_module.addRPath(.{ .cwd_relative = ffi_path_abs });
    const registry_run = b.addRunArtifact(registry_tests);
    registry_run.setCwd(b.path("../../test_documents"));
    registry_run.setEnvironmentVariable("CRAWLBERG_ALLOW_PRIVATE_NETWORK", "true");
    if (mock_server_url) |_url| {
        registry_run.setEnvironmentVariable("MOCK_SERVER_URL", _url);
    }
    if (mock_servers_json) |_json| {
        registry_run.setEnvironmentVariable("MOCK_SERVERS", _json);
    }
    {
        var _it = mock_servers_map.iterator();
        while (_it.next()) |_entry| {
            registry_run.setEnvironmentVariable(_entry.key_ptr.*, _entry.value_ptr.*);
        }
    }
    registry_run.step.dependOn(&post_processor_management_run.step);
    test_step.dependOn(&registry_run.step);

    const renderer_management_module = b.createModule(.{
        .root_source_file = b.path("src/renderer_management_test.zig"),
        .target = target,
        .optimize = optimize,
        .link_libc = true,
    });
    renderer_management_module.addImport("xberg", xberg_module);
    const renderer_management_tests = b.addTest(.{
        .name = "renderer_management_test",
        .root_module = renderer_management_module,
        .use_llvm = true,
    });
    renderer_management_tests.root_module.addRPath(.{ .cwd_relative = ffi_path_abs });
    const renderer_management_run = b.addRunArtifact(renderer_management_tests);
    renderer_management_run.setCwd(b.path("../../test_documents"));
    renderer_management_run.setEnvironmentVariable("CRAWLBERG_ALLOW_PRIVATE_NETWORK", "true");
    if (mock_server_url) |_url| {
        renderer_management_run.setEnvironmentVariable("MOCK_SERVER_URL", _url);
    }
    if (mock_servers_json) |_json| {
        renderer_management_run.setEnvironmentVariable("MOCK_SERVERS", _json);
    }
    {
        var _it = mock_servers_map.iterator();
        while (_it.next()) |_entry| {
            renderer_management_run.setEnvironmentVariable(_entry.key_ptr.*, _entry.value_ptr.*);
        }
    }
    renderer_management_run.step.dependOn(&registry_run.step);
    test_step.dependOn(&renderer_management_run.step);

    const reranker_backend_management_module = b.createModule(.{
        .root_source_file = b.path("src/reranker_backend_management_test.zig"),
        .target = target,
        .optimize = optimize,
        .link_libc = true,
    });
    reranker_backend_management_module.addImport("xberg", xberg_module);
    const reranker_backend_management_tests = b.addTest(.{
        .name = "reranker_backend_management_test",
        .root_module = reranker_backend_management_module,
        .use_llvm = true,
    });
    reranker_backend_management_tests.root_module.addRPath(.{ .cwd_relative = ffi_path_abs });
    const reranker_backend_management_run = b.addRunArtifact(reranker_backend_management_tests);
    reranker_backend_management_run.setCwd(b.path("../../test_documents"));
    reranker_backend_management_run.setEnvironmentVariable("CRAWLBERG_ALLOW_PRIVATE_NETWORK", "true");
    if (mock_server_url) |_url| {
        reranker_backend_management_run.setEnvironmentVariable("MOCK_SERVER_URL", _url);
    }
    if (mock_servers_json) |_json| {
        reranker_backend_management_run.setEnvironmentVariable("MOCK_SERVERS", _json);
    }
    {
        var _it = mock_servers_map.iterator();
        while (_it.next()) |_entry| {
            reranker_backend_management_run.setEnvironmentVariable(_entry.key_ptr.*, _entry.value_ptr.*);
        }
    }
    reranker_backend_management_run.step.dependOn(&renderer_management_run.step);
    test_step.dependOn(&reranker_backend_management_run.step);

    const smoke_module = b.createModule(.{
        .root_source_file = b.path("src/smoke_test.zig"),
        .target = target,
        .optimize = optimize,
        .link_libc = true,
    });
    smoke_module.addImport("xberg", xberg_module);
    const smoke_tests = b.addTest(.{
        .name = "smoke_test",
        .root_module = smoke_module,
        .use_llvm = true,
    });
    smoke_tests.root_module.addRPath(.{ .cwd_relative = ffi_path_abs });
    const smoke_run = b.addRunArtifact(smoke_tests);
    smoke_run.setCwd(b.path("../../test_documents"));
    smoke_run.setEnvironmentVariable("CRAWLBERG_ALLOW_PRIVATE_NETWORK", "true");
    if (mock_server_url) |_url| {
        smoke_run.setEnvironmentVariable("MOCK_SERVER_URL", _url);
    }
    if (mock_servers_json) |_json| {
        smoke_run.setEnvironmentVariable("MOCK_SERVERS", _json);
    }
    {
        var _it = mock_servers_map.iterator();
        while (_it.next()) |_entry| {
            smoke_run.setEnvironmentVariable(_entry.key_ptr.*, _entry.value_ptr.*);
        }
    }
    smoke_run.step.dependOn(&reranker_backend_management_run.step);
    test_step.dependOn(&smoke_run.step);

    const summarization_module = b.createModule(.{
        .root_source_file = b.path("src/summarization_test.zig"),
        .target = target,
        .optimize = optimize,
        .link_libc = true,
    });
    summarization_module.addImport("xberg", xberg_module);
    const summarization_tests = b.addTest(.{
        .name = "summarization_test",
        .root_module = summarization_module,
        .use_llvm = true,
    });
    summarization_tests.root_module.addRPath(.{ .cwd_relative = ffi_path_abs });
    const summarization_run = b.addRunArtifact(summarization_tests);
    summarization_run.setCwd(b.path("../../test_documents"));
    summarization_run.setEnvironmentVariable("CRAWLBERG_ALLOW_PRIVATE_NETWORK", "true");
    if (mock_server_url) |_url| {
        summarization_run.setEnvironmentVariable("MOCK_SERVER_URL", _url);
    }
    if (mock_servers_json) |_json| {
        summarization_run.setEnvironmentVariable("MOCK_SERVERS", _json);
    }
    {
        var _it = mock_servers_map.iterator();
        while (_it.next()) |_entry| {
            summarization_run.setEnvironmentVariable(_entry.key_ptr.*, _entry.value_ptr.*);
        }
    }
    summarization_run.step.dependOn(&smoke_run.step);
    test_step.dependOn(&summarization_run.step);

    const url_module = b.createModule(.{
        .root_source_file = b.path("src/url_test.zig"),
        .target = target,
        .optimize = optimize,
        .link_libc = true,
    });
    url_module.addImport("xberg", xberg_module);
    const url_tests = b.addTest(.{
        .name = "url_test",
        .root_module = url_module,
        .use_llvm = true,
    });
    url_tests.root_module.addRPath(.{ .cwd_relative = ffi_path_abs });
    const url_run = b.addRunArtifact(url_tests);
    url_run.setCwd(b.path("../../test_documents"));
    url_run.setEnvironmentVariable("CRAWLBERG_ALLOW_PRIVATE_NETWORK", "true");
    if (mock_server_url) |_url| {
        url_run.setEnvironmentVariable("MOCK_SERVER_URL", _url);
    }
    if (mock_servers_json) |_json| {
        url_run.setEnvironmentVariable("MOCK_SERVERS", _json);
    }
    {
        var _it = mock_servers_map.iterator();
        while (_it.next()) |_entry| {
            url_run.setEnvironmentVariable(_entry.key_ptr.*, _entry.value_ptr.*);
        }
    }
    url_run.step.dependOn(&summarization_run.step);
    test_step.dependOn(&url_run.step);

    const validator_management_module = b.createModule(.{
        .root_source_file = b.path("src/validator_management_test.zig"),
        .target = target,
        .optimize = optimize,
        .link_libc = true,
    });
    validator_management_module.addImport("xberg", xberg_module);
    const validator_management_tests = b.addTest(.{
        .name = "validator_management_test",
        .root_module = validator_management_module,
        .use_llvm = true,
    });
    validator_management_tests.root_module.addRPath(.{ .cwd_relative = ffi_path_abs });
    const validator_management_run = b.addRunArtifact(validator_management_tests);
    validator_management_run.setCwd(b.path("../../test_documents"));
    validator_management_run.setEnvironmentVariable("CRAWLBERG_ALLOW_PRIVATE_NETWORK", "true");
    if (mock_server_url) |_url| {
        validator_management_run.setEnvironmentVariable("MOCK_SERVER_URL", _url);
    }
    if (mock_servers_json) |_json| {
        validator_management_run.setEnvironmentVariable("MOCK_SERVERS", _json);
    }
    {
        var _it = mock_servers_map.iterator();
        while (_it.next()) |_entry| {
            validator_management_run.setEnvironmentVariable(_entry.key_ptr.*, _entry.value_ptr.*);
        }
    }
    validator_management_run.step.dependOn(&url_run.step);
    test_step.dependOn(&validator_management_run.step);

}
