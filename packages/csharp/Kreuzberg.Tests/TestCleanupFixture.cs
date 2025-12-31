using System;
using Xunit;

namespace Kreuzberg.Tests;

/// <summary>
/// Test fixture that ensures cleanup of registered callbacks after each test method.
/// This prevents resource leaks from accumulated GCHandles for post-processors,
/// validators, and OCR backends.
/// </summary>
public class TestCleanupFixture : IDisposable
{
    public TestCleanupFixture()
    {
        // Ensure native library is loaded before any tests run
        NativeTestHelper.EnsureNativeLibraryLoaded();
    }

    public void Dispose()
    {
        // Clean up after each test method to prevent GCHandle accumulation
        CleanupAllRegistrations();
    }

    private static void CleanupAllRegistrations()
    {
        try
        {
            KreuzbergClient.ClearPostProcessors();
        }
        catch
        {
            // Ignore cleanup errors - some tests may not have registered anything
        }

        try
        {
            KreuzbergClient.ClearValidators();
        }
        catch
        {
            // Ignore cleanup errors
        }

        try
        {
            KreuzbergClient.ClearOcrBackends();
        }
        catch
        {
            // Ignore cleanup errors
        }
    }
}
