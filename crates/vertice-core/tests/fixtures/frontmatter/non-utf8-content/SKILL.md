---
name: non-utf8-content
description: This description carries a raw ÿ byte, which is not valid UTF-8.
---

# Non-UTF-8 Content

The frontmatter above is well-formed YAML apart from the single invalid
byte on the description line. A reader that decodes leniently would parse
this file successfully; a reader that validates UTF-8 before splitting the
fence rejects it. That distinction is what this fixture exists to prove.
