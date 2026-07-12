/**
 * Supertrait transitive cycle detection tests.
 *
 * Exercises the DFS-based cycle detector in `typechecker::supertraits`.
 * Covers:
 *   - Sanity case: a single trait with no supertraits
 *   - Linear chain: A: B: C — must NOT be flagged
 *   - Two-level cycle: A: B, B: A — must be flagged
 *   - Three-level cycle: A: B: C: A — must list all three participants
 *
 * @author Ruyi Team
 * @date 2026-07-11
 */
use std::collections::HashMap;

use ruyic::typechecker::diagnostics::DiagnosticBag;
use ruyic::typechecker::supertraits::{detect_supertrait_cycle, validate_supertrait_chain};

// ── 1. Sanity ─────────────────────────────────────────────────

#[test]
fn test_no_supertrait_no_cycle() {
    // A single trait with no supertraits → no cycle, no diagnostics.
    let mut supertraits: HashMap<String, Vec<String>> = HashMap::new();
    supertraits.insert("A".to_string(), vec![]);

    let mut bag = DiagnosticBag::new();
    validate_supertrait_chain(&mut bag, "A", &supertraits);

    assert!(
        !bag.has_errors(),
        "single trait with no supertraits must not be flagged as cyclic, got: {:?}",
        bag.diagnostics()
    );
}

// ── 2. Linear chain (must not be flagged) ─────────────────────

#[test]
fn test_two_level_chain_no_cycle() {
    // A : B, B : C, C : {} — pure linear chain.
    let mut supertraits: HashMap<String, Vec<String>> = HashMap::new();
    supertraits.insert("A".to_string(), vec!["B".to_string()]);
    supertraits.insert("B".to_string(), vec!["C".to_string()]);
    supertraits.insert("C".to_string(), vec![]);

    let result = detect_supertrait_cycle("A", &supertraits);
    assert!(
        result.is_ok(),
        "linear chain must not be a cycle: {:?}",
        result
    );
}

// ── 3. Two-level cycle (A → B → A) ────────────────────────────

#[test]
fn test_two_level_cycle() {
    // A : B, B : A — direct mutual cycle.
    let mut supertraits: HashMap<String, Vec<String>> = HashMap::new();
    supertraits.insert("A".to_string(), vec!["B".to_string()]);
    supertraits.insert("B".to_string(), vec!["A".to_string()]);

    let mut bag = DiagnosticBag::new();
    validate_supertrait_chain(&mut bag, "A", &supertraits);

    assert!(
        bag.has_errors(),
        "two-level cycle must produce at least one error diagnostic"
    );
    let joined: Vec<String> = bag
        .diagnostics()
        .iter()
        .map(|d| d.message().to_string())
        .collect();
    let message = joined.join("\n");
    assert!(
        message.contains("cycle"),
        "diagnostic must mention `cycle`, got: {}",
        message
    );
    assert!(
        message.contains("A") && message.contains("B"),
        "diagnostic must list both A and B, got: {}",
        message
    );
}

// ── 4. Three-level cycle (A → B → C → A) ──────────────────────

#[test]
fn test_three_level_cycle() {
    // P : Q, Q : R, R : P — three-node transitive cycle.
    let mut supertraits: HashMap<String, Vec<String>> = HashMap::new();
    supertraits.insert("P".to_string(), vec!["Q".to_string()]);
    supertraits.insert("Q".to_string(), vec!["R".to_string()]);
    supertraits.insert("R".to_string(), vec!["P".to_string()]);

    let mut bag = DiagnosticBag::new();
    validate_supertrait_chain(&mut bag, "P", &supertraits);

    assert!(
        bag.has_errors(),
        "three-level cycle must produce at least one error diagnostic"
    );
    let joined: Vec<String> = bag
        .diagnostics()
        .iter()
        .map(|d| d.message().to_string())
        .collect();
    let message = joined.join("\n");
    for name in ["P", "Q", "R"] {
        assert!(
            message.contains(name),
            "diagnostic must include trait name `{}`, got: {}",
            name,
            message
        );
    }
}
