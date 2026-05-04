#[cfg(feature = "inkwell")]
pub mod landing_pad;
pub mod runtime;
pub mod types;

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

/// Unique identifier for an exception type.
///
/// Type IDs are generated at compile time and used by both the runtime
/// exception matcher and the LLVM `llvm.eh.typeid.for` intrinsic.
pub type TypeId = u64;

/// Global counter for generating runtime type IDs.
static NEXT_TYPE_ID: AtomicU64 = AtomicU64::new(1);

/// Generate a fresh runtime type ID.
pub fn fresh_type_id() -> TypeId {
    NEXT_TYPE_ID.fetch_add(1, Ordering::SeqCst)
}

/// Predefined exception type IDs used by the Ruyi runtime.
pub mod builtin_type_ids {
    use super::TypeId;
    pub const ANY: TypeId = 0; // catch-all (catch { })
    pub const ERROR: TypeId = 1;
    pub const TYPE_ERROR: TypeId = 2;
    pub const RANGE_ERROR: TypeId = 3;
    pub const RUNTIME_ERROR: TypeId = 4;
}

/// A single entry in a per-function exception table.
///
/// Ruyi uses the Itanium C++ ABI / LLVM landing-pad model. Each
/// `try { ... }` block produces one `ExceptionTableEntry` that maps
/// the protected PC range to a landing pad and a list of catch clauses.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExceptionTableEntry {
    /// Byte offset of the first instruction protected by this entry.
    pub try_start: u64,
    /// Byte offset of the first instruction **after** the protected range.
    pub try_end: u64,
    /// Byte offset of the landing-pad basic block.
    pub landing_pad: u64,
    /// Ordered list of catch clauses (evaluated left-to-right).
    pub catch_clauses: Vec<CatchClause>,
    /// `true` if this entry has a `finally` / cleanup action.
    pub has_cleanup: bool,
}

impl ExceptionTableEntry {
    /// Create a new exception table entry.
    pub fn new(try_start: u64, try_end: u64, landing_pad: u64) -> Self {
        Self {
            try_start,
            try_end,
            landing_pad,
            catch_clauses: Vec::new(),
            has_cleanup: false,
        }
    }

    /// Add a catch clause that filters on `type_id`.
    ///
    /// `type_id == 0` means "catch all".
    pub fn catch(mut self, type_id: TypeId, handler_offset: u64) -> Self {
        self.catch_clauses.push(CatchClause {
            type_id,
            handler_offset,
        });
        self
    }

    /// Add a catch-all clause.
    pub fn catch_all(mut self, handler_offset: u64) -> Self {
        self.catch_clauses.push(CatchClause {
            type_id: builtin_type_ids::ANY,
            handler_offset,
        });
        self
    }

    /// Mark this entry as having a cleanup (`finally`) action.
    pub fn cleanup(mut self) -> Self {
        self.has_cleanup = true;
        self
    }

    /// Return the handler offset for the first matching catch clause,
    /// or `None` if no clause matches.
    pub fn matching_handler(&self, thrown_type: TypeId) -> Option<u64> {
        for clause in &self.catch_clauses {
            if clause.matches(thrown_type) {
                return Some(clause.handler_offset);
            }
        }
        None
    }
}

/// One arm of a `catch` block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatchClause {
    /// Type filter. `0` = catch-all.
    pub type_id: TypeId,
    /// Byte offset of the handler basic block.
    pub handler_offset: u64,
}

impl CatchClause {
    /// Check whether this clause matches `thrown_type`.
    pub fn matches(&self, thrown_type: TypeId) -> bool {
        self.type_id == builtin_type_ids::ANY || self.type_id == thrown_type
    }
}

/// Per-function exception table used by the runtime unwinder.
///
/// LLVM emits a reference to this table via the `landingpad` instruction.
/// The runtime (or libunwind) walks the table to find the correct handler.
#[derive(Debug, Clone, Default)]
pub struct FunctionExceptionTable {
    /// Function name (for diagnostics).
    pub function_name: String,
    /// Ordered list of protected regions.
    pub entries: Vec<ExceptionTableEntry>,
}

impl FunctionExceptionTable {
    pub fn new(function_name: impl Into<String>) -> Self {
        Self {
            function_name: function_name.into(),
            entries: Vec::new(),
        }
    }

    /// Register a new try/catch region.
    pub fn add_entry(&mut self, entry: ExceptionTableEntry) {
        self.entries.push(entry);
    }

    /// Look up the exception table entry that covers `pc_offset`.
    pub fn entry_for_pc(&self, pc_offset: u64) -> Option<&ExceptionTableEntry> {
        self.entries
            .iter()
            .find(|e| e.try_start <= pc_offset && pc_offset < e.try_end)
    }
}

/// Global registry of all function exception tables.
///
/// In a real program this is populated by the compiler and referenced
/// from the unwind info section.
#[derive(Debug, Default)]
pub struct ExceptionTableRegistry {
    tables: HashMap<String, FunctionExceptionTable>,
}

impl ExceptionTableRegistry {
    pub fn new() -> Self {
        Self {
            tables: HashMap::new(),
        }
    }

    pub fn register(&mut self, table: FunctionExceptionTable) {
        self.tables.insert(table.function_name.clone(), table);
    }

    pub fn get(&self, function_name: &str) -> Option<&FunctionExceptionTable> {
        self.tables.get(function_name)
    }

    pub fn len(&self) -> usize {
        self.tables.len()
    }

    pub fn is_empty(&self) -> bool {
        self.tables.is_empty()
    }
}

