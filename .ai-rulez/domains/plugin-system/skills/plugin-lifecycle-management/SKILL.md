---
name: plugin-lifecycle-management
---
Manage plugin initialization and shutdown

1. In initialize():
   a. Load resources (models, configs)
   b. Validate dependencies
   c. Set up connections
   d. Return error if failure
2. Track plugin state (ready, error)
3. In shutdown():
   a. Release resources
   b. Close connections
   c. Persist state if needed
   d. Return error if failure
4. Test lifecycle with cleanup verification
5. Handle errors during both phases
