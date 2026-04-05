using System;
using System.Collections.Generic;
using System.IO;
using System.Runtime.InteropServices;
using Xunit;
using Xunit.Sdk;

namespace Kreuzberg.Tests;

internal static class NativeTestHelper
{
    private static readonly Lazy<string> WorkspaceRootLoader = new(ResolveWorkspaceRoot);
    private static readonly Lazy<bool> LibraryLoaded = new(() =>
    {
        var root = WorkspaceRoot;
        var lib = LibraryFileName();
        var candidateList = new List<string>();
        var ffiDir = Environment.GetEnvironmentVariable("KREUZBERG_FFI_DIR");
        if (!string.IsNullOrEmpty(ffiDir))
        {
            candidateList.Add(Path.Combine(ffiDir, lib));
        }
        candidateList.Add(Path.Combine(root, "target", "release", lib));
        candidateList.Add(Path.Combine(root, "target", "debug", lib));
        var candidates = candidateList.ToArray();

        // Load Pdfium before loading the FFI library to ensure it's available
        LoadPdfiumIfPresent(root);

        foreach (var candidate in candidates)
        {
            if (File.Exists(candidate))
            {
                try
                {
                    NativeLibrary.Load(candidate);
                }
                catch (InvalidOperationException ex) when (ex.Message.Contains("already loaded"))
                {
                    // Library already loaded - this is fine when using a custom test framework
                }
                return true;
            }
        }

        throw new XunitException($"Native library not found. Checked: {string.Join(", ", candidates)}");
    });

    internal static string WorkspaceRoot => WorkspaceRootLoader.Value;

    internal static void EnsureNativeLibraryLoaded() => _ = LibraryLoaded.Value;

    internal static string GetDocumentPath(string relative)
    {
        var path = Path.Combine(WorkspaceRoot, "test_documents", relative.Replace('/', Path.DirectorySeparatorChar));
        if (!File.Exists(path))
        {
            throw new XunitException($"Test document missing: {path}");
        }
        return path;
    }

    private static string ResolveWorkspaceRoot()
    {
        var dir = AppContext.BaseDirectory;
        while (true)
        {
            if (Directory.Exists(Path.Combine(dir, "test_documents")))
            {
                return dir;
            }

            var parent = Directory.GetParent(dir);
            if (parent == null)
            {
                break;
            }
            dir = parent.FullName;
        }

        throw new XunitException("Could not locate workspace root (missing test_documents directory).");
    }

    private static string LibraryFileName()
    {
        if (OperatingSystem.IsWindows())
        {
            return "kreuzberg_ffi.dll";
        }

        if (OperatingSystem.IsMacOS())
        {
            return "libkreuzberg_ffi.dylib";
        }

        return "libkreuzberg_ffi.so";
    }

    private static void LoadPdfiumIfPresent(string root)
    {
        var pdfiumNames = new[]
        {
            "libpdfium.dylib",
            "pdfium.dll",
            "libpdfium.so",
        };

        foreach (var name in pdfiumNames)
        {
            var path = Path.Combine(root, "target", "release", name);
            if (!File.Exists(path))
            {
                path = Path.Combine(root, "target", "debug", name);
            }

            if (File.Exists(path))
            {
                try
                {
                    NativeLibrary.Load(path);
                }
                catch (InvalidOperationException ex) when (ex.Message.Contains("already loaded"))
                {
                    // Pdfium already loaded - this is fine when using a custom test framework
                }
                return;
            }
        }
    }
}
