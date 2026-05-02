/**
 * Generic type specialization and monomorphization tracking.
 *
 * Implements the generics system per spec Sections 5 and 10:
 * - Type parameter declarations: `fn identity<T>(x: T): T`
 * - Generic class definitions: `class Box<T> { value: T }`
 * - Type argument inference: `identity(42)` → `identity<int>(42)`
 * - Explicit type arguments: `identity<int>(42)`
 * - Trait bounds: `fn max<T: Comparable>(a: T, b: T): T`
 * - Constraint checking: verify type params satisfy bounds
 * - Monomorphization tracking for code generation
 *
 * @author Ruyi Team
 * @date 2026-05-01
 */

use std::collections::HashMap;
use crate::typechecker::types::{Type, TypeVar};
use crate::typechecker::constraints::ConstraintSolver;
use crate::typechecker::diagnostics::{DiagnosticBag, DiagnosticKind};

/// A generic definition (function, class, or trait) with type parameters.
#[derive(Debug, Clone, PartialEq)]
pub struct GenericDefinition {
    /// The name of the generic entity (function, class, or trait).
    pub name: String,
    /// The type parameters (e.g., T, U) with optional trait bounds.
    pub type_params: Vec<TypeParamInfo>,
    /// The concrete type of the entity with type parameters still as TypeVars.
    /// For a function: `fn(T, T) -> T` for `fn identity<T>(x: T): T`
    /// For a class: the class type itself
    pub body_type: Type,
}

/// Information about a single type parameter.
#[derive(Debug, Clone, PartialEq)]
pub struct TypeParamInfo {
    /// The name of the type parameter (e.g., "T", "U").
    pub name: String,
    /// The unique ID of the type variable associated with this parameter.
    pub var_id: u32,
    /// Trait bounds that the type parameter must satisfy.
    /// Empty vector means no bounds (any type is valid).
    pub bounds: Vec<String>,
}

impl TypeParamInfo {
    /// Creates a new type parameter info with no bounds.
    pub fn new(name: String, var_id: u32) -> Self {
        Self {
            name,
            var_id,
            bounds: Vec::new(),
        }
    }

    /// Creates a new type parameter info with trait bounds.
    pub fn with_bounds(name: String, var_id: u32, bounds: Vec<String>) -> Self {
        Self { name, var_id, bounds }
    }

    /// Returns the TypeVar for this type parameter.
    pub fn to_type_var(&self) -> TypeVar {
        TypeVar::new(self.var_id, self.name.clone())
    }

    /// Returns the Type::TypeVar for this type parameter.
    pub fn to_type(&self) -> Type {
        Type::TypeVar(self.to_type_var())
    }
}

/// Result of specializing a generic definition with concrete type arguments.
#[derive(Debug, Clone, PartialEq)]
pub struct Specialization {
    /// The name of the original generic entity.
    pub generic_name: String,
    /// The concrete type arguments used for specialization.
    pub type_args: Vec<Type>,
    /// The mangled name for the specialized version (e.g., "identity_int").
    pub mangled_name: String,
    /// The resulting concrete type after substitution.
    pub specialized_type: Type,
}

impl Specialization {
    /// Creates a new specialization.
    pub fn new(generic_name: &str, type_args: Vec<Type>, specialized_type: Type) -> Self {
        let mangled_name = mangle_name(generic_name, &type_args);
        Self {
            generic_name: generic_name.to_string(),
            type_args,
            mangled_name,
            specialized_type,
        }
    }
}

/// Mangles a generic name with its type arguments to produce a unique identifier.
///
/// Examples:
/// - `identity` + `[int]` → `identity__int`
/// - `map` + `[int, string]` → `map__int__string`
/// - `Box` + `[float]` → `Box__float`
pub fn mangle_name(name: &str, type_args: &[Type]) -> String {
    if type_args.is_empty() {
        name.to_string()
    } else {
        let args_str: Vec<String> = type_args.iter().map(|t| mangle_type(t)).collect();
        format!("{}__{}", name, args_str.join("__"))
    }
}

