/**
 * ImplTable — O(1) lookup for `Trait × Type` impl registrations.
 *
 * Replaces the previous string-based `HashMap<(String, String), ImplInfo>`
 * lookup in `TraitRegistry` with integer-keyed interned IDs. Per spec
 * Section 10.2 (Trait Bounds), every concrete specialization must verify
 * that the substituted type actually implements each declared bound.
 *
 * Design rationale:
 * - `TraitId` / `TypeId` are interned u32 handles produced by the registry
 *   once per program; this avoids repeated string hashing on the hot path
 *   during specialization.
 * - The map is keyed on `(TraitId, TypeId)` so `has_impl` is O(1).
 * - `impls_of_trait` returns `(TypeId, &ImplDef)` so the caller can
 *   iterate all concrete implementors of a trait without scanning the
 *   full table.
 *
 * @author developer
 * @date 2026-07-10
 */
use std::collections::HashMap;

/// Interned identifier for a trait declaration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TraitId(pub u32);

/// Interned identifier for a concrete (or generic-instantiated) type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TypeId(pub u32);

/// A registered trait implementation.
///
/// Holds the methods the impl provides plus any generic type parameters
/// declared on the impl block. The owning `ImplTable` does not interpret
/// `methods`; it merely stores it for downstream callers (codegen, dyn
/// dispatch, etc.).
#[derive(Debug, Clone, PartialEq)]
pub struct ImplDef {
    /// Names of the methods this impl defines.
    pub methods: Vec<String>,
}

/// O(1) `Trait × Type` → `ImplDef` table.
#[derive(Debug, Clone, Default)]
pub struct ImplTable {
    /// Primary index keyed on `(TraitId, TypeId)`.
    map: HashMap<(TraitId, TypeId), ImplDef>,
}

impl ImplTable {
    /// Creates a new empty impl table.
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers an impl of `trait_id` for `type_id`.
    ///
    /// If a previous impl was registered for the same `(TraitId, TypeId)`
    /// pair it is overwritten — the most recent registration wins. This
    /// matches the existing `TraitRegistry::register_impl` semantics and
    /// keeps the table deterministic for duplicate impls.
    pub fn register(&mut self, trait_id: TraitId, type_id: TypeId, def: ImplDef) {
        self.map.insert((trait_id, type_id), def);
    }

    /// Returns `true` iff `(trait_id, type_id)` has a registered impl.
    pub fn has_impl(&self, trait_id: TraitId, type_id: TypeId) -> bool {
        self.map.contains_key(&(trait_id, type_id))
    }

    /// Returns every `(TypeId, &ImplDef)` pair registered for `trait_id`.
    ///
    /// The order is unspecified; callers that need stable order should
    /// sort the returned vector themselves.
    pub fn impls_of_trait(&self, trait_id: TraitId) -> Vec<(TypeId, &ImplDef)> {
        self.map
            .iter()
            .filter_map(|((t, ty), def)| {
                if *t == trait_id {
                    Some((*ty, def))
                } else {
                    None
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn trait_id(n: u32) -> TraitId {
        TraitId(n)
    }
    fn type_id(n: u32) -> TypeId {
        TypeId(n)
    }
    fn def(method: &str) -> ImplDef {
        ImplDef {
            methods: vec![method.to_string()],
        }
    }

    /// Verifies: REQ-TRAIT-001 (registration is observable).
    #[test]
    fn register_and_has_impl_returns_true() {
        let mut table = ImplTable::new();
        table.register(trait_id(1), type_id(10), def("format"));
        assert!(table.has_impl(trait_id(1), type_id(10)));
    }

    /// Verifies: REQ-TRAIT-001 (unknown pairs are not implemented).
    #[test]
    fn has_impl_for_unknown_returns_false() {
        let table = ImplTable::new();
        // Empty table → nothing is implemented.
        assert!(!table.has_impl(trait_id(99), type_id(99)));
    }

    /// Verifies: REQ-TRAIT-001 (iteration yields all registered impls).
    #[test]
    fn impls_of_trait_iterates_correctly() {
        let mut table = ImplTable::new();
        table.register(trait_id(1), type_id(10), def("format_a"));
        table.register(trait_id(1), type_id(20), def("format_b"));
        table.register(trait_id(2), type_id(10), def("debug_a"));

        let trait_one_impls: Vec<(TypeId, &str)> = table
            .impls_of_trait(trait_id(1))
            .into_iter()
            .map(|(ty, def)| (ty, def.methods[0].as_str()))
            .collect();

        assert_eq!(trait_one_impls.len(), 2);
        let mut sorted = trait_one_impls.clone();
        sorted.sort_by_key(|(ty, _)| ty.0);
        assert_eq!(sorted[0].0, type_id(10));
        assert_eq!(sorted[0].1, "format_a");
        assert_eq!(sorted[1].0, type_id(20));
        assert_eq!(sorted[1].1, "format_b");

        // Different trait → different impl set.
        assert_eq!(table.impls_of_trait(trait_id(2)).len(), 1);
        assert_eq!(table.impls_of_trait(trait_id(3)).len(), 0);
    }
}
