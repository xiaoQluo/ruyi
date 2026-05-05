/**
 * Type environment for the Ruyi gradual type checker.
 *
 * Implements scoped variable-to-type mappings with support for
 * nested scopes, type narrowing, and const/mutability tracking.
 *
 * @author Ruyi Team
 * @date 2026-05-01
 */
use crate::typechecker::types::Type;

/// Binding info for a variable in the type environment.
#[derive(Debug, Clone, PartialEq)]
pub struct Binding {
    pub name: String,
    pub ty: Type,
    pub mutable: bool,
}

/// A single scope level in the type environment.
#[derive(Debug, Clone)]
pub struct Scope {
    bindings: Vec<Binding>,
}

impl Scope {
    fn new() -> Self {
        Self {
            bindings: Vec::new(),
        }
    }

    fn declare(&mut self, binding: Binding) {
        self.bindings.push(binding);
    }

    fn lookup(&self, name: &str) -> Option<&Binding> {
        self.bindings.iter().find(|b| b.name == name)
    }

    fn lookup_mut(&mut self, name: &str) -> Option<&mut Binding> {
        self.bindings.iter_mut().find(|b| b.name == name)
    }
}

/// Type environment: a chain of scopes mapping variable names to types.
///
/// Supports push/pop for block scoping, variable declaration,
/// lookup with shadowing, and type narrowing for control flow.
#[derive(Debug, Clone)]
pub struct TypeEnvironment {
    scopes: Vec<Scope>,
}

impl TypeEnvironment {
    /// Creates a new empty type environment with a single global scope.
    pub fn new() -> Self {
        Self {
            scopes: vec![Scope::new()],
        }
    }

    /// Enters a new nested scope.
    pub fn push_scope(&mut self) {
        self.scopes.push(Scope::new());
    }

    /// Exits the current scope, discarding all bindings in it.
    pub fn pop_scope(&mut self) {
        if self.scopes.len() > 1 {
            self.scopes.pop();
        }
    }

    /// Declares a variable in the current scope.
    pub fn declare(&mut self, name: &str, ty: Type, mutable: bool) {
        let scope = self.scopes.last_mut().expect("at least one scope");
        scope.declare(Binding {
            name: name.to_string(),
            ty,
            mutable,
        });
    }

    /// Declares a `let` (mutable) variable.
    pub fn declare_let(&mut self, name: &str, ty: Type) {
        self.declare(name, ty, true);
    }

    /// Declares a `const` (immutable) variable.
    pub fn declare_const(&mut self, name: &str, ty: Type) {
        self.declare(name, ty, false);
    }

    /// Looks up a variable's type, searching from innermost to outermost scope.
    pub fn lookup(&self, name: &str) -> Option<&Type> {
        for scope in self.scopes.iter().rev() {
            if let Some(binding) = scope.lookup(name) {
                return Some(&binding.ty);
            }
        }
        None
    }

    /// Looks up whether a variable is mutable.
    pub fn is_mutable(&self, name: &str) -> Option<bool> {
        for scope in self.scopes.iter().rev() {
            if let Some(binding) = scope.lookup(name) {
                return Some(binding.mutable);
            }
        }
        None
    }

    /// Narrows the type of a variable in the current scope.
    ///
    /// Used for control-flow type narrowing (e.g., after `if (x !== null)`).
    /// Creates a shadow binding in the current scope with the narrowed type.
    pub fn narrow(&mut self, name: &str, narrowed_ty: Type) {
        let mutable = self.is_mutable(name).unwrap_or(true);
        let scope = self.scopes.last_mut().expect("at least one scope");
        if let Some(binding) = scope.lookup_mut(name) {
            binding.ty = narrowed_ty;
        } else {
            // Variable was declared in an outer scope; shadow it in current scope
            scope.declare(Binding {
                name: name.to_string(),
                ty: narrowed_ty,
                mutable,
            });
        }
    }

    /// Updates the type of an existing variable (for re-assignment).
    /// Returns `false` if the variable is immutable or not found.
    pub fn update(&mut self, name: &str, new_ty: Type) -> bool {
        for scope in self.scopes.iter_mut().rev() {
            if let Some(binding) = scope.lookup_mut(name) {
                if binding.mutable {
                    binding.ty = binding.ty.least_upper_bound(&new_ty);
                    return true;
                }
                return false;
            }
        }
        false
    }

    /// Returns the number of active scopes (for debugging/testing).
    pub fn scope_depth(&self) -> usize {
        self.scopes.len()
    }

    /// Declares a function parameter in the current scope.
    pub fn declare_param(&mut self, name: &str, ty: Type) {
        self.declare_let(name, ty);
    }
}

impl Default for TypeEnvironment {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_declare_and_lookup() {
        let mut env = TypeEnvironment::new();
        env.declare_let("x", Type::Int);
        assert_eq!(env.lookup("x"), Some(&Type::Int));
    }

    #[test]
    fn test_const_immutable() {
        let mut env = TypeEnvironment::new();
        env.declare_const("PI", Type::Float);
        assert_eq!(env.is_mutable("PI"), Some(false));
        assert!(!env.update("PI", Type::Int));
    }

    #[test]
    fn test_let_mutable() {
        let mut env = TypeEnvironment::new();
        env.declare_let("x", Type::Int);
        assert_eq!(env.is_mutable("x"), Some(true));
        assert!(env.update("x", Type::Float));
        assert_eq!(env.lookup("x"), Some(&Type::Float));
    }

    #[test]
    fn test_scope_push_pop() {
        let mut env = TypeEnvironment::new();
        env.declare_let("x", Type::Int);
        env.push_scope();
        env.declare_let("y", Type::String);
        assert_eq!(env.lookup("y"), Some(&Type::String));
        assert_eq!(env.lookup("x"), Some(&Type::Int));
        env.pop_scope();
        assert_eq!(env.lookup("y"), None);
        assert_eq!(env.lookup("x"), Some(&Type::Int));
    }

    #[test]
    fn test_shadowing() {
        let mut env = TypeEnvironment::new();
        env.declare_let("x", Type::Int);
        env.push_scope();
        env.declare_let("x", Type::String);
        assert_eq!(env.lookup("x"), Some(&Type::String));
        env.pop_scope();
        assert_eq!(env.lookup("x"), Some(&Type::Int));
    }

    #[test]
    fn test_narrowing() {
        let mut env = TypeEnvironment::new();
        env.declare_let("x", Type::Nullable(Box::new(Type::String)));
        env.push_scope();
        env.narrow("x", Type::String);
        assert_eq!(env.lookup("x"), Some(&Type::String));
        env.pop_scope();
        assert_eq!(
            env.lookup("x"),
            Some(&Type::Nullable(Box::new(Type::String)))
        );
    }

    #[test]
    fn test_scope_depth() {
        let mut env = TypeEnvironment::new();
        assert_eq!(env.scope_depth(), 1);
        env.push_scope();
        assert_eq!(env.scope_depth(), 2);
        env.pop_scope();
        assert_eq!(env.scope_depth(), 1);
    }

    #[test]
    fn test_lookup_unknown() {
        let env = TypeEnvironment::new();
        assert_eq!(env.lookup("unknown"), None);
    }
}
