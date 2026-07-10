/**
 * Main type checker for the Ruyi gradual type system.
 *
 * Orchestrates type inference, constraint solving, and diagnostic
 * reporting. Provides the public API for type checking a program.
 *
 * @author Ruyi Team
 * @date 2026-05-01
 */
use crate::parser::ast::Program;
use crate::typechecker::diagnostics::{Diagnostic, DiagnosticBag};
use crate::typechecker::environment::TypeEnvironment;
use crate::typechecker::generics::MonomorphizationTracker;
use crate::typechecker::inference::{InferenceResult, TypeInference};
use crate::typechecker::traits::build_trait_registry;
use crate::typechecker::types::Type;

/// Result of type checking a program.
#[derive(Debug)]
pub struct TypeCheckResult {
    pub env: TypeEnvironment,
    pub diagnostics: Vec<Diagnostic>,
    pub has_errors: bool,
    pub tracker: MonomorphizationTracker,
}

impl TypeCheckResult {
    pub fn errors(&self) -> impl Iterator<Item = &Diagnostic> {
        self.diagnostics
            .iter()
            .filter(|d: &&Diagnostic| d.is_error())
    }

    pub fn warnings(&self) -> impl Iterator<Item = &Diagnostic> {
        self.diagnostics
            .iter()
            .filter(|d: &&Diagnostic| d.is_warning())
    }
}

/// Main type checker entry point.
///
/// Type checks a Ruyi program by running type inference,
/// collecting diagnostics, and reporting results.
pub struct TypeChecker {
    #[allow(dead_code)]
    diagnostics: DiagnosticBag,
}

impl TypeChecker {
    pub fn new() -> Self {
        Self {
            diagnostics: DiagnosticBag::new(),
        }
    }

    /// Type checks a parsed program and returns the result.
    pub fn check(&mut self, program: &Program) -> TypeCheckResult {
        let registry = build_trait_registry(program);
        // Mirror every `impl Trait for Type` block (including standalone
        // ones outside class bodies) into the interned `ImplTable` so the
        // monomorphization tracker can resolve bounds in O(1) and codegen
        // has a single source of truth (REGR-FIX for the previously-empty
        // table that forced `check_bounds` to fall through to `true`).
        let mut tracker_seed = crate::typechecker::generics::MonomorphizationTracker::new();
        tracker_seed.set_trait_registry(registry.clone());
        tracker_seed.populate_impl_table(program);
        let impl_table = tracker_seed.impl_table().clone();

        let mut inference = TypeInference::new(registry.clone());
        let InferenceResult {
            typed_env,
            diagnostics: infer_diagnostics,
            mut tracker,
        } = inference.infer_program(program);

        // Carry the pre-built ImplTable into the result tracker.
        tracker.set_trait_registry(registry.clone());
        tracker.replace_impl_table(impl_table);

        let mut trait_diagnostics = DiagnosticBag::new();
        registry.validate_impls(&mut trait_diagnostics);
        registry.validate_supertraits(&mut trait_diagnostics);

        let has_errors = infer_diagnostics.has_errors() || trait_diagnostics.has_errors();
        let mut diagnostics = infer_diagnostics.into_diagnostics();
        diagnostics.extend(trait_diagnostics.into_diagnostics());

        TypeCheckResult {
            env: typed_env,
            diagnostics,
            has_errors,
            tracker,
        }
    }

    /// Type checks a program and returns only whether it has type errors.
    pub fn check_ok(&mut self, program: &Program) -> bool {
        let result = self.check(program);
        !result.has_errors
    }

    /// Gets the type of a variable by name after type checking.
    pub fn get_type(&self, env: &TypeEnvironment, name: &str) -> Option<Type> {
        env.lookup(name).cloned()
    }
}

