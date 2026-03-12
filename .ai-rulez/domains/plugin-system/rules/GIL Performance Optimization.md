---
name: GIL Performance Optimization
priority: high
---
Minimize GIL overhead in Python plugins

- Profile GIL usage (typical: 5-55us per call)
- Cache frequently-accessed Python data
- Minimize GIL hold times
- Use detach() for expensive operations
- Measure optimization impact
