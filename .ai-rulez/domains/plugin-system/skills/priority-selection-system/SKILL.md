---
name: priority-selection-system
---
Implement priority-based plugin selection

1. Define priority semantics:
   - Higher = more priority
   - Default = 50 (middle)
   - Custom = > 50
   - Fallback = < 50
2. Get matching plugins from registry
3. Sort by priority (descending)
4. For each plugin:
   a. Check if supports capability
   b. If not, try next
   c. If yes, attempt operation
   d. On success, return
   e. On failure, try next (fallback)
5. Return best result or error
6. Document priority decisions in logs
