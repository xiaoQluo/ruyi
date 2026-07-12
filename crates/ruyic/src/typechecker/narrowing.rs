/**
 * Type narrowing for the Ruyi type checker.
 *
 * Implements the tri-state narrowing environment used by the type checker
 * to track per-branch type facts after `if` / `else` / `match` / `instanceof`
 * / `typeof` predicates. The narrow environment flows through
 * `TypeInference::infer_statement` and lets the `else` branch observe the
 * "reverse" (widened) type of variables that were positively narrowed in
 * the `if` branch.
 *
 * Spec: `changes/v0.5.7-p1-defects/specs/narrowing/spec.md` (item 3.4).
 *
 * @author Ruyi Team
 * @date 2026-07-11
 */
use crate::typechecker::types::Type;

/// Tri-state narrowing environment used during per-branch inference.
///
/// - `Narrowed(T)` — the variable has been positively narrowed to `T`
///   in the current branch (e.g. `if (x !== null) { ... }`).
/// - `Widened(T)` — the variable has been reverse-narrowed to its
///   original (possibly nullable) type `T` in the `else` branch.
/// - `Unknown`    — no narrowing information is available for this
///   variable in the current scope.
#[derive(Debug, Clone, PartialEq, Default)]
pub enum NarrowEnv {
    /// Positive narrowing: variable is `T` in this branch.
    Narrowed(Type),
    /// Reverse narrowing: variable is the original `T` (possibly `T?`)
    /// in the `else` branch.
    Widened(Type),
    /// No narrowing fact recorded yet.
    #[default]
    Unknown,
}

impl NarrowEnv {
    /// Applies the recorded narrowing to `original`, producing the
    /// type the variable should have in the current branch.
    ///
    /// - `Narrowed(t)` / `Widened(t)` ⇒ `t.clone()`
    /// - `Unknown` ⇒ `original.clone()` (fallback to declared type)
    pub fn apply_to(&self, original: &Type) -> Type {
        match self {
            NarrowEnv::Narrowed(t) | NarrowEnv::Widened(t) => t.clone(),
            NarrowEnv::Unknown => original.clone(),
        }
    }
}

/// Apply reverse narrowing for the `else` branch after a positive
/// condition such as `x !== null`.
///
/// When the `if` branch positively narrowed a variable from `T?` to
/// `T`, the `else` branch must see it widened back to `T?` so the
/// `!` operator can still be used to assert non-null. This helper
/// records that fact on `env` if no narrowing has been recorded yet.
///
/// `narrowed_ty` is the type the variable had inside the `if` branch
/// (recorded for future diagnostics; not currently used in the type
/// itself but kept so callers can attach diagnostic context without
/// recomputing the original type).
pub fn apply_reverse_narrow(env: &mut NarrowEnv, original_ty: &Type, narrowed_ty: &Type) {
    if matches!(env, NarrowEnv::Unknown) {
        *env = NarrowEnv::Widened(original_ty.clone());
        let _ = narrowed_ty; // recorded for diagnostics; silence unused warning
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_falls_back_to_original() {
        let env = NarrowEnv::Unknown;
        let original = Type::Int;
        assert_eq!(env.apply_to(&original), Type::Int);
    }

    #[test]
    fn narrowed_returns_narrowed_type() {
        let env = NarrowEnv::Narrowed(Type::Int);
        let original = Type::Dynamic;
        assert_eq!(env.apply_to(&original), Type::Int);
    }

    #[test]
    fn widened_returns_widened_type() {
        let env = NarrowEnv::Widened(Type::Nullable(Box::new(Type::String)));
        let original = Type::Dynamic;
        assert_eq!(
            env.apply_to(&original),
            Type::Nullable(Box::new(Type::String))
        );
    }

    #[test]
    fn apply_reverse_narrow_moves_unknown_to_widened() {
        let mut env = NarrowEnv::Unknown;
        let original = Type::Nullable(Box::new(Type::Int));
        let narrowed = Type::Int;
        apply_reverse_narrow(&mut env, &original, &narrowed);
        assert_eq!(env, NarrowEnv::Widened(Type::Nullable(Box::new(Type::Int))));
    }

    #[test]
    fn apply_reverse_narrow_does_not_overwrite_existing() {
        let mut env = NarrowEnv::Narrowed(Type::Int);
        let original = Type::Nullable(Box::new(Type::Int));
        let narrowed = Type::Int;
        apply_reverse_narrow(&mut env, &original, &narrowed);
        // Existing Narrowed is preserved (not overwritten).
        assert_eq!(env, NarrowEnv::Narrowed(Type::Int));
    }
}
