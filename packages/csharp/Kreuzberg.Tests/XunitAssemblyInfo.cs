using System;
using System.Runtime.CompilerServices;
using Xunit;

// Assembly-level attributes must come first
[assembly: CollectionBehavior(DisableTestParallelization = true)]

namespace Kreuzberg.Tests;

/// <summary>
/// Module initializer that runs before any tests execute.
/// This ensures Pdfium is initialized exactly once per test run.
/// </summary>
internal static class PdfiumModuleInitializer
{
    [ModuleInitializer]
    public static void Initialize()
    {
        PdfiumInitializer.Initialize();
    }
}

/// <summary>
/// Static initializer for Pdfium library.
/// Ensures initialization happens exactly once per test run.
/// </summary>
internal static class PdfiumInitializer
{
    private static volatile bool s_initialized = false;
    private static readonly object s_lock = new();

    public static void Initialize()
    {
        // Double-checked locking to ensure initialization happens exactly once
        if (s_initialized)
            return;

        lock (s_lock)
        {
            if (s_initialized)
                return;

            try
            {
                System.Console.WriteLine("[Test Init] Loading native library...");

                // Only load the FFI library - let Rust handle Pdfium initialization lazily
                // Pdfium will be initialized on first PDF extraction call from Rust
                NativeTestHelper.EnsureNativeLibraryLoaded();

                // Register cleanup handler to run when AppDomain unloads
                AppDomain.CurrentDomain.ProcessExit += (sender, e) => CleanupAllRegistrations();
                AppDomain.CurrentDomain.DomainUnload += (sender, e) => CleanupAllRegistrations();

                System.Console.WriteLine("[Test Init] Native library loaded. Pdfium will initialize lazily on first use.");
                s_initialized = true;
            }
            catch (Exception ex)
            {
                System.Console.WriteLine($"[Test Init] Warning: {ex.Message}");
                s_initialized = true; // Mark as initialized to avoid repeated attempts
            }
        }
    }

    private static void CleanupAllRegistrations()
    {
        try
        {
            System.Console.WriteLine("[Test Cleanup] Cleaning up registered callbacks...");
            KreuzbergClient.ClearPostProcessors();
            KreuzbergClient.ClearValidators();
            KreuzbergClient.ClearOcrBackends();
            System.Console.WriteLine("[Test Cleanup] Cleanup complete.");
        }
        catch (Exception ex)
        {
            System.Console.WriteLine($"[Test Cleanup] Warning during cleanup: {ex.Message}");
        }
    }
}