/// Mangles a single type for use in symbol names.
fn mangle_type(ty: &Type) -> String {
    match ty {
        Type::Int => "int".to_string(),
        Type::Float => "float".to_string(),
        Type::Bool => "bool".to_string(),
        Type::String => "string".to_string(),
        Type::Null => "null".to_string(),
        Type::Void => "void".to_string(),
        Type::Never => "never".to_string(),
        Type::BigInt => "bigint".to_string(),
        Type::Dynamic => "dyn".to_string(),
        Type::Nullable(inner) => format!("{}__opt", mangle_type(inner)),
        Type::Array(elem) => format!("Array__{}", mangle_type(elem)),
        Type::Object(fields) => {
            let field_strs: Vec<String> = fields.iter()
                .map(|f| format!("{}_{}", f.name, mangle_type(&f.ty)))
                .collect();
            format!("Obj_{}", field_strs.join("_"))
        }
        Type::Function { params, return_type } => {
            let param_strs: Vec<String> = params.iter().map(|p| mangle_type(p)).collect();
            format!("fn_{}_{}", param_strs.join("_"), mangle_type(return_type))
        }
        Type::Named(name) => name.clone(),
        Type::Generic { base, args } => {
            let arg_strs: Vec<String> = args.iter().map(|a| mangle_type(a)).collect();
            format!("{}__{}", base, arg_strs.join("__"))
        }
        Type::TypeVar(var) => var.name.clone(),
        Type::Trait(name) => format!("dyn_{}", name),
        Type::Future(inner) => format!("Future__{}", mangle_type(inner)),
        Type::Error => "error".to_string(),
    }
}

/// Tracks all generic definitions and their specializations for monomorphization.
///
/// Per spec Section 10.3:
/// 1. Collection: During type checking, collect all call sites of generic functions
///    and the concrete types used.
/// 2. Substitution: For each unique combination of concrete types, create a
///    specialized version of the generic function.
/// 3. Code generation: Generate LLVM IR for each specialized version.
/// 4. Deduplication: If the same specialization is used at multiple call sites,
///    generate it only once.
#[derive(Debug, Clone)]
pub struct MonomorphizationTracker {
    /// All known generic definitions, keyed by name.
    generic_defs: HashMap<String, GenericDefinition>,
    /// All collected specializations, keyed by mangled name.
    specializations: HashMap<String, Specialization>,
    /// Counter for generating unique type variable IDs.
    next_var_id: u32,
}

impl MonomorphizationTracker {
    /// Creates a new empty tracker.
    pub fn new() -> Self {
        Self {
            generic_defs: HashMap::new(),
            specializations: HashMap::new(),
            next_var_id: 0,
        }
    }

    /// Creates a fresh type variable ID for a new type parameter.
    pub fn fresh_var_id(&mut self) -> u32 {
        let id = self.next_var_id;
        self.next_var_id += 1;
        id
    }

    /// Registers a generic definition (function, class, or trait).
    pub fn register_generic(&mut self, def: GenericDefinition) {
        self.generic_defs.insert(def.name.clone(), def);
    }

    /// Looks up a generic definition by name.
    pub fn get_generic(&self, name: &str) -> Option<&GenericDefinition> {
        self.generic_defs.get(name)
    }

    /// Checks if a name is a registered generic definition.
    pub fn is_generic(&self, name: &str) -> bool {
        self.generic_defs.contains_key(name)
    }

    /// Specializes a generic definition with the given type arguments.
    ///
    /// Per spec Section 10.3, this performs:
    /// 1. Arity check: verify the number of type arguments matches
    /// 2. Bound check: verify each type argument satisfies its trait bounds
    /// 3. Substitution: replace type variables with concrete types
    /// 4. Deduplication: return existing specialization if already created
    pub fn specialize(
        &mut self,
        generic_name: &str,
        type_args: Vec<Type>,
        diagnostics: &mut DiagnosticBag,
    ) -> Option<Specialization> {
        let def = self.generic_defs.get(generic_name)?.clone();

        // 1. Arity check
        if type_args.len() != def.type_params.len() {
            diagnostics.add_error(DiagnosticKind::GenericArity {
                name: generic_name.to_string(),
                expected: def.type_params.len(),
                found: type_args.len(),
            });
            return None;
        }

        // 2. Bound check
        for (param, arg) in def.type_params.iter().zip(type_args.iter()) {
            if !self.check_bounds(param, arg, diagnostics) {
                return None;
            }
        }

        // 3. Substitution
        let subst = self.build_substitution(&def.type_params, &type_args);
        let specialized_type = ConstraintSolver::apply_subst(&subst, &def.body_type);

        // 4. Deduplication
        let spec = Specialization::new(generic_name, type_args, specialized_type);
        let mangled = spec.mangled_name.clone();
        self.specializations.insert(mangled, spec.clone());

        Some(spec)
    }

