---
name: blank-description
description: >
---

# blank-description

The `.claude/skills` copy of this fixture carries an empty folded block
scalar description — `description: >` with nothing under it — which
`frontmatter::read` surfaces as `Some("\n")`, present but blank to a reader.
The `.agents/skills` copy of the same name carries a real description, and
must win consolidation precedence over this one.