/// Runtime representation of a thrown Ruyi exception.
///
/// This is the object that travels through the unwind machinery.
#[derive(Debug, Clone)]
pub struct RuyiException {
    /// Type ID of the exception (used for catch filtering).
    pub type_id: TypeId,
    /// Human-readable message.
    pub message: String,
    /// Optional stack trace frames.
    pub stack_trace: Vec<StackFrame>,
}

impl RuyiException {
    pub fn new(type_id: TypeId, message: impl Into<String>) -> Self {
        Self {
            type_id,
            message: message.into(),
            stack_trace: Vec::new(),
        }
    }

    pub fn with_stack_trace(mut self, trace: Vec<StackFrame>) -> Self {
        self.stack_trace = trace;
        self
    }
}

/// One frame in a Ruyi stack trace.
#[repr(C)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StackFrame {
    pub function_name: String,
    pub file: String,
    pub line: u32,
}

impl StackFrame {
    pub fn new(function_name: impl Into<String>, file: impl Into<String>, line: u32) -> Self {
        Self {
            function_name: function_name.into(),
            file: file.into(),
            line,
        }
    }
}

/// Landing-pad action kinds used when generating LLVM IR.
///
/// These correspond to the clauses passed to the LLVM `landingpad`
/// instruction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LandingPadAction {
    /// `catch` clause — matches a specific type.
    Catch(TypeId),
    /// `filter` clause — matches if the type is **not** in the list.
    Filter(&'static [TypeId]),
    /// `cleanup` clause — always runs (used for `finally`).
    Cleanup,
}

/// Descriptor for a single landing pad basic block.
///
/// Code generation uses this structure to emit the correct LLVM
/// `landingpad` instruction and branching logic.
#[derive(Debug, Clone, Default)]
pub struct LandingPadDescriptor {
    /// Actions evaluated in order.
    pub actions: Vec<LandingPadAction>,
    /// Label of the catch dispatch block (or 0 if none).
    pub catch_block: u64,
    /// Label of the cleanup block (or 0 if none).
    pub cleanup_block: u64,
}

impl LandingPadDescriptor {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_action(mut self, action: LandingPadAction) -> Self {
        self.actions.push(action);
        self
    }

    pub fn set_catch_block(mut self, label: u64) -> Self {
        self.catch_block = label;
        self
    }

    pub fn set_cleanup_block(mut self, label: u64) -> Self {
        self.cleanup_block = label;
        self
    }
}

/// Throw a Ruyi exception.
///
/// In the full implementation this will invoke the unwinder (e.g.
/// `_Unwind_RaiseException`). The current version panics with a
/// descriptive message so that tests and early integration can verify
/// exception paths.
pub fn throw_exception(exc: RuyiException) -> ! {
    panic!(
        "RuyiException(type_id={}, message={})",
        exc.type_id, exc.message
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exception_table_lookup() {
        let mut table = FunctionExceptionTable::new("main");
        table.add_entry(
            ExceptionTableEntry::new(0, 100, 200)
                .catch(builtin_type_ids::ERROR, 300)
                .catch_all(400)
                .cleanup(),
        );

        let entry = table.entry_for_pc(50).unwrap();
        assert_eq!(entry.try_start, 0);
        assert_eq!(entry.try_end, 100);
        assert_eq!(entry.landing_pad, 200);
        assert!(entry.has_cleanup);

        assert_eq!(entry.matching_handler(builtin_type_ids::ERROR), Some(300));
        assert_eq!(
            entry.matching_handler(builtin_type_ids::TYPE_ERROR),
            Some(400)
        );
    }

    #[test]
    fn test_catch_clause_matches() {
        let catch_error = CatchClause {
            type_id: builtin_type_ids::ERROR,
            handler_offset: 10,
        };
        let catch_all = CatchClause {
            type_id: builtin_type_ids::ANY,
            handler_offset: 20,
        };

        assert!(catch_error.matches(builtin_type_ids::ERROR));
        assert!(!catch_error.matches(builtin_type_ids::TYPE_ERROR));
        assert!(catch_all.matches(builtin_type_ids::TYPE_ERROR));
        assert!(catch_all.matches(999));
    }

    #[test]
    fn test_registry() {
        let mut registry = ExceptionTableRegistry::new();
        let table = FunctionExceptionTable::new("foo");
        registry.register(table);
        assert_eq!(registry.len(), 1);
        assert!(registry.get("foo").is_some());
        assert!(registry.get("bar").is_none());
    }

    #[test]
    fn test_landing_pad_descriptor() {
        let desc = LandingPadDescriptor::new()
            .add_action(LandingPadAction::Catch(builtin_type_ids::ERROR))
            .add_action(LandingPadAction::Cleanup)
            .set_catch_block(100)
            .set_cleanup_block(200);

        assert_eq!(desc.actions.len(), 2);
        assert_eq!(desc.catch_block, 100);
        assert_eq!(desc.cleanup_block, 200);
    }

    #[test]
    fn test_ruyi_exception() {
        let exc = RuyiException::new(builtin_type_ids::RUNTIME_ERROR, "oops")
            .with_stack_trace(vec![StackFrame::new("main", "main.ry", 42)]);

        assert_eq!(exc.type_id, builtin_type_ids::RUNTIME_ERROR);
        assert_eq!(exc.message, "oops");
        assert_eq!(exc.stack_trace.len(), 1);
        assert_eq!(exc.stack_trace[0].line, 42);
    }

    #[test]
    #[should_panic(expected = "RuyiException")]
    fn test_throw_exception() {
        throw_exception(RuyiException::new(builtin_type_ids::ERROR, "test throw"));
    }
}
