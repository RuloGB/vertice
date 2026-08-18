---
name: valid-folded-description
description: >
  This description is written as a folded block scalar that spans
  several lines of source and must be joined into a single string
  with spaces, never truncated or altered.
license: MIT
disable-model-invocation: true
metadata:
  author: vertice-team
  version: "1.0"
---

# Valid Folded Description

This fixture also doubles as the generic-reuse probe input: a second,
non-skill target type reads this same file to prove unknown-field
tolerance (`license`, `disable-model-invocation`, `metadata`).
