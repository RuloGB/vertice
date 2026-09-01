# Delta for Domain Model

## ADDED Requirements

### Requirement: ClientInstallSlot Admits A Fifth Variant For The OpenCode Desktop Probe

`ClientInstallSlot` MUST gain a fifth variant, `OpenCodeDesktop`, sanctioned
evolution per this type's documented growth-follows-platform/adapter-growth
rule. Every exhaustive `match` over `ClientInstallSlot` in core MUST be
updated to cover it, never gaining a wildcard arm.

#### Scenario: ClientInstallSlot is exhaustively matchable with five variants

- GIVEN a `match` over every `ClientInstallSlot` variant, including `OpenCodeDesktop`, with no wildcard arm
- WHEN the code is compiled
- THEN compilation succeeds, proving the enum remains closed

#### Scenario: ClientPresence can carry the new slot

- GIVEN a `ClientPresence` value constructed with `slot: ClientInstallSlot::OpenCodeDesktop`
- WHEN the value is inspected
- THEN it is a valid, representable value, structurally identical in shape to one carrying any other slot variant
