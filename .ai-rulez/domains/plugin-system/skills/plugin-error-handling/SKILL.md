---
name: plugin-error-handling
---
Handle plugin errors and implement recovery

1. Classify plugin errors:
   - InitializationError
   - ExecutionError
   - ConfigurationError
2. Catch plugin exceptions
3. Wrap in KreuzbergError with context
4. Preserve original error information
5. Implement fallback:
   a. Try next-priority plugin
   b. Return partial result if available
   c. Log error for debugging
6. Return structured error to user
