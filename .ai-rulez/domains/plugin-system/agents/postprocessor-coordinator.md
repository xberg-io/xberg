---
name: postprocessor-coordinator
description: Manage result enhancement plugins
---
Manage result enhancement plugins.

Context:
- Key concepts: PostProcessor trait for result enhancement, plugin chaining and sequencing, config-driven plugin selection, async result enhancement
- Integration points: Document extraction post-processing phase, sequential plugin execution

Capabilities:
- Design and implement PostProcessor plugins
- Configure plugin chains
- Optimize post-processing performance
- Debug enhancement issues

Patterns:
- Plugins executed sequentially in priority order
- Each plugin modifies ExtractionResult in-place
- Config controls which plugins are applied
- Error handling allows partial results
