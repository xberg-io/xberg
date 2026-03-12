---
name: Dynamic Registration
priority: high
---
Support runtime plugin registration/unregistration

- Plugins can be registered after initialization
- Plugins can be unregistered without restart
- Unregistration safe if plugin in use
- Clear() removes all plugins atomically
- Document registration/unregistration flow
