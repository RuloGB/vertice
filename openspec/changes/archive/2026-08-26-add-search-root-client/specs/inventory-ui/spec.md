# Delta for Inventory UI

## ADDED Requirements

### Requirement: Detail Pages Summarize AI Clients By Location Ownership

The AgentDetail, SkillDetail, and McpDetail pages MUST consume each
`Location.client` value and render a deduplicated client summary. Groups MUST
appear in fixed order: `claudeCode`, `openCode`, `codex`, then shared (`null`),
and MUST include the number of locations in each group. Groups with no
locations MUST NOT be rendered.

#### Scenario: Locations are deduplicated, ordered, and counted

- GIVEN a component with locations in arbitrary order, including repeated clients
- WHEN any detail page computes its AI-client groups
- THEN each distinct client appears once with its location count
- AND the order is Claude Code, OpenCode, Codex, then Shared

#### Scenario: Shared locations use localized common-noun copy

- GIVEN a component has a location whose `client` is `null`
- WHEN its AI-client group is rendered
- THEN the group label comes from the i18n key `aiClients.shared`
- AND the English and Spanish values are “Shared” and “Compartido” respectively

#### Scenario: Client names remain proper nouns

- GIVEN a group whose client is `claudeCode`, `openCode`, or `codex`
- WHEN its label is rendered in either supported locale
- THEN it is the hardcoded proper noun “Claude Code”, “OpenCode”, or “Codex”
- AND no client display name is looked up as an i18n key

#### Scenario: Existing empty state is preserved

- GIVEN an Agent, Skill, or MCP component has no locations
- WHEN its detail page renders the AI Clients section
- THEN the existing localized empty-state message is shown
- AND no client group row is fabricated
