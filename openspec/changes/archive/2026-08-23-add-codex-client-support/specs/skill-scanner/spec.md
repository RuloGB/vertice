# Delta for Skill Scanner

The skill scanner's own code (`skills.rs`) is unchanged by this proposal — it
is already client-agnostic and walks whatever roots `roots::skill_roots`
returns. The delta below is entirely in the root count: a fourth,
`codex-skills`, root joins the three existing ones. The
plugin-exclusion argument, which rests on "the scanner only ever walks the
fixed roots", is restated at four so it does not silently drift out of sync
with the code. The reference-fixture pin (69 entries) is untouched by this
change: Codex fixtures live in a new, separate fixture home, never inside
`crates/vertice-core/tests/fixtures/roots/reference/`, so that pin's
requirement below is not modified.

## MODIFIED Requirements

### Requirement: User Root Set Is Fixed and Hardcoded

The scanner MUST resolve exactly four user roots by concatenating the
resolved home directory with a hardcoded, per-client relative suffix:
`.claude/skills/`, `.agents/skills/`, `.config/opencode/skills/`, and
`.codex/skills/`. The singular `.config/opencode/skill/` MUST be treated as
the same OpenCode root as its plural form, matching the glob
`{skill,skills}/**/SKILL.md`. Root paths MUST NOT be derived from any OS
config-directory convention (e.g. `%APPDATA%` on Windows); they are computed
from the home directory alone. The `codex-skills` root MUST be appended after
the three existing roots, never inserted before or between them, so that
canonical root order for the three pre-existing roots — and therefore
first-non-empty field precedence for any component already merged across
them — is unchanged by this addition.
(Previously: exactly three user roots — `.claude/skills/`,
`.agents/skills/`, and `.config/opencode/skills/` — with no `codex-skills`
root.)

#### Scenario: OpenCode root resolves under the home directory on every OS

- GIVEN the scanner runs on any supported platform
- WHEN the OpenCode root is resolved
- THEN it is `<home>/.config/opencode/skills/` (or its singular alias), never a platform config-dir path

#### Scenario: Singular and plural OpenCode roots are the same root

- GIVEN fixtures for both `.config/opencode/skill/` and `.config/opencode/skills/`
- WHEN the scanner resolves the OpenCode root
- THEN both are scanned as one logical root, not two

#### Scenario: The Codex root resolves under the home directory on every OS

- GIVEN the scanner runs on any supported platform
- WHEN the Codex skill root is resolved
- THEN it is `<home>/.codex/skills/`, never a platform config-dir path

#### Scenario: A Codex SKILL.md with vendor-specific extra keys still parses

- GIVEN a fixture Codex skill root containing a `SKILL.md` whose frontmatter declares `name`, `description`, and the Codex-specific keys `disable-model-invocation`, `user-invocable`, `license`, and `metadata.*`
- WHEN the scanner walks that root
- THEN a `Component` is produced for it, with the unmodelled keys silently ignored rather than causing a parse failure — the same permissive behavior the frontmatter reader already applies to the other three roots

#### Scenario: The Codex root is appended last, not inserted mid-order

- GIVEN the four resolved skill roots
- WHEN their order is inspected
- THEN `.codex/skills/` is the fourth entry, and the relative order of `.claude/skills/`, `.agents/skills/`, and `.config/opencode/skills/` is identical to their order before this root was added

### Requirement: Every Skill Component Has Scope::User

Every `Component` produced by this scanner MUST have `scope: Scope::User`.
The scanner MUST NOT construct any root or component associated with
`Scope::Project` or `Scope::Local`.
(Previously: the "outside the roots" scenario referenced three roots; no
change to the requirement text itself.)

#### Scenario: All discovered skills are User-scoped

- GIVEN a full scan across the four roots
- WHEN the produced `Component` values are inspected
- THEN every one has `scope == Scope::User`

#### Scenario: A project-shaped tree outside the four roots yields nothing

- GIVEN a fixture `.claude/skills/` directory located outside the four resolved roots
- WHEN the scanner runs
- THEN no `Component` is produced from it

### Requirement: No Plugin-Provided Skill Appears In The Result

The scan result MUST NOT contain any component sourced from a plugin-provided
location. This MUST hold because the scanner only ever walks the four fixed
roots — no plugin-exclusion filter is required or permitted as a substitute
for root scoping.
(Previously: "the scanner only ever walks the three fixed roots".)

#### Scenario: A plugin-shaped fixture outside the four roots is absent from the result

- GIVEN a fixture tree resembling a plugin skill location, located outside the four resolved roots
- WHEN the scanner runs
- THEN no `Component` in the result traces back to that fixture
