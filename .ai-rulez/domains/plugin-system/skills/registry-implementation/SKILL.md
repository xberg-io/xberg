---
name: registry-implementation
---
Build plugin registry systems

1. Define registry struct:
   - Arc<RwLock<Vec<Plugin>>>
   - Arc<RwLock<HashMap<MimeType, Indices>>>
2. Implement register():
   - Acquire write lock
   - Add to storage
   - Update indices
   - Release lock
3. Implement get_for_capability():
   - Acquire read lock
   - Query indices
   - Sort by priority
   - Release lock
   - Return plugins
4. Implement unregister():
   - Acquire write lock
   - Remove from storage
   - Update indices
5. Implement clear():
   - Acquire write lock
   - Clear all data structures
6. Test concurrent operations
