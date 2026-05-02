/**
 * Trait system type checker for Ruyi.
 *
 * Manages trait declarations, implementations, bounds checking,
 * and dispatch information for static and dynamic trait resolution.
 *
 * @author Ruyi Team
 * @date 2026-05-01
 */

use std::collections::HashMap;

use crate::parser::ast::{ClassElement, Declaration, TraitElement, TypeAnnotation, TypeParam};
use crate::typechecker::diagnostics::{DiagnosticBag, DiagnosticKind};
use crate::typechecker::types::Type;

/// Method signature within a trait declaration.
#[derive(Debug, Clone, PartialEq)]
pub struct TraitMethod {
    pub name: String,
    pub param_types: Vec<Type>,
    pub return_type: Type,
    pub has_default: bool,
}

/// Information about a declared trait.
#[derive(Debug, Clone, PartialEq)]
pub struct TraitInfo {
    pub name: String,
    pub type_params: Vec<TypeParam>,
    pub methods: HashMap<String, TraitMethod>,
    pub supertraits: Vec<String>,
    pub is_marker: bool,
}

/// Information about a trait implementation.
#[derive(Debug, Clone, PartialEq)]
pub struct ImplInfo {
    pub trait_name: String,
    pub trait_args: Vec<TypeAnnotation>,
    pub for_type: TypeAnnotation,
    pub type_params: Vec<TypeParam>,
    pub methods: Vec<String>,
}

/// Registry of all traits and implementations visible in a program.
#[derive(Debug, Clone, Default)]
pub struct TraitRegistry {
    traits: HashMap<String, TraitInfo>,
    impls: Vec<ImplInfo>,
    /// Maps (concrete type name, trait name) -> impl index
    type_trait_impls: HashMap<(String, String), usize>,
}

impl TraitRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a trait declaration.
    pub fn register_trait(&mut self, decl: &Declaration) {
        if let Declaration::Trait { name, type_params, body } = decl {
            let mut methods = HashMap::new();
            let mut is_marker = true;

            for element in body {
                if let TraitElement::Method { name: prop_name, params, return_type, body, .. } = element {
                    let method_name = match prop_name {
                        crate::parser::ast::PropertyName::Ident(n) => n.clone(),
                        _ => continue,
                    };
                    let param_types: Vec<Type> = params
                        .iter()
                        .map(|p| {
                            p.ty.as_ref()
                                .map(Type::from_annotation)
                                .unwrap_or(Type::Dynamic)
                        })
                        .collect();
                    let ret_type = return_type
                        .as_ref()
                        .map(Type::from_annotation)
                        .unwrap_or(Type::Void);

                    methods.insert(
                        method_name.clone(),
                        TraitMethod {
                            name: method_name,
                            param_types,
                            return_type: ret_type,
                            has_default: body.is_some(),
                        },
                    );
                    is_marker = false;
                }
            }

            self.traits.insert(
                name.clone(),
                TraitInfo {
                    name: name.clone(),
                    type_params: type_params.clone(),
                    methods,
                    supertraits: Vec::new(),
                    is_marker,
                },
            );
        }
    }

    /// Register an impl declaration.
    pub fn register_impl(&mut self, decl: &Declaration) {
        if let Declaration::Impl { type_params, trait_name, trait_args, for_type, body } = decl {
            let mut methods = Vec::new();
            for element in body {
                if let ClassElement::Method { name: prop_name, .. } = element {
                    let method_name = match prop_name {
                        crate::parser::ast::PropertyName::Ident(n) => n.clone(),
                        _ => continue,
                    };
                    methods.push(method_name);
                }
            }

            let impl_info = ImplInfo {
                trait_name: trait_name.clone(),
                trait_args: trait_args.clone(),
                for_type: for_type.clone(),
                type_params: type_params.clone(),
                methods,
            };

            let impl_idx = self.impls.len();
            self.impls.push(impl_info);

            // Index by concrete type name if possible
            let type_key = type_annotation_name(for_type);
            if !type_key.is_empty() {
                self.type_trait_impls.insert((type_key, trait_name.clone()), impl_idx);
            }
        }
    }

    /// Look up a trait by name.
    pub fn get_trait(&self, name: &str) -> Option<&TraitInfo> {
        self.traits.get(name)
    }

    /// Check if a type implements a trait.
    pub fn implements(&self, ty: &Type, trait_name: &str) -> bool {
        match ty {
            Type::Named(name) | Type::Generic { base: name, .. } => {
                self.type_trait_impls.contains_key(&(name.clone(), trait_name.to_string()))
            }
            Type::Dynamic | Type::Error => true,
            _ => false,
        }
    }

    /// Get the impl index for a (type, trait) pair.
    pub fn get_impl_index(&self, ty: &Type, trait_name: &str) -> Option<usize> {
        match ty {
            Type::Named(name) | Type::Generic { base: name, .. } => {
                self.type_trait_impls.get(&(name.clone(), trait_name.to_string())).copied()
            }
            _ => None,
        }
    }

    /// Get all registered traits.
    pub fn traits(&self) -> &HashMap<String, TraitInfo> {
        &self.traits
    }

    /// Get all registered impls.
    pub fn impls(&self) -> &[ImplInfo] {
        &self.impls
    }

    /// Validate that all required trait methods are implemented.
    pub fn validate_impls(&self, diagnostics: &mut DiagnosticBag) {
        for (i, impl_info) in self.impls.iter().enumerate() {
            if let Some(trait_info) = self.traits.get(&impl_info.trait_name) {
                // Check that all non-default trait methods are implemented
                for (method_name, trait_method) in &trait_info.methods {
                    if !trait_method.has_default && !impl_info.methods.contains(method_name) {
                        let for_type_str = type_annotation_name(&impl_info.for_type);
                        diagnostics.add_error(DiagnosticKind::TraitNotImplemented {
                            ty: Type::Named(for_type_str.clone()),
                            trait_name: format!("{}::{}", impl_info.trait_name, method_name),
                        });
                    }
                }
            } else {
                diagnostics.add_error(DiagnosticKind::Other {
                    message: format!("Impl {} references unknown trait `{}`", i, impl_info.trait_name),
                });
            }
        }
    }

    /// Check if a type satisfies a trait bound.
    pub fn check_bound(&self, ty: &Type, trait_name: &str) -> bool {
        self.implements(ty, trait_name)
    }

    /// Returns true if the trait is a marker trait (no methods).
    pub fn is_marker_trait(&self, trait_name: &str) -> bool {
        self.traits
            .get(trait_name)
            .map(|t| t.is_marker)
            .unwrap_or(false)
    }

    /// Get method signatures for a trait.
    pub fn trait_methods(&self, trait_name: &str) -> Option<&HashMap<String, TraitMethod>> {
        self.traits.get(trait_name).map(|t| &t.methods)
    }
}