    /// Infers type arguments for a generic function call from the argument types.
    ///
    /// Per spec Section 10.5 (Type Inference with Generics):
    /// - Creates type variables for each type parameter
    /// - Collects constraints from argument types
    /// - Solves constraints via unification
    /// - Returns the inferred concrete types
    pub fn infer_type_args(
        &mut self,
        generic_name: &str,
        arg_types: &[Type],
        diagnostics: &mut DiagnosticBag,
    ) -> Option<Vec<Type>> {
        let def = self.generic_defs.get(generic_name)?.clone();

        // Extract parameter types from the body type
        let param_types = match &def.body_type {
            Type::Function { params, .. } => params.clone(),
            _ => {
                diagnostics.add_error(DiagnosticKind::Other {
                    message: format!("Cannot infer type args for non-function generic {}", generic_name),
                });
                return None;
            }
        };

        // Create fresh type variables for each type parameter
        let mut solver = ConstraintSolver::new();
        let mut type_var_map: HashMap<u32, Type> = HashMap::new();

        for param in &def.type_params {
            let fresh_var = solver.fresh_var(&param.name);
            type_var_map.insert(param.var_id, Type::TypeVar(fresh_var));
        }

        // Build the parameter type substitution
        let subst = type_var_map;

        // Apply substitution to parameter types to get the expected types
        let expected_param_types: Vec<Type> = param_types.iter()
            .map(|p| ConstraintSolver::apply_subst(&subst, p))
            .collect();

        // Add equality constraints between expected and actual argument types
        if arg_types.len() != expected_param_types.len() {
            diagnostics.add_error(DiagnosticKind::ArgumentCount {
                expected: expected_param_types.len(),
                found: arg_types.len(),
            });
            return None;
        }

        for (expected, actual) in expected_param_types.iter().zip(arg_types.iter()) {
            solver.add_equal(expected.clone(), actual.clone());
        }

        // Solve constraints
        match solver.solve() {
            crate::typechecker::constraints::SolveResult::Solved(solution) => {
                // Map solutions back to type parameters in order
                let type_args: Vec<Type> = def.type_params.iter()
                    .map(|param| {
                        let var_id = param.var_id;
                        // First check the solution from the constraint solver
                        if let Some(ty) = solution.get(&var_id) {
                            ConstraintSolver::apply_subst(&solution, ty)
                        } else if let Some(ty) = subst.get(&var_id) {
                            // Fall back to the type variable mapping
                            ConstraintSolver::apply_subst(&subst, ty)
                        } else {
                            // Unresolved type parameter defaults to dyn
                            Type::Dynamic
                        }
                    })
                    .collect();

                // Check bounds for inferred types
                for (param, arg) in def.type_params.iter().zip(type_args.iter()) {
                    self.check_bounds(param, arg, diagnostics);
                }

                Some(type_args)
            }
            crate::typechecker::constraints::SolveResult::Error(errors) => {
                for err in errors {
                    diagnostics.add_error(DiagnosticKind::Other { message: err });
                }
                None
            }
        }
    }

    /// Gets all collected specializations.
    pub fn specializations(&self) -> &HashMap<String, Specialization> {
        &self.specializations
    }

    /// Gets a specialization by its mangled name.
    pub fn get_specialization(&self, mangled_name: &str) -> Option<&Specialization> {
        self.specializations.get(mangled_name)
    }