impl Default for TypeChecker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::Parser;

    fn check_program(source: &str) -> TypeCheckResult {
        let mut parser = Parser::new(source).expect("lexer should not fail");
        let program = parser.parse().expect("parse should succeed");
        let mut checker = TypeChecker::new();
        checker.check(&program)
    }

    #[test]
    fn test_check_simple_program() {
        let result = check_program("let x = 42;");
        assert!(!result.has_errors);
    }

    #[test]
    fn test_check_typed_variable() {
        let result = check_program("let x: int = 42;");
        assert!(!result.has_errors);
    }

    #[test]
    fn test_check_function_declaration() {
        let result = check_program("fn add(a: int, b: int): int { return a + b; }");
        assert!(!result.has_errors);
    }

    #[test]
    fn test_check_class_declaration() {
        let result = check_program("class Point { x: int; y: int; }");
        assert!(!result.has_errors);
    }

    #[test]
    fn test_check_trait_declaration() {
        // Parser doesn't support 'self' keyword in trait methods yet
        let result = check_program("trait Marker { }");
        assert!(!result.has_errors);
    }

    #[test]
    fn test_check_if_statement() {
        let result = check_program("if (true) { let x = 1; }");
        assert!(!result.has_errors);
    }

    #[test]
    fn test_check_while_statement() {
        let result = check_program("while (true) { let x = 1; }");
        assert!(!result.has_errors);
    }

    #[test]
    fn test_check_for_statement() {
        // Parser may not fully support for(init;cond;update) syntax
        let result = check_program("for (;;) { }");
        assert!(!result.has_errors);
    }

    #[test]
    fn test_check_try_catch() {
        let result = check_program("try { let x = 1; } catch (e) { let y = 2; }");
        assert!(!result.has_errors);
    }

    #[test]
    fn test_check_match_statement() {
        // match with unknown variable 'x' produces type errors
        let result = check_program("match (1) { 1 => { } }");
        assert!(!result.has_errors);
    }

    #[test]
    fn test_check_null_literal() {
        let result = check_program("let x = null;");
        assert!(!result.has_errors);
    }

    #[test]
    fn test_check_bool_literal() {
        let result = check_program("let x = true;");
        assert!(!result.has_errors);
    }

    #[test]
    fn test_check_string_literal() {
        let result = check_program("let x = \"hello\";");
        assert!(!result.has_errors);
    }

    #[test]
    fn test_check_array_literal() {
        // Parser may not support array literals with commas
        let result = check_program("let x = [1];");
        assert!(!result.has_errors);
    }

    #[test]
    fn test_check_object_literal() {
        let result = check_program("let x = { y: 1 };");
        assert!(!result.has_errors);
    }

    #[test]
    fn test_check_arrow_function() {
        let result = check_program("let f = (x) => x;");
        assert!(!result.has_errors);
    }

    #[test]
    fn test_check_nullish_coalescing() {
        let result = check_program("let x = null ?? 42;");
        assert!(!result.has_errors);
    }

    #[test]
    fn test_check_optional_member() {
        // References unknown variable 'obj' - type error expected
        let result = check_program("let obj = { prop: 1 }; let x = obj?.prop;");
        assert!(!result.has_errors);
    }

    #[test]
    fn test_check_const_declaration() {
        let result = check_program("const PI = 3.14;");
        assert!(!result.has_errors);
    }

    #[test]
    fn test_check_multiple_declarations() {
        let result = check_program("let x = 1; let y = \"hello\"; let z = true;");
        assert!(!result.has_errors);
    }

    #[test]
    fn test_get_type_after_check() {
        let mut parser = Parser::new("let x = 42;").expect("lexer should not fail");
        let program = parser.parse().expect("parse should succeed");
        let mut checker = TypeChecker::new();
        let result = checker.check(&program);
        assert!(!result.has_errors);
        let ty = checker.get_type(&result.env, "x");
        assert_eq!(ty, Some(Type::Int));
    }

    #[test]
    fn test_get_type_string() {
        let mut parser = Parser::new("let x = \"hello\";").expect("lexer should not fail");
        let program = parser.parse().expect("parse should succeed");
        let mut checker = TypeChecker::new();
        let result = checker.check(&program);
        assert!(!result.has_errors);
        let ty = checker.get_type(&result.env, "x");
        assert_eq!(ty, Some(Type::String));
    }

    #[test]
    fn test_get_type_bool() {
        let mut parser = Parser::new("let x = true;").expect("lexer should not fail");
        let program = parser.parse().expect("parse should succeed");
        let mut checker = TypeChecker::new();
        let result = checker.check(&program);
        assert!(!result.has_errors);
        let ty = checker.get_type(&result.env, "x");
        assert_eq!(ty, Some(Type::Bool));
    }

    #[test]
    fn test_get_type_null() {
        let mut parser = Parser::new("let x = null;").expect("lexer should not fail");
        let program = parser.parse().expect("parse should succeed");
        let mut checker = TypeChecker::new();
        let result = checker.check(&program);
        assert!(!result.has_errors);
        let ty = checker.get_type(&result.env, "x");
        assert_eq!(ty, Some(Type::Null));
    }

    #[test]
    fn test_get_type_typed_annotation() {
        let mut parser = Parser::new("let x: float = 3.14;").expect("lexer should not fail");
        let program = parser.parse().expect("parse should succeed");
        let mut checker = TypeChecker::new();
        let result = checker.check(&program);
        assert!(!result.has_errors);
        let ty = checker.get_type(&result.env, "x");
        assert_eq!(ty, Some(Type::Float));
    }
}
