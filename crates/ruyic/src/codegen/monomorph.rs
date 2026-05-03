use crate::typechecker::generics::{MonomorphizationTracker, Specialization};
use crate::typechecker::types::Type;
/**
 * Monomorphization for generic function code generation.
 *
 * Per spec Section 10.3, generates separate LLVM functions for each
 * concrete type instantiation of a generic function.
 *
 * @author Ruyi Team
 * @date 2026-05-01
 */
use std::collections::HashMap;

/// Represents a monomorphized function that needs to be generated.
#[derive(Debug, Clone)]
pub struct MonomorphizedFunction {
    /// The mangled name for this specialization (e.g., "identity__int").
    pub mangled_name: String,
    /// The original generic function name.
    pub generic_name: String,
    /// The concrete type arguments.
    pub type_args: Vec<Type>,
    /// The specialized parameter types.
    pub param_types: Vec<Type>,
    /// The specialized return type.
    pub return_type: Type,
}

impl MonomorphizedFunction {
    /// Creates a new monomorphized function from a specialization.
    pub fn from_specialization(spec: &Specialization) -> Self {
        let (param_types, return_type) = match &spec.specialized_type {
            Type::Function {
                params,
                return_type,
            } => (params.clone(), *return_type.clone()),
            _ => (vec![], Type::Void),
        };

        Self {
            mangled_name: spec.mangled_name.clone(),
            generic_name: spec.generic_name.clone(),
            type_args: spec.type_args.clone(),
            param_types,
            return_type,
        }
    }
}

/// Collects all monomorphized functions that need to be generated.
///
/// This is the bridge between the type checker's specialization tracker
/// and the code generator. It converts specializations into concrete
/// function definitions that can be compiled to LLVM IR.
pub fn collect_monomorphizations(tracker: &MonomorphizationTracker) -> Vec<MonomorphizedFunction> {
    tracker
        .specializations()
        .values()
        .map(|spec| MonomorphizedFunction::from_specialization(spec))
        .collect()
}

/// Manages the mapping from generic function names to their monomorphized versions.
#[derive(Debug, Default)]
pub struct MonomorphizationContext {
    /// Maps mangled names to monomorphized function info.
    functions: HashMap<String, MonomorphizedFunction>,
    /// Tracks which specializations have already been generated.
    generated: HashMap<String, bool>,
}

impl MonomorphizationContext {
    /// Creates a new empty context.
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers a monomorphized function for code generation.
    pub fn register(&mut self, func: MonomorphizedFunction) {
        self.functions.insert(func.mangled_name.clone(), func);
    }

    /// Marks a function as having been generated.
    pub fn mark_generated(&mut self, mangled_name: &str) {
        self.generated.insert(mangled_name.to_string(), true);
    }

    /// Checks if a function has already been generated.
    pub fn is_generated(&self, mangled_name: &str) -> bool {
        self.generated.get(mangled_name).copied().unwrap_or(false)
    }

    /// Gets all registered monomorphized functions.
    pub fn functions(&self) -> &HashMap<String, MonomorphizedFunction> {
        &self.functions
    }

    /// Gets a monomorphized function by its mangled name.
    pub fn get_function(&self, mangled_name: &str) -> Option<&MonomorphizedFunction> {
        self.functions.get(mangled_name)
    }

    /// Populates the context from a monomorphization tracker.
    pub fn populate_from_tracker(&mut self, tracker: &MonomorphizationTracker) {
        for spec in tracker.specializations().values() {
            let func = MonomorphizedFunction::from_specialization(spec);
            self.register(func);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::typechecker::types::TypeVar;

    #[test]
    fn test_monomorphized_function_from_specialization() {
        let spec = Specialization::new(
            "identity",
            vec![Type::Int],
            Type::Function {
                params: vec![Type::Int],
                return_type: Box::new(Type::Int),
            },
        );
        let func = MonomorphizedFunction::from_specialization(&spec);
        assert_eq!(func.mangled_name, "identity__int");
        assert_eq!(func.generic_name, "identity");
        assert_eq!(func.param_types, vec![Type::Int]);
        assert_eq!(func.return_type, Type::Int);
    }

    #[test]
    fn test_monomorphized_function_multiple_args() {
        let spec = Specialization::new(
            "map",
            vec![Type::Int, Type::String],
            Type::Function {
                params: vec![Type::Array(Box::new(Type::Int))],
                return_type: Box::new(Type::Array(Box::new(Type::String))),
            },
        );
        let func = MonomorphizedFunction::from_specialization(&spec);
        assert_eq!(func.mangled_name, "map__int__string");
        assert_eq!(func.type_args, vec![Type::Int, Type::String]);
    }

    #[test]
    fn test_monomorphization_context() {
        let mut ctx = MonomorphizationContext::new();
        let func = MonomorphizedFunction {
            mangled_name: "identity__int".to_string(),
            generic_name: "identity".to_string(),
            type_args: vec![Type::Int],
            param_types: vec![Type::Int],
            return_type: Type::Int,
        };
        ctx.register(func);
        assert!(ctx.get_function("identity__int").is_some());
        assert!(!ctx.is_generated("identity__int"));
        ctx.mark_generated("identity__int");
        assert!(ctx.is_generated("identity__int"));
    }

    #[test]
    fn test_collect_monomorphizations() {
        let mut tracker = MonomorphizationTracker::new();
        let var_id = tracker.fresh_var_id();
        let type_var = TypeVar::new(var_id, "T".to_string());
        let def = crate::typechecker::generics::GenericDefinition {
            name: "identity".to_string(),
            type_params: vec![crate::typechecker::generics::TypeParamInfo::new(
                "T".to_string(),
                var_id,
            )],
            body_type: Type::Function {
                params: vec![Type::TypeVar(type_var.clone())],
                return_type: Box::new(Type::TypeVar(type_var)),
            },
        };
        tracker.register_generic(def);

        let mut diagnostics = crate::typechecker::diagnostics::DiagnosticBag::new();
        tracker.specialize("identity", vec![Type::Int], &mut diagnostics);

        let monomorphizations = collect_monomorphizations(&tracker);
        assert_eq!(monomorphizations.len(), 1);
        assert_eq!(monomorphizations[0].mangled_name, "identity__int");
    }

    #[test]
    fn test_populate_from_tracker() {
        let mut tracker = MonomorphizationTracker::new();
        let var_id = tracker.fresh_var_id();
        let type_var = TypeVar::new(var_id, "T".to_string());
        let def = crate::typechecker::generics::GenericDefinition {
            name: "identity".to_string(),
            type_params: vec![crate::typechecker::generics::TypeParamInfo::new(
                "T".to_string(),
                var_id,
            )],
            body_type: Type::Function {
                params: vec![Type::TypeVar(type_var.clone())],
                return_type: Box::new(Type::TypeVar(type_var)),
            },
        };
        tracker.register_generic(def);

        let mut diagnostics = crate::typechecker::diagnostics::DiagnosticBag::new();
        tracker.specialize("identity", vec![Type::String], &mut diagnostics);

        let mut ctx = MonomorphizationContext::new();
        ctx.populate_from_tracker(&tracker);
        assert!(ctx.get_function("identity__string").is_some());
    }
}