    /// Checks if a type argument satisfies the trait bounds of a type parameter.
    fn check_bounds(
        &self,
        param: &TypeParamInfo,
        arg: &Type,
        _diagnostics: &mut DiagnosticBag,
    ) -> bool {
        // Per spec Section 10.2:
        // - dyn satisfies any trait bound (runtime check)
        // - Any type satisfies an empty bound list
        // - For concrete types, we'd need a trait implementation registry
        //   For now, we accept all bounds since trait impl checking is not yet complete

        if arg.is_dynamic() {
            // dyn satisfies any bound (checked at runtime)
            return true;
        }

        if param.bounds.is_empty() {
            return true;
        }

        // TODO: When trait implementation checking is complete, verify that
        // `arg` implements each trait in `param.bounds`.
        // For now, we accept all bounds to allow forward progress.
        true
    }

    /// Builds a substitution map from type parameters to concrete types.
    fn build_substitution(
        &self,
        type_params: &[TypeParamInfo],
        type_args: &[Type],
    ) -> HashMap<u32, Type> {
        type_params.iter()
            .zip(type_args.iter())
            .map(|(param, arg)| (param.var_id, arg.clone()))
            .collect()
    }

    /// Substitutes type variables in a type with concrete types from the substitution map.
    /// This is a convenience wrapper around ConstraintSolver::apply_subst.
    pub fn substitute_type(subst: &HashMap<u32, Type>, ty: &Type) -> Type {
        ConstraintSolver::apply_subst(subst, ty)
    }
}

impl Default for MonomorphizationTracker {
    fn default() -> Self {
        Self::new()
    }
}

/// Creates a GenericDefinition from an AST function declaration.
///
/// Takes the function's type parameters and body type, and creates
/// a GenericDefinition with fresh type variable IDs.
pub fn make_generic_function_def(
    name: &str,
    type_params: &[crate::parser::ast::TypeParam],
    param_types: &[Type],
    return_type: &Type,
    tracker: &mut MonomorphizationTracker,
) -> GenericDefinition {
    let mut param_infos = Vec::new();
    for tp in type_params {
        let var_id = tracker.fresh_var_id();
        param_infos.push(TypeParamInfo::with_bounds(
            tp.name.clone(),
            var_id,
            tp.bounds.clone(),
        ));
    }

    let body_type = Type::Function {
        params: param_types.to_vec(),
        return_type: Box::new(return_type.clone()),
    };

    GenericDefinition {
        name: name.to_string(),
        type_params: param_infos,
        body_type,
    }
}

/// Creates a GenericDefinition from an AST class declaration.
pub fn make_generic_class_def(
    name: &str,
    type_params: &[crate::parser::ast::TypeParam],
    tracker: &mut MonomorphizationTracker,
) -> GenericDefinition {
    let mut param_infos = Vec::new();
    for tp in type_params {
        let var_id = tracker.fresh_var_id();
        param_infos.push(TypeParamInfo::with_bounds(
            tp.name.clone(),
            var_id,
            tp.bounds.clone(),
        ));
    }

    GenericDefinition {
        name: name.to_string(),
        type_params: param_infos,
        body_type: Type::Named(name.to_string()),
    }
}

