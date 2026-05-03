/**
 * Type constraint generation and solving for generic type inference.
 *
 * Implements constraint-based inference per spec Section 8.2.4:
 * collects type constraints during type checking and solves them
 * via unification to infer concrete types for generic parameters.
 *
 * @author Ruyi Team
 * @date 2026-05-01
 */
use crate::typechecker::types::{Type, TypeConstraint, TypeVar};
use std::collections::HashMap;

/// Result of constraint solving.
#[derive(Debug, Clone, PartialEq)]
pub enum SolveResult {
    /// All constraints satisfied; provides the substitution map.
    Solved(HashMap<u32, Type>),
    /// Constraints are unsatisfiable; contains error descriptions.
    Error(Vec<String>),
}

/// Constraint solver for type inference.
///
/// Collects equality and subtype constraints, then solves them
/// via unification to produce a substitution mapping type variables
/// to concrete types.
#[derive(Debug, Clone)]
pub struct ConstraintSolver {
    constraints: Vec<TypeConstraint>,
    next_var_id: u32,
}

impl ConstraintSolver {
    pub fn new() -> Self {
        Self {
            constraints: Vec::new(),
            next_var_id: 0,
        }
    }

    /// Creates a fresh type variable for inference.
    pub fn fresh_var(&mut self, prefix: &str) -> TypeVar {
        let id = self.next_var_id;
        self.next_var_id += 1;
        TypeVar::new(id, format!("{}{}", prefix, id))
    }

    /// Adds an equality constraint: `t1 = t2`.
    pub fn add_equal(&mut self, t1: Type, t2: Type) {
        self.constraints.push(TypeConstraint::Equal(t1, t2));
    }

    /// Adds a subtype constraint: `sub <: sup`.
    pub fn add_subtype(&mut self, sub: Type, sup: Type) {
        self.constraints.push(TypeConstraint::Subtype { sub, sup });
    }

    /// Adds a trait bound constraint: type variable must implement trait.
    pub fn add_trait_bound(&mut self, type_var: TypeVar, trait_name: String) {
        self.constraints.push(TypeConstraint::Implements {
            type_var,
            trait_name,
        });
    }

    /// Solves all collected constraints via unification.
    pub fn solve(self) -> SolveResult {
        let mut subst: HashMap<u32, Type> = HashMap::new();
        let mut errors = Vec::new();

        for constraint in self.constraints {
            if let Err(e) = solve_constraint(constraint, &mut subst) {
                errors.push(e);
            }
        }

        if errors.is_empty() {
            SolveResult::Solved(subst)
        } else {
            SolveResult::Error(errors)
        }
    }

    /// Applies the substitution to a type, replacing type variables with their solutions.
    pub fn apply_subst(subst: &HashMap<u32, Type>, ty: &Type) -> Type {
        match ty {
            Type::TypeVar(var) => match subst.get(&var.id) {
                Some(resolved) => Self::apply_subst(subst, resolved),
                None => ty.clone(),
            },
            Type::Nullable(inner) => Type::Nullable(Box::new(Self::apply_subst(subst, inner))),
            Type::Array(elem) => Type::Array(Box::new(Self::apply_subst(subst, elem))),
            Type::Object(fields) => Type::Object(
                fields
                    .iter()
                    .map(|f| crate::typechecker::types::ObjectField {
                        name: f.name.clone(),
                        ty: Self::apply_subst(subst, &f.ty),
                        optional: f.optional,
                    })
                    .collect(),
            ),
            Type::Function {
                params,
                return_type,
            } => Type::Function {
                params: params.iter().map(|p| Self::apply_subst(subst, p)).collect(),
                return_type: Box::new(Self::apply_subst(subst, return_type)),
            },
            Type::Generic { base, args } => Type::Generic {
                base: base.clone(),
                args: args.iter().map(|a| Self::apply_subst(subst, a)).collect(),
            },
            _ => ty.clone(),
        }
    }
}

impl Default for ConstraintSolver {
    fn default() -> Self {
        Self::new()
    }
}

/// Solves a single constraint, updating the substitution map.
fn solve_constraint(
    constraint: TypeConstraint,
    subst: &mut HashMap<u32, Type>,
) -> Result<(), String> {
    match constraint {
        TypeConstraint::Equal(t1, t2) => unify(&t1, &t2, subst),
        TypeConstraint::Subtype { sub, sup } => {
            if sub.is_subtype_of(&sup) {
                Ok(())
            } else if sub.is_dynamic() || sup.is_dynamic() {
                Ok(())
            } else {
                // Try unification as fallback for type variables
                unify(&sub, &sup, subst)
            }
        }
        TypeConstraint::Implements { .. } => {
            // Trait bounds are checked at monomorphization time
            Ok(())
        }
    }
}

/// Unifies two types, producing a substitution or an error.
fn unify(t1: &Type, t2: &Type, subst: &mut HashMap<u32, Type>) -> Result<(), String> {
    let t1 = ConstraintSolver::apply_subst(subst, t1);
    let t2 = ConstraintSolver::apply_subst(subst, t2);

    // Same type — done
    if t1 == t2 {
        return Ok(());
    }

    // Error type unifies with anything
    if t1.is_error() || t2.is_error() {
        return Ok(());
    }

    // dyn unifies with anything
    if t1.is_dynamic() || t2.is_dynamic() {
        return Ok(());
    }

    // Type variable unification
    if let (Type::TypeVar(v1), Type::TypeVar(v2)) = (&t1, &t2) {
        if v1.id == v2.id {
            return Ok(());
        }
    }
    if let Type::TypeVar(var) = &t1 {
        if occurs_in(var, &t2) {
            return Err(format!(
                "Cannot construct infinite type: {} = {}",
                var.name, t2
            ));
        }
        subst.insert(var.id, t2.clone());
        return Ok(());
    }
    if let Type::TypeVar(var) = &t2 {
        if occurs_in(var, &t1) {
            return Err(format!(
                "Cannot construct infinite type: {} = {}",
                var.name, t1
            ));
        }
        subst.insert(var.id, t1.clone());
        return Ok(());
    }

    // Structural unification
    match (&t1, &t2) {
        (Type::Nullable(inner1), Type::Nullable(inner2)) => unify(inner1, inner2, subst),
        (Type::Array(elem1), Type::Array(elem2)) => unify(elem1, elem2, subst),
        (
            Type::Function {
                params: p1,
                return_type: r1,
            },
            Type::Function {
                params: p2,
                return_type: r2,
            },
        ) => {
            if p1.len() != p2.len() {
                return Err(format!(
                    "Function parameter count mismatch: {} vs {}",
                    p1.len(),
                    p2.len()
                ));
            }
            for (param1, param2) in p1.iter().zip(p2.iter()) {
                unify(param1, param2, subst)?;
            }
            unify(r1, r2, subst)
        }
        (Type::Generic { base: b1, args: a1 }, Type::Generic { base: b2, args: a2 }) => {
            if b1 != b2 {
                return Err(format!("Cannot unify {} with {}", t1, t2));
            }
            if a1.len() != a2.len() {
                return Err(format!(
                    "Generic argument count mismatch for {}: {} vs {}",
                    b1,
                    a1.len(),
                    a2.len()
                ));
            }
            for (arg1, arg2) in a1.iter().zip(a2.iter()) {
                unify(arg1, arg2, subst)?;
            }
            Ok(())
        }
        (Type::Object(fields1), Type::Object(fields2)) => unify_objects(fields1, fields2, subst),
        // int unifies with float (widening)
        (Type::Int, Type::Float) | (Type::Float, Type::Int) => Ok(()),
        // Never unifies with anything
        (Type::Never, _) | (_, Type::Never) => Ok(()),
        _ => Err(format!("Cannot unify {} with {}", t1, t2)),
    }
}

/// Unifies two object types structurally.
fn unify_objects(
    fields1: &[crate::typechecker::types::ObjectField],
    fields2: &[crate::typechecker::types::ObjectField],
    subst: &mut HashMap<u32, Type>,
) -> Result<(), String> {
    let map2: HashMap<&str, &Type> = fields2.iter().map(|f| (f.name.as_str(), &f.ty)).collect();

    for f1 in fields1 {
        if let Some(f2_ty) = map2.get(f1.name.as_str()) {
            unify(&f1.ty, f2_ty, subst)?;
        }
        // Fields only in f1 are OK (structural subtyping)
    }
    Ok(())
}

