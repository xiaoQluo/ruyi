/**
 * Supertrait cycle detection for the Ruyi type checker.
 *
 * Uses DFS white/gray/black coloring to detect transitive cycles
 * in the supertrait chain. Emits a diagnostic with the full cycle path
 * via `validate_supertrait_chain`.
 *
 * Spec ref: specs/supertraits/spec.md, tasks.md Section 1.1
 *
 * @author Ruyi Team
 * @date 2026-07-11
 */
use std::collections::HashMap;

use crate::typechecker::diagnostics::{Diagnostic, DiagnosticBag, DiagnosticKind};

/// DFS node color: White = unvisited, Gray = on current path, Black = fully visited.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Color {
    White,
    Gray,
    Black,
}

/// Detects cycles in supertrait chains via DFS 3-coloring.
///
/// Starting from `trait_name`, follows the `supertraits` adjacency map.
/// Returns `Err(cycle_path)` on the first back edge found, where the path
/// is the sequence of trait names from the start through the cycle back to
/// the duplicate (inclusive), formatted for human consumption.
///
/// Unknown traits (those that appear as supertraits but are absent from the
/// map) are treated as leaves — they cannot contribute to a cycle because
/// they have no outgoing edges we know about.
pub fn detect_supertrait_cycle(
    trait_name: &str,
    supertraits: &HashMap<String, Vec<String>>,
) -> Result<(), Vec<String>> {
    let mut colors: HashMap<&str, Color> = supertraits
        .keys()
        .map(|k| (k.as_str(), Color::White))
        .collect();
    colors.insert(trait_name, Color::White);

    let mut path: Vec<String> = Vec::new();
    path.push(trait_name.to_string());

    fn dfs<'a>(
        node: &'a str,
        supertraits: &'a HashMap<String, Vec<String>>,
        colors: &mut HashMap<&'a str, Color>,
        path: &mut Vec<String>,
    ) -> Option<Vec<String>> {
        colors.insert(node, Color::Gray);
        if let Some(sups) = supertraits.get(node) {
            for sup in sups {
                match colors.get(sup.as_str()).copied() {
                    Some(Color::White) => {
                        path.push(sup.clone());
                        if let Some(cycle) = dfs(sup.as_str(), supertraits, colors, path) {
                            return Some(cycle);
                        }
                        path.pop();
                    }
                    Some(Color::Gray) => {
                        // Back edge → cycle. Extract from current path.
                        let start = path.iter().position(|n| n == sup).unwrap_or(0);
                        let mut cycle: Vec<String> = path[start..].to_vec();
                        cycle.push(sup.clone());
                        return Some(cycle);
                    }
                    Some(Color::Black) | None => {
                        // Cross edge / unknown trait — cannot participate in a back edge.
                    }
                }
            }
        }
        colors.insert(node, Color::Black);
        None
    }

    if let Some(cycle) = dfs(trait_name, supertraits, &mut colors, &mut path) {
        return Err(cycle);
    }
    Ok(())
}

/// High-level entry point. Validates the supertrait chain reachable from
/// `trait_name` and pushes an error diagnostic into `bag` if a cycle is
/// detected. The diagnostic message lists the full cycle path.
///
/// This is the function `TraitRegistry::validate_supertraits` calls for
/// each registered trait.
pub fn validate_supertrait_chain(
    bag: &mut DiagnosticBag,
    trait_name: &str,
    supertraits_map: &HashMap<String, Vec<String>>,
) {
    if let Err(cycle) = detect_supertrait_cycle(trait_name, supertraits_map) {
        let cycle_str = cycle.join(" -> ");
        bag.add(Diagnostic::error(DiagnosticKind::Other {
            message: format!("supertrait cycle detected: {}", cycle_str),
        }));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn map_of(pairs: &[(&str, &[&str])]) -> HashMap<String, Vec<String>> {
        pairs
            .iter()
            .map(|(k, v)| {
                (
                    (*k).to_string(),
                    v.iter().map(|s| (*s).to_string()).collect(),
                )
            })
            .collect()
    }

    #[test]
    fn empty_chain_is_acyclic() {
        let m = map_of(&[("A", &[])]);
        assert!(detect_supertrait_cycle("A", &m).is_ok());
    }

    #[test]
    fn linear_chain_is_acyclic() {
        let m = map_of(&[("A", &["B"]), ("B", &["C"]), ("C", &[])]);
        assert!(detect_supertrait_cycle("A", &m).is_ok());
    }

    #[test]
    fn self_loop_detected() {
        let m = map_of(&[("A", &["A"])]);
        let err = detect_supertrait_cycle("A", &m).unwrap_err();
        assert!(err.contains(&"A".to_string()));
    }

    #[test]
    fn two_node_cycle_detected() {
        let m = map_of(&[("A", &["B"]), ("B", &["A"])]);
        let err = detect_supertrait_cycle("A", &m).unwrap_err();
        assert!(err.contains(&"A".to_string()));
        assert!(err.contains(&"B".to_string()));
    }

    #[test]
    fn three_node_cycle_lists_all() {
        let m = map_of(&[("P", &["Q"]), ("Q", &["R"]), ("R", &["P"])]);
        let err = detect_supertrait_cycle("P", &m).unwrap_err();
        for name in ["P", "Q", "R"] {
            assert!(err.iter().any(|n| n == name), "missing {} in {:?}", name, err);
        }
    }

    #[test]
    fn unknown_supertrait_treated_as_leaf() {
        // `B` is not in the map → unknown leaf, cannot cycle.
        let m = map_of(&[("A", &["B"])]);
        assert!(detect_supertrait_cycle("A", &m).is_ok());
    }
}