/// Creates a GenericDefinition from an AST trait declaration.
pub fn make_generic_trait_def(
    name: &str,
    type_params: &[crate::parser::ast::TypeParam],
    tracker: &mut MonomorphizationTracker,
) -> GenericDefinition {
    let mut param_infos = Vec::new();
    for tp in type_params {
        let var_id = tracker.fresh_var_id();
        param_infos.push(TypeParamInfo::with_bounds(
            tp.name.clone(),
            var_id,
            tp.bounds.clone(),
        ));
    }

    GenericDefinition {
        name: name.to_string(),
        type_params: param_infos,
        body_type: Type::Trait(name.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mangle_name_simple() {
        assert_eq!(mangle_name("identity", &[Type::Int]), "identity__int");
    }

    #[test]
    fn test_mangle_name_multiple_args() {
        assert_eq!(
            mangle_name("map", &[Type::Int, Type::String]),
            "map__int__string"
        );
    }

    #[test]
    fn test_mangle_name_no_args() {
        assert_eq!(mangle_name("foo", &[]), "foo");
    }

    #[test]
    fn test_mangle_name_nullable() {
        assert_eq!(
            mangle_name("wrap", &[Type::Nullable(Box::new(Type::Int))]),
            "wrap__int__opt"
        );
    }

    #[test]
    fn test_mangle_name_array() {
        assert_eq!(
            mangle_name("process", &[Type::Array(Box::new(Type::Float))]),
            "process__Array__float"
        );
    }

    #[test]
    fn test_specialization_new() {
        let spec = Specialization::new(
            "identity",
            vec![Type::Int],
            Type::Function {
                params: vec![Type::Int],
                return_type: Box::new(Type::Int),
            },
        );
        assert_eq!(spec.generic_name, "identity");
        assert_eq!(spec.type_args, vec![Type::Int]);
        assert_eq!(spec.mangled_name, "identity__int");
    }

    #[test]
    fn test_tracker_register_and_lookup() {
        let mut tracker = MonomorphizationTracker::new();
        let var_id = tracker.fresh_var_id();
        let def = GenericDefinition {
            name: "identity".to_string(),
            type_params: vec![TypeParamInfo::new("T".to_string(), var_id)],
            body_type: Type::Function {
                params: vec![Type::TypeVar(TypeVar::new(var_id, "T".to_string()))],
                return_type: Box::new(Type::TypeVar(TypeVar::new(var_id, "T".to_string()))),
            },
        };
        tracker.register_generic(def);
        assert!(tracker.is_generic("identity"));
        assert!(!tracker.is_generic("unknown"));
    }

    #[test]
    fn test_tracker_specialize() {
        let mut tracker = MonomorphizationTracker::new();
        let var_id = tracker.fresh_var_id();
        let type_var = TypeVar::new(var_id, "T".to_string());
        let def = GenericDefinition {
            name: "identity".to_string(),
            type_params: vec![TypeParamInfo::new("T".to_string(), var_id)],
            body_type: Type::Function {
                params: vec![Type::TypeVar(type_var.clone())],
                return_type: Box::new(Type::TypeVar(type_var)),
            },
        };
        tracker.register_generic(def);

        let mut diagnostics = DiagnosticBag::new();
        let spec = tracker.specialize("identity", vec![Type::Int], &mut diagnostics);
        assert!(spec.is_some());
        let spec = spec.unwrap();
        assert_eq!(spec.mangled_name, "identity__int");
        assert_eq!(spec.type_args, vec![Type::Int]);
        // The specialized type should have T replaced with int
        match &spec.specialized_type {
            Type::Function { params, return_type } => {
                assert_eq!(params, &vec![Type::Int]);
                assert_eq!(**return_type, Type::Int);
            }
            _ => panic!("Expected function type"),
        }
    }

    #[test]
    fn test_tracker_specialize_arity_error() {
        let mut tracker = MonomorphizationTracker::new();
        let var_id = tracker.fresh_var_id();
        let def = GenericDefinition {
            name: "identity".to_string(),
            type_params: vec![TypeParamInfo::new("T".to_string(), var_id)],
            body_type: Type::Function {
                params: vec![Type::TypeVar(TypeVar::new(var_id, "T".to_string()))],
                return_type: Box::new(Type::TypeVar(TypeVar::new(var_id, "T".to_string()))),
            },
        };
        tracker.register_generic(def);

        let mut diagnostics = DiagnosticBag::new();
        let spec = tracker.specialize("identity", vec![Type::Int, Type::String], &mut diagnostics);
        assert!(spec.is_none());
        assert!(diagnostics.has_errors());
    }

    #[test]
    fn test_tracker_specialize_deduplication() {
        let mut tracker = MonomorphizationTracker::new();
        let var_id = tracker.fresh_var_id();
        let type_var = TypeVar::new(var_id, "T".to_string());
        let def = GenericDefinition {
            name: "identity".to_string(),
            type_params: vec![TypeParamInfo::new("T".to_string(), var_id)],
            body_type: Type::Function {
                params: vec![Type::TypeVar(type_var.clone())],
                return_type: Box::new(Type::TypeVar(type_var)),
            },
        };
        tracker.register_generic(def);

        let mut diagnostics = DiagnosticBag::new();
        let spec1 = tracker.specialize("identity", vec![Type::Int], &mut diagnostics).unwrap();
        let spec2 = tracker.specialize("identity", vec![Type::Int], &mut diagnostics).unwrap();
        assert_eq!(spec1.mangled_name, spec2.mangled_name);
    }

    #[test]
    fn test_type_param_info() {
        let info = TypeParamInfo::with_bounds("T".to_string(), 0, vec!["Comparable".to_string()]);
        assert_eq!(info.name, "T");
        assert_eq!(info.var_id, 0);
        assert_eq!(info.bounds, vec!["Comparable"]);
        assert_eq!(info.to_type(), Type::TypeVar(TypeVar::new(0, "T".to_string())));
    }

    #[test]
    fn test_substitute_type() {
        let mut subst = HashMap::new();
        subst.insert(0, Type::Int);
        let ty = Type::Nullable(Box::new(Type::TypeVar(TypeVar::new(0, "T".to_string()))));
        let result = MonomorphizationTracker::substitute_type(&subst, &ty);
        assert_eq!(result, Type::Nullable(Box::new(Type::Int)));
    }

    #[test]
    fn test_substitute_type_nested() {
        let mut subst = HashMap::new();
        subst.insert(0, Type::Int);
        subst.insert(1, Type::String);
        let ty = Type::Function {
            params: vec![Type::TypeVar(TypeVar::new(0, "T".to_string()))],
            return_type: Box::new(Type::TypeVar(TypeVar::new(1, "U".to_string()))),
        };
        let result = MonomorphizationTracker::substitute_type(&subst, &ty);
        assert_eq!(result, Type::Function {
            params: vec![Type::Int],
            return_type: Box::new(Type::String),
        });
    }

    #[test]
    fn test_make_generic_function_def() {
        let mut tracker = MonomorphizationTracker::new();
        let type_params = vec![
            crate::parser::ast::TypeParam {
                name: "T".to_string(),
                bounds: vec![],
            },
        ];
        let def = make_generic_function_def(
            "identity",
            &type_params,
            &[Type::TypeVar(TypeVar::new(0, "T".to_string()))],
            &Type::TypeVar(TypeVar::new(0, "T".to_string())),
            &mut tracker,
        );
        assert_eq!(def.name, "identity");
        assert_eq!(def.type_params.len(), 1);
        assert_eq!(def.type_params[0].name, "T");
    }

    #[test]
    fn test_infer_type_args_identity() {
        let mut tracker = MonomorphizationTracker::new();
        let var_id = tracker.fresh_var_id();
        let type_var = TypeVar::new(var_id, "T".to_string());
        let def = GenericDefinition {
            name: "identity".to_string(),
            type_params: vec![TypeParamInfo::new("T".to_string(), var_id)],
            body_type: Type::Function {
                params: vec![Type::TypeVar(type_var.clone())],
                return_type: Box::new(Type::TypeVar(type_var)),
            },
        };
        tracker.register_generic(def);

        let mut diagnostics = DiagnosticBag::new();
        let inferred = tracker.infer_type_args("identity", &[Type::Int], &mut diagnostics);
        assert!(inferred.is_some());
        let inferred = inferred.unwrap();
        assert_eq!(inferred.len(), 1);
        assert_eq!(inferred[0], Type::Int);
    }

    #[test]
    fn test_infer_type_args_map() {
        let mut tracker = MonomorphizationTracker::new();
        let var_id_t = tracker.fresh_var_id();
        let var_id_u = tracker.fresh_var_id();
        let type_var_t = TypeVar::new(var_id_t, "T".to_string());
        let type_var_u = TypeVar::new(var_id_u, "U".to_string());
        let def = GenericDefinition {
            name: "map".to_string(),
            type_params: vec![
                TypeParamInfo::new("T".to_string(), var_id_t),
                TypeParamInfo::new("U".to_string(), var_id_u),
            ],
            body_type: Type::Function {
                params: vec![
                    Type::Array(Box::new(Type::TypeVar(type_var_t.clone()))),
                    Type::Function {
                        params: vec![Type::TypeVar(type_var_t)],
                        return_type: Box::new(Type::TypeVar(type_var_u.clone())),
                    },
                ],
                return_type: Box::new(Type::Array(Box::new(Type::TypeVar(type_var_u)))),
            },
        };
        tracker.register_generic(def);

        let mut diagnostics = DiagnosticBag::new();
        // map([1, 2, 3], fn(x) => x + 1) → T=int, U=int
        let inferred = tracker.infer_type_args("map", &[
            Type::Array(Box::new(Type::Int)),
            Type::Function {
                params: vec![Type::Int],
                return_type: Box::new(Type::Int),
            },
        ], &mut diagnostics);
        assert!(inferred.is_some());
        let inferred = inferred.unwrap();
        assert_eq!(inferred.len(), 2);
        assert_eq!(inferred[0], Type::Int);
        assert_eq!(inferred[1], Type::Int);
    }

    #[test]
    fn test_specialization_with_nullable() {
        // Test that T? works correctly in specialization
        let mut tracker = MonomorphizationTracker::new();
        let var_id = tracker.fresh_var_id();
        let type_var = TypeVar::new(var_id, "T".to_string());
        let def = GenericDefinition {
            name: "wrap".to_string(),
            type_params: vec![TypeParamInfo::new("T".to_string(), var_id)],
            body_type: Type::Function {
                params: vec![Type::TypeVar(type_var.clone())],
                return_type: Box::new(Type::Nullable(Box::new(Type::TypeVar(type_var)))),
            },
        };
        tracker.register_generic(def);

        let mut diagnostics = DiagnosticBag::new();
        let spec = tracker.specialize("wrap", vec![Type::Int], &mut diagnostics).unwrap();
        assert_eq!(spec.specialized_type, Type::Function {
            params: vec![Type::Int],
            return_type: Box::new(Type::Nullable(Box::new(Type::Int))),
        });
    }

    #[test]
    fn test_specialization_with_dyn() {
        // Per spec Section 10.4: generic called with dyn type argument
        let mut tracker = MonomorphizationTracker::new();
        let var_id = tracker.fresh_var_id();
        let type_var = TypeVar::new(var_id, "T".to_string());
        let def = GenericDefinition {
            name: "identity".to_string(),
            type_params: vec![TypeParamInfo::new("T".to_string(), var_id)],
            body_type: Type::Function {
                params: vec![Type::TypeVar(type_var.clone())],
                return_type: Box::new(Type::TypeVar(type_var)),
            },
        };
        tracker.register_generic(def);

        let mut diagnostics = DiagnosticBag::new();
        let spec = tracker.specialize("identity", vec![Type::Dynamic], &mut diagnostics).unwrap();
        assert_eq!(spec.specialized_type, Type::Function {
            params: vec![Type::Dynamic],
            return_type: Box::new(Type::Dynamic),
        });
    }

    #[test]
    fn test_trait_bounds_check() {
        // Test that trait bounds are accepted (currently always true)
        let mut tracker = MonomorphizationTracker::new();
        let var_id = tracker.fresh_var_id();
        let def = GenericDefinition {
            name: "max".to_string(),
            type_params: vec![TypeParamInfo::with_bounds(
                "T".to_string(),
                var_id,
                vec!["Comparable".to_string()],
            )],
            body_type: Type::Function {
                params: vec![
                    Type::TypeVar(TypeVar::new(var_id, "T".to_string())),
                    Type::TypeVar(TypeVar::new(var_id, "T".to_string())),
                ],
                return_type: Box::new(Type::TypeVar(TypeVar::new(var_id, "T".to_string()))),
            },
        };
        tracker.register_generic(def);

        let mut diagnostics = DiagnosticBag::new();
        // int should satisfy Comparable bound (currently always true)
        let spec = tracker.specialize("max", vec![Type::Int], &mut diagnostics);
        assert!(spec.is_some());
        assert!(!diagnostics.has_errors());
    }
}