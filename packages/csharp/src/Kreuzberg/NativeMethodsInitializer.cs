#nullable enable

using System;
using System.IO;
using System.Reflection;
using System.Runtime.InteropServices;

namespace Kreuzberg;

/// <summary>
/// Initializes the native library resolver for kreuzberg_ffi.
/// Handles loading the native library from the RID-specific runtime directory structure.
/// </summary>
internal static partial class NativeMethods
{
    static NativeMethods()
    {
        NativeLibrary.SetDllImportResolver(typeof(NativeMethods).Assembly, ImportResolver);
    }

    private static IntPtr ImportResolver(string libraryName, System.Reflection.Assembly assembly, DllImportSearchPath? searchPath)
    {
        if (libraryName != "kreuzberg_ffi")
        {
            return IntPtr.Zero;
        }

        // Get the current runtime identifier
        var rid = GetCurrentRuntimeIdentifier();
        if (rid == null)
        {
            return IntPtr.Zero;
        }

        // Construct the path to the native library in the RID-specific directory
        var assemblyDirectory = Path.GetDirectoryName(assembly.Location) ?? "";
        var nativeLibraryPath = Path.Combine(assemblyDirectory, "runtimes", rid, "native", GetNativeLibraryName());

        if (File.Exists(nativeLibraryPath))
        {
            if (NativeLibrary.TryLoad(nativeLibraryPath, assembly, searchPath, out var handle))
            {
                return handle;
            }
        }

        return IntPtr.Zero;
    }

    private static string? GetCurrentRuntimeIdentifier()
    {
        // Determine the current platform and architecture
        if (RuntimeInformation.IsOSPlatform(OSPlatform.Windows))
        {
            return RuntimeInformation.ProcessArchitecture switch
            {
                Architecture.X64 => "win-x64",
                Architecture.Arm64 => "win-arm64",
                Architecture.X86 => "win-x86",
                _ => null,
            };
        }
        else if (RuntimeInformation.IsOSPlatform(OSPlatform.OSX))
        {
            return RuntimeInformation.ProcessArchitecture switch
            {
                Architecture.X64 => "osx-x64",
                Architecture.Arm64 => "osx-arm64",
                _ => null,
            };
        }
        else if (RuntimeInformation.IsOSPlatform(OSPlatform.Linux))
        {
            return RuntimeInformation.ProcessArchitecture switch
            {
                Architecture.X64 => "linux-x64",
                Architecture.Arm64 => "linux-arm64",
                Architecture.Arm => "linux-arm",
                _ => null,
            };
        }

        return null;
    }

    private static string GetNativeLibraryName()
    {
        if (RuntimeInformation.IsOSPlatform(OSPlatform.Windows))
        {
            return "kreuzberg_ffi.dll";
        }
        else if (RuntimeInformation.IsOSPlatform(OSPlatform.OSX))
        {
            return "libkreuzberg_ffi.dylib";
        }
        else
        {
            // Linux and other Unix-like systems
            return "libkreuzberg_ffi.so";
        }
    }

    // ===== Fixed FFI signatures for functions missing from alef-generated NativeMethods =====
    // These overloads include the correct parameter lists for functions that work with byte arrays.

}