fn type_annotation_name(annotation: &TypeAnnotation) -> String {
    match annotation {
        TypeAnnotation::Identifier(name) => name.clone(),
        TypeAnnotation::Generic { base, .. } => base.clone(),
        _ => String::new(),
    }
}

/// Collect trait and impl declarations from a program into a registry.
pub fn build_trait_registry(program: &crate::parser::ast::Program) -> TraitRegistry {
    let mut registry = TraitRegistry::new();
    for item in &program.items {
        if let crate::parser::ast::ModuleItem::Declaration(decl) = item {
            match decl {
                Declaration::Trait { .. } => registry.register_trait(decl),
                Declaration::Impl { .. } => registry.register_impl(decl),
                _ => {}
            }
        }
    }
    registry
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::Parser;

    fn registry_from_source(source: &str) -> TraitRegistry {
        let mut parser = Parser::new(source).expect("lexer should not fail");
        let program = parser.parse().expect("parse should succeed");
        build_trait_registry(&program)
    }

    #[test]
    fn test_register_trait() {
        let registry = registry_from_source("trait Printable { fn format(self): string; }");
        assert!(registry.get_trait("Printable").is_some());
        let trait_info = registry.get_trait("Printable").unwrap();
        assert!(!trait_info.is_marker);
        assert!(trait_info.methods.contains_key("format"));
    }

    #[test]
    fn test_register_marker_trait() {
        let registry = registry_from_source("trait Marker { }");
        let trait_info = registry.get_trait("Marker").unwrap();
        assert!(trait_info.is_marker);
    }

    #[test]
    fn test_register_impl() {
        let registry = registry_from_source(
            "trait Printable { fn format(self): string; }\nimpl Printable for int { fn format(self): string { return \"\"; } }"
        );
        assert!(registry.implements(&Type::Named("int".into()), "Printable"));
    }

    #[test]
    fn test_impl_validation_missing_method() {
        let mut registry = registry_from_source(
            "trait Printable { fn format(self): string; }\nimpl Printable for int { }"
        );
        let mut diagnostics = DiagnosticBag::new();
        registry.validate_impls(&mut diagnostics);
        assert!(diagnostics.has_errors());
    }

    #[test]
    fn test_trait_bound_check() {
        let registry = registry_from_source(
            "trait Printable { fn format(self): string; }\nimpl Printable for int { fn format(self): string { return \"\"; } }"
        );
        assert!(registry.check_bound(&Type::Named("int".into()), "Printable"));
        assert!(!registry.check_bound(&Type::Named("string".into()), "Printable"));
    }
}
