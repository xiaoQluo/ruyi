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
pub mod types;
pub mod environment;
pub mod diagnostics;
pub mod constraints;
pub mod inference;
pub mod checker;
pub mod patterns;
pub mod traits;
pub mod generics;

pub use arc::ArcClassRegistry;
pub use types::*;
pub use environment::TypeEnvironment;
pub use diagnostics::{Diagnostic, DiagnosticBag, DiagnosticKind, Severity};
pub use constraints::{ConstraintSolver, SolveResult};
pub use inference::TypeInference;
pub use checker::{TypeChecker, TypeCheckResult};
pub use generics::{MonomorphizationTracker, GenericDefinition, TypeParamInfo, Specialization};