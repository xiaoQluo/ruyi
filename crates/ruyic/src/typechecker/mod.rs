/**
 * Gradual type checker module for Ruyi.
 *
 * Implements the gradual type system per spec Sections 8-11:
 * - Static and dynamic typing coexist (dyn type)
 * - Bidirectional type inference (synthesize/check)
 * - Type narrowing for null safety
 * - Constraint-based generic inference
 *
 * @author Ruyi Team
 * @date 2026-05-01
 */
pub mod arc;
pub mod checker;
pub mod constraints;
pub mod diagnostics;
pub mod environment;
pub mod generics;
pub mod impl_table;
pub mod inference;
pub mod patterns;
pub mod traits;
pub mod types;

pub use arc::ArcClassRegistry;
pub use checker::{TypeCheckResult, TypeChecker};
pub use constraints::{ConstraintSolver, SolveResult};
pub use diagnostics::{Diagnostic, DiagnosticBag, DiagnosticKind, Severity};
pub use environment::TypeEnvironment;
pub use generics::{GenericDefinition, MonomorphizationTracker, Specialization, TypeParamInfo};
pub use impl_table::{ImplDef, ImplTable, TraitId, TypeId};
pub use inference::TypeInference;
pub use types::*;
