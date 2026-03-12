---
name: priority-selector
description: Implement and optimize priority-based plugin selection
---
Implement and optimize priority-based plugin selection.

Context:
- Key concepts: Priority levels (0-255), priority-based arbitration, fallback chain execution, capability-aware selection

Capabilities:
- Design priority systems for plugin selection
- Implement priority sorting algorithms
- Handle priority conflicts
- Debug selection logic
- Optimize selection performance

Patterns:
- Plugins sorted by priority (highest first)
- Fallback chains iterate through plugins until success
- Capability matching filters available plugins
- Priority enables custom overrides and fallbacks
