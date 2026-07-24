/**
 * ARC type checker support for Ruyi.
 *
 * Tracks which classes are annotated with `@arc` and provides
 * type checking rules specific to ARC-managed objects.
 *
 * @author Ruyi Team
 * @date 2026-05-02
 */
use std::collections::HashSet;

use crate::parser::ast::Program;
use crate::typechecker::types::Type;

/// Registry of ARC-annotated class names.
#[derive(Debug, Clone, Default)]
pub struct ArcClassRegistry {
    arc_classes: HashSet<String>,
}

impl ArcClassRegistry {
    pub fn new() -> Self {
        Self {
            arc_classes: HashSet::new(),
        }
    }

    /// Scan a program for `@arc` class declarations and register them.
    pub fn scan_program(&mut self, program: &Program) {
        for item in &program.items {
            use crate::parser::ast::ModuleItem;
            if let ModuleItem::Declaration(decl) = item {
                self.scan_declaration(decl);
            }
        }
    }

    /// Register a single class name as ARC-managed.
    pub fn register(&mut self, name: &str) {
        self.arc_classes.insert(name.to_string());
    }

    fn scan_declaration(&mut self, decl: &crate::parser::ast::Declaration) {
        use crate::parser::ast::Declaration;
        if let Declaration::Class {
            name, annotations, ..
        } = decl
        {
            if annotations.iter().any(|a| a == "arc") {
                self.arc_classes.insert(name.clone());
            }
        }
    }

    /// Returns `true` if the given class name is registered as ARC.
    pub fn is_arc_class(&self, name: &str) -> bool {
        self.arc_classes.contains(name)
    }

    /// Returns `true` if the given type is an ARC-managed class.
    pub fn is_arc_type(&self, ty: &Type) -> bool {
        match ty {
            Type::Named(name, _) => self.is_arc_class(name),
            Type::Generic { base, .. } => self.is_arc_class(base),
            _ => false,
        }
    }

    /// Returns the number of registered ARC classes.
    pub fn len(&self) -> usize {
        self.arc_classes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.arc_classes.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::Parser;

    #[test]
    fn test_scan_arc_class() {
        let mut parser = Parser::new("@arc class Foo {}").unwrap();
        let program = parser.parse().unwrap();
        let mut registry = ArcClassRegistry::new();
        registry.scan_program(&program);
        assert!(registry.is_arc_class("Foo"));
        assert!(!registry.is_arc_class("Bar"));
    }

    #[test]
    fn test_scan_mixed_classes() {
        let source = r#"
            @arc class ArcClass {}
            class GcClass {}
        "#;
        let mut parser = Parser::new(source).unwrap();
        let program = parser.parse().unwrap();
        let mut registry = ArcClassRegistry::new();
        registry.scan_program(&program);
        assert!(registry.is_arc_class("ArcClass"));
        assert!(!registry.is_arc_class("GcClass"));
    }

    #[test]
    fn test_is_arc_type() {
        let mut registry = ArcClassRegistry::new();
        registry.arc_classes.insert("ArcBox".into());

        assert!(registry.is_arc_type(&Type::Named("ArcBox".into(), vec![])));
        assert!(!registry.is_arc_type(&Type::Named("GcBox".into(), vec![])));
        assert!(registry.is_arc_type(&Type::Generic {
            base: "ArcBox".into(),
            args: vec![Type::Int],
        }));
    }
}