/// Occurs check: does the type variable appear in the type?
fn occurs_in(var: &TypeVar, ty: &Type) -> bool {
    match ty {
        Type::TypeVar(v) => v.id == var.id,
        Type::Nullable(inner) => occurs_in(var, inner),
        Type::Array(elem) => occurs_in(var, elem),
        Type::Object(fields) => fields.iter().any(|f| occurs_in(var, &f.ty)),
        Type::Function {
            params,
            return_type,
        } => params.iter().any(|p| occurs_in(var, p)) || occurs_in(var, return_type),
        Type::Generic { args, .. } => args.iter().any(|a| occurs_in(var, a)),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fresh_var() {
        let mut solver = ConstraintSolver::new();
        let v0 = solver.fresh_var("T");
        let v1 = solver.fresh_var("T");
        assert_ne!(v0.id, v1.id);
        assert_eq!(v0.name, "T0");
        assert_eq!(v1.name, "T1");
    }

    #[test]
    fn test_unify_same_type() {
        let mut solver = ConstraintSolver::new();
        solver.add_equal(Type::Int, Type::Int);
        let result = solver.solve();
        match result {
            SolveResult::Solved(_) => {}
            SolveResult::Error(errs) => panic!("Expected success, got errors: {:?}", errs),
        }
    }

    #[test]
    fn test_unify_type_var() {
        let mut solver = ConstraintSolver::new();
        let var = solver.fresh_var("T");
        solver.add_equal(Type::TypeVar(var.clone()), Type::Int);
        let result = solver.solve();
        match result {
            SolveResult::Solved(subst) => {
                assert_eq!(subst.get(&var.id), Some(&Type::Int));
            }
            SolveResult::Error(errs) => panic!("Expected success, got errors: {:?}", errs),
        }
    }

    #[test]
    fn test_unify_incompatible() {
        let mut solver = ConstraintSolver::new();
        solver.add_equal(Type::Int, Type::String);
        let result = solver.solve();
        match result {
            SolveResult::Solved(_) => panic!("Expected error for incompatible types"),
            SolveResult::Error(_) => {}
        }
    }

    #[test]
    fn test_unify_dyn_with_anything() {
        let mut solver = ConstraintSolver::new();
        solver.add_equal(Type::Dynamic, Type::Int);
        let result = solver.solve();
        match result {
            SolveResult::Solved(_) => {}
            SolveResult::Error(errs) => panic!("Expected success, got errors: {:?}", errs),
        }
    }

    #[test]
    fn test_unify_int_float() {
        let mut solver = ConstraintSolver::new();
        solver.add_equal(Type::Int, Type::Float);
        let result = solver.solve();
        match result {
            SolveResult::Solved(_) => {}
            SolveResult::Error(errs) => panic!("Expected success, got errors: {:?}", errs),
        }
    }

    #[test]
    fn test_occurs_check() {
        let mut solver = ConstraintSolver::new();
        let var = solver.fresh_var("T");
        let recursive = Type::Function {
            params: vec![Type::TypeVar(var.clone())],
            return_type: Box::new(Type::TypeVar(var.clone())),
        };
        solver.add_equal(Type::TypeVar(var.clone()), recursive);
        let result = solver.solve();
        match result {
            SolveResult::Solved(_) => panic!("Expected error for recursive type"),
            SolveResult::Error(_) => {}
        }
    }

    #[test]
    fn test_apply_subst() {
        let mut subst = HashMap::new();
        let var = TypeVar::new(0, "T0".into());
        subst.insert(0, Type::Int);
        let ty = Type::Nullable(Box::new(Type::TypeVar(var)));
        let result = ConstraintSolver::apply_subst(&subst, &ty);
        assert_eq!(result, Type::Nullable(Box::new(Type::Int)));
    }
}
