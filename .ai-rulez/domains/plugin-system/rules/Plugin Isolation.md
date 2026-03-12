---
name: Plugin Isolation
priority: high
---
Plugins must not depend on or interfere with each other

- Plugins have independent initialization
- Plugin failures don't affect other plugins
- Shared resources (cache, config) properly synchronized
- No implicit plugin ordering dependencies
- Graceful handling of missing plugins
