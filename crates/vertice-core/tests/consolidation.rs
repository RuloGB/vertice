//! Integration suite for `vertice_core::consolidate::consolidate`, exercised
//! against the real skill scanner and the reference fixture tree committed
//! under `crates/vertice-core/tests/fixtures/roots/reference/` (69 files, 25
//! unique names: 22 shared across all three skill roots, 3 present in
//! exactly one, none in exactly two). `design.md` (duplicate-consolidation)
//! §9 is the authority for every assertion below.

use std::path::PathBuf;

use vertice_core::consolidate::consolidate;
use vertice_core::model::SearchRootId;
use vertice_core::skills;

/// Build a path under `crates/vertice-core/tests/fixtures/roots/<case>/`
/// from per-segment pushes — never a `"/"`-joined literal, so it stays
/// separator-correct on Windows.
fn fixture_home(case: &str) -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("tests");
    path.push("fixtures");
    path.push("roots");
    path.push(case);
    path
}

/// Canonical skill-root order, matching `consolidate::ROOT_ORDER`'s
/// skill-facing prefix (design §4) — used only to assert `locations`
/// ordering, never to compute it.
const SKILL_ROOT_ORDER: [&str; 3] = ["claude-skills", "agents-skills", "opencode-skills"];

fn root_rank(id: &SearchRootId) -> usize {
    SKILL_ROOT_ORDER
        .iter()
        .position(|&candidate| candidate == id.0)
        .unwrap_or(SKILL_ROOT_ORDER.len())
}

/// CA-2: the reference fixture's 69 flattened entries collapse into exactly
/// 25 components.
#[test]
fn reference_fixture_collapses_sixty_nine_entries_into_twenty_five_components() {
    let home = fixture_home("reference");
    let scan = skills::scan(&home);
    assert_eq!(
        scan.components.len(),
        69,
        "fixture oracle: 69 flattened entries"
    );

    let consolidated = consolidate(scan.components);

    assert_eq!(consolidated.len(), 25);
}

/// CA-3: exactly 22 components have `locations.len() == 3`, each in
/// canonical root order.
#[test]
fn exactly_twenty_two_components_have_three_locations_in_canonical_order() {
    let home = fixture_home("reference");
    let scan = skills::scan(&home);

    let consolidated = consolidate(scan.components);

    let triple_located: Vec<_> = consolidated
        .iter()
        .filter(|c| c.locations.len() == 3)
        .collect();

    assert_eq!(triple_located.len(), 22);

    for component in &triple_located {
        let ranks: Vec<usize> = component
            .locations
            .iter()
            .map(|loc| root_rank(&loc.root))
            .collect();
        let mut sorted_ranks = ranks.clone();
        sorted_ranks.sort_unstable();
        assert_eq!(
            ranks, sorted_ranks,
            "locations for {:?} must already be in canonical root order",
            component.id
        );
    }
}

/// CA-4: exactly 3 components have `locations.len() == 1` — the three
/// single-root fixture names — and zero have `locations.len() == 2`.
#[test]
fn exactly_three_components_have_a_single_location_and_none_has_two() {
    let home = fixture_home("reference");
    let scan = skills::scan(&home);

    let consolidated = consolidate(scan.components);

    let single_located: Vec<_> = consolidated
        .iter()
        .filter(|c| c.locations.len() == 1)
        .map(|c| c.name.as_str())
        .collect();

    assert_eq!(single_located.len(), 3);
    assert!(single_located.contains(&"claude-only-01"));
    assert!(single_located.contains(&"agents-only-01"));
    assert!(single_located.contains(&"agents-only-02"));

    assert!(consolidated.iter().all(|c| c.locations.len() != 2));
}

/// Conservation: the sum of `locations.len()` across the output equals the
/// input length — no copy hidden, no winner elected.
#[test]
fn total_location_count_is_conserved() {
    let home = fixture_home("reference");
    let scan = skills::scan(&home);
    let input_len = scan.components.len();

    let consolidated = consolidate(scan.components);

    let total_locations: usize = consolidated.iter().map(|c| c.locations.len()).sum();

    assert_eq!(total_locations, input_len);
}

/// CA-8, both halves: a `_shared`-prefixed name survives with no
/// name-prefix filtering AND, existing in all three roots, consolidates
/// into ONE component carrying all three locations in canonical root
/// order — exactly like any other duplicated name.
#[test]
fn underscore_shared_existing_in_three_roots_consolidates_like_any_other_name() {
    let home = fixture_home("underscore-shared");
    let scan = skills::scan(&home);
    let input_len = scan.components.len();

    let consolidated = consolidate(scan.components);

    let underscore_shared: Vec<_> = consolidated
        .iter()
        .filter(|c| c.name == "_shared")
        .collect();

    assert_eq!(
        input_len, 3,
        "the fixture must place _shared under all three roots"
    );
    assert_eq!(
        underscore_shared.len(),
        1,
        "no name-prefix filter may drop it"
    );
    assert_eq!(
        underscore_shared[0].locations.len(),
        3,
        "CA-8 requires _shared to be duplicated across the three roots"
    );

    let roots: Vec<&str> = underscore_shared[0]
        .locations
        .iter()
        .map(|loc| loc.root.0.as_str())
        .collect();
    assert_eq!(roots, ["claude-skills", "agents-skills", "opencode-skills"]);
}

/// Determinism: two consecutive `consolidate` calls over the same input
/// yield byte-identical output vectors.
#[test]
fn two_consecutive_calls_over_the_same_input_yield_identical_output() {
    let home = fixture_home("reference");
    let scan = skills::scan(&home);

    let first = consolidate(scan.components.clone());
    let second = consolidate(scan.components);

    assert_eq!(first, second);
}

/// Precedence, real pipeline: the `.claude/skills` copy has a blank folded
/// description, the `.agents/skills` copy has a real one — the merged
/// component keeps the real description, never `Some("\n")`. `Some("\n")`
/// only arises through actual YAML parsing, so this cannot be a unit test
/// (design §9).
#[test]
fn real_pipeline_precedence_prefers_the_later_roots_real_description() {
    let home = fixture_home("precedence-description");
    let scan = skills::scan(&home);

    let consolidated = consolidate(scan.components);

    assert_eq!(consolidated.len(), 1);
    let description = consolidated[0]
        .description
        .as_deref()
        .expect("the agents-skills copy has a real description");

    assert_ne!(description, "\n");
    assert!(!description.trim().is_empty());
    assert!(description.contains("real"));
}
