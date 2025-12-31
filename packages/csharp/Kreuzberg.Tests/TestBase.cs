using System;
using Xunit;

namespace Kreuzberg.Tests;

/// <summary>
/// Base class for all test classes that ensures proper cleanup of registered callbacks.
/// This prevents resource leaks from accumulated GCHandles.
/// </summary>
public abstract class TestBase : IDisposable
{
    protected TestBase()
    {
        NativeTestHelper.EnsureNativeLibraryLoaded();
    }

    public virtual void Dispose()
    {
        // Clean up all registered callbacks after each test class
        // This prevents GCHandle accumulation which can cause test host crashes
        CleanupRegistrations();
        GC.SuppressFinalize(this);
    }

    private static void CleanupRegistrations()
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
