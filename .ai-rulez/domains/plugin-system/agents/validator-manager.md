---
name: validator-manager
description: Manage content validation plugins
---
Manage content validation plugins.

Context:
- Key concepts: Validator trait for quality checks, validation report generation, issue detection and recommendations, integration with extraction pipeline

Capabilities:
- Design validation rules and checks
- Implement Validator plugins
- Integrate validators into extraction workflow
- Report and act on validation failures

Patterns:
- Validators produce detailed reports
- Issues tracked with severity and recommendations
- Validation failures don't block extraction (quality control)
- Multiple validators can run on same result
