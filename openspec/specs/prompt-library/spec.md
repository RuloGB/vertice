# Delta for Prompt Library

## ADDED Requirements

### Requirement: Local Prompt CRUD
The system MUST provide a Prompts page where users can list, create, edit, and delete prompts. Each prompt MUST include `id`, `title`, `body`, `tags`, `bestForContext`, and `updatedAt`. The system MUST block save when `title` or `body` is empty; `tags` and `bestForContext` MAY be empty.

#### Scenario: Create and list a prompt
- GIVEN the library is empty
- WHEN the user saves a valid prompt
- THEN the prompt appears in the Prompts list with its stored fields

#### Scenario: Edit a prompt
- GIVEN an existing prompt
- WHEN the user saves changes to its title, body, tags, or best-for context
- THEN the same prompt identity is preserved and `updatedAt` reflects the latest save

#### Scenario: Empty title or body blocks save
- GIVEN the user is creating or editing a prompt
- WHEN `title` or `body` is empty at save time
- THEN the save is blocked and the prompt is not persisted

### Requirement: Local Search, Actions, and Copy
The system MUST search prompts by normalized substring over `title`, `tags`, `body`, and `bestForContext`. Search MUST ignore case, surrounding whitespace, and accents. The page MUST let the user copy a prompt body manually without invoking any external client integration. Visible Copy, Edit, and Delete actions MUST expose clear hover and keyboard-focus feedback without changing their accessible names; Delete MUST retain danger semantics.

#### Scenario: Search matches multiple fields
- GIVEN stored prompts whose query text appears in different searchable fields
- WHEN the user enters that query
- THEN every prompt with a normalized substring match is shown

#### Scenario: Search does not use fuzzy ranking
- GIVEN no searchable field contains the normalized query as a substring
- WHEN the user searches
- THEN no result is returned because fuzzy or ranked matching is out of scope

#### Scenario: Copy is manual and local
- GIVEN a visible prompt
- WHEN the user activates copy
- THEN the prompt body is copied for manual reuse and no external client is opened or modified

#### Scenario: Action feedback preserves semantics
- GIVEN a visible prompt row
- WHEN the user hovers or tabs to Copy, Edit, or Delete
- THEN each action shows a visible interactive state
- AND Copy and Edit keep their accessible names
- AND Delete keeps destructive danger styling

### Requirement: Paginated Results and Durable Page States
The system MUST paginate visible prompt results with page-size choices of 5, 10, and 15, using the same page-navigation behavior as Skills and Agents. A query change MUST reset the current page to the first page. If filtering or page-size changes make the current page out of range, the page MUST clamp to the last available page. The system MUST persist prompts in a schema-versioned `prompts.json` stored only inside the application data directory. Writes MUST be atomic. The Prompts page MUST distinguish loading, empty, success, and failure states.

#### Scenario: Query reset returns to first page
- GIVEN the user is on a later prompt-results page
- WHEN the search query changes
- THEN the visible prompt results restart on page 1

#### Scenario: Page bounds clamp after result shrink or page-size change
- GIVEN the user is on a page that stops existing after filtering or changing page size
- WHEN the visible result count or page size reduces the page count
- THEN the current page clamps to the last available page

#### Scenario: Prompts survive restart
- GIVEN one or more saved prompts
- WHEN the application restarts
- THEN the same prompts are loaded from `prompts.json`

#### Scenario: Empty state is distinct from failure
- GIVEN the prompts store loads successfully with zero prompts
- WHEN the Prompts page renders
- THEN it shows an empty state, not an error state

#### Scenario: Store failure shows a failure state
- GIVEN loading or saving the prompts store fails
- WHEN the Prompts page settles
- THEN it shows a failure state without pretending the library is empty
