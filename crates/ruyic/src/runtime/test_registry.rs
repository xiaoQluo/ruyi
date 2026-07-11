/**
 * Registry of `@test` function declarations collected during compilation.
 *
 * Each entry is keyed by `file:line` so that two `@test fn runs()` in
 * different files produce two distinct entries (the function name alone
 * is NOT a unique identifier). The registry is populated once at
 * typecheck time and treated as read-only thereafter.
 *
 * @author Ruyi Team
 * @date 2026-07-12
 */
use std::collections::BTreeMap;

use crate::parser::ast::Declaration;

/// A single `@test fn` declaration captured by the registry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestFnEntry {
    pub name: String,
    pub file: String,
    pub line: usize,
    pub module: String,
}

/// In-memory registry of `@test fn` declarations keyed by `file:line`.
#[derive(Debug, Clone, Default)]
pub struct TestFunctionRegistry {
    tests: BTreeMap<String, TestFnEntry>,
}

impl TestFunctionRegistry {
    /// Construct a fresh, empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a single `@test fn` entry. The `file:line` pair forms the
    /// dedup key — re-registering the same location replaces the entry.
    pub fn register(&mut self, entry: TestFnEntry) {
        self.tests
            .insert(format!("{}:{}", entry.file, entry.line), entry);
    }

    /// Walk a slice of declarations and register every `Function` that
    /// carries the `@test` annotation. The `file` and `module` labels are
    /// stored verbatim on each entry.
    pub fn collect_from_program(
        &mut self,
        decls: &[Declaration],
        file: &str,
        module: &str,
    ) {
        for decl in decls {
            if let Declaration::Function {
                name,
                annotations,
                ..
            } = decl
            {
                if annotations.iter().any(|a| a == "test") {
                    self.register(TestFnEntry {
                        name: name.clone(),
                        file: file.to_string(),
                        line: 0,
                        module: module.to_string(),
                    });
                }
            }
        }
    }

    /// Borrow every entry, sorted by the `file:line` key.
    pub fn all(&self) -> Vec<&TestFnEntry> {
        self.tests.values().collect()
    }

    /// Number of registered entries.
    pub fn count(&self) -> usize {
        self.tests.len()
    }
}