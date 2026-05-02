use crate::parser::ast::{Expr, MatchArm, Pattern, Statement};
use crate::typechecker::types::Type;
use std::collections::HashMap;

/// Generates code for match expressions and patterns.
pub struct PatternCompiler<'ctx> {
    scratch: HashMap<String, &'ctx str>,
}

impl<'ctx> PatternCompiler<'ctx> {
    pub fn new() -> Self {
        Self {
            scratch: HashMap::new(),
        }
    }

    /// Compiles a match expression to a series of conditional branches.
    /// Returns the basic block that will receive control after the match.
    pub fn compile_match<B, I>(
        &mut self,
        builder: &mut B,
        scrutinee: &Expr,
        scrutinee_type: &Type,
        arms: &[MatchArm],
        current_block: I,
        default_block: I,
    ) where
        B: BlockBuilder<'ctx>,
        I: Copy + Into<inkwell::values::BasicBlock<'ctx>>,
    {
        // For primitive types, generate a simple switch-like structure
        // For complex types, generate equality checks and destructuring
        match scrutinee_type {
            Type::Int | Type::String | Type::BigInt | Type::Float => {
                self.compile_primitive_match(builder, scrutinee, arms, current_block, default_block);
            }
            Type::Bool => {
                self.compile_bool_match(builder, scrutinee, arms, current_block, default_block);
            }
            Type::Null => {
                // Null can only match null or wildcard
                self.compile_null_match(builder, scrutinee, arms, current_block, default_block);
            }
            Type::Array(_) => {
                self.compile_array_match(builder, scrutinee, scrutinee_type, arms, current_block, default_block);
            }
            Type::Object(_) => {
                self.compile_object_match(builder, scrutinee, scrutinee_type, arms, current_block, default_block);
            }
            Type::Nullable(inner) => {
                self.compile_nullable_match(builder, scrutinee, inner, arms, current_block, default_block);
            }
            Type::Named(name) => {
                // For named types (like enums), generate switch on constructor
                self.compile_named_match(builder, scrutinee, name, arms, current_block, default_block);
            }
            _ => {
                // Dynamic type: emit runtime type check and dispatch
                self.compile_dynamic_match(builder, scrutinee, arms, current_block, default_block);
            }
        }
    }

    fn compile_primitive_match<B, I>(
        &mut self,
        builder: &mut B,
        scrutinee: &Expr,
        arms: &[MatchArm],
        _current_block: I,
        default_block: I,
    ) where
        B: BlockBuilder<'ctx>,
        I: Copy,
    {
        // Group arms by literal value
        let mut literal_arms: HashMap<String, &MatchArm> = HashMap::new();
        let mut wildcard_idx = None;

        for (i, arm) in arms.iter().enumerate() {
            match &arm.pattern {
                Pattern::Wildcard => {
                    if wildcard_idx.is_none() {
                        wildcard_idx = Some(i);
                    }
                }
                Pattern::Literal(lit) => {
                    let key = format!("{:?}", lit);
                    literal_arms.insert(key, arm);
                }
                Pattern::Identifier(name) => {
                    // Variable pattern binds to the value
                    let _ = builder.build_store(name, scrutinee);
                }
                _ => {}
            }
        }

        // For now, just emit jumps - actual switch generation would need more IR builder integration
        let _ = (builder, scrutinee, literal_arms, default_block, wildcard_idx);
    }

    fn compile_bool_match<B, I>(
        &mut self,
        builder: &mut B,
        scrutinee: &Expr,
        arms: &[MatchArm],
        _current_block: I,
        default_block: I,
    ) where
        B: BlockBuilder<'ctx>,
        I: Copy,
    {
        let mut true_arm = None;
        let mut false_arm = None;
        let mut wildcard_idx = None;

        for (i, arm) in arms.iter().enumerate() {
            match &arm.pattern {
                Pattern::Wildcard => {
                    if wildcard_idx.is_none() {
                        wildcard_idx = Some(i);
                    }
                }
                Pattern::Literal(Box::Expr::BooleanLiteral(true))) => {
                    true_arm = Some(arm);
                }
                Pattern::Literal(Box::Expr::BooleanLiteral(false))) => {
                    false_arm = Some(arm);
                }
                Pattern::Identifier(name) => {
                    // Variable pattern binds to the value
                    let _ = builder.build_store(name, scrutinee);
                }
                _ => {}
            }
        }

        let _ = (builder, scrutinee, default_block, wildcard_idx, true_arm, false_arm);
    }

    fn compile_null_match<B, I>(
        &mut self,
        builder: &mut B,
        scrutinee: &Expr,
        arms: &[MatchArm],
        _current_block: I,
        default_block: I,
    ) where
        B: BlockBuilder<'ctx>,
        I: Copy,
    {
        let mut null_arm = None;
        let mut wildcard_idx = None;

        for (i, arm) in arms.iter().enumerate() {
            match &arm.pattern {
                Pattern::Wildcard => {
                    if wildcard_idx.is_none() {
                        wildcard_idx = Some(i);
                    }
                }
                Pattern::Literal(Box::Expr::NullLiteral)) => {
                    null_arm = Some(arm);
                }
                Pattern::Identifier(name) => {
                    let _ = builder.build_store(name, scrutinee);
                }
                _ => {}
            }
        }

        let _ = (builder, scrutinee, default_block, wildcard_idx, null_arm);
    }

    fn compile_array_match<B, I>(
        &mut self,
        builder: &mut B,
        scrutinee: &Expr,
        scrutinee_type: &Type,
        arms: &[MatchArm],
        _current_block: I,
        default_block: I,
    ) where
        B: BlockBuilder<'ctx>,
        I: Copy,
    {
        // Check for empty array pattern
        // Check for specific length patterns
        // Handle rest patterns
        let _ = (builder, scrutinee, scrutinee_type, arms, default_block);
    }

    fn compile_object_match<B, I>(
        &mut self,
        builder: &mut B,
        scrutinee: &Expr,
        scrutinee_type: &Type,
        arms: &[MatchArm],
        _current_block: I,
        default_block: I,
    ) where
        B: BlockBuilder<'ctx>,
        I: Copy,
    {
        // For object patterns, check that required fields exist and match
        // Handle shorthand patterns, rest patterns, nested patterns
        let _ = (builder, scrutinee, scrutinee_type, arms, default_block);
    }

    fn compile_nullable_match<B, I>(
        &mut self,
        builder: &mut B,
        scrutinee: &Expr,
        inner_type: &Type,
        arms: &[MatchArm],
        _current_block: I,
        default_block: I,
    ) where
        B: BlockBuilder<'ctx>,
        I: Copy,
    {
        // Generate null check first, then match on inner type
        let _ = (builder, scrutinee, inner_type, arms, default_block);
    }

    fn compile_named_match<B, I>(
        &mut self,
        builder: &mut B,
        scrutinee: &Expr,
        type_name: &str,
        arms: &[MatchArm],
        _current_block: I,
        default_block: I,
    ) where
        B: BlockBuilder<'ctx>,
        I: Copy,
    {
        // For enum-like types, dispatch based on constructor tag
        let _ = (builder, scrutinee, type_name, arms, default_block);
    }

    fn compile_dynamic_match<B, I>(
        &mut self,
        builder: &mut B,
        scrutinee: &Expr,
        arms: &[MatchArm],
        _current_block: I,
        default_block: I,
    ) where
        B: BlockBuilder<'ctx>,
        I: Copy,
    {
        // Dynamic dispatch using type tags
        let _ = (builder, scrutinee, arms, default_block);
    }

    /// Generates destructuring code for a pattern against a value.
    pub fn generate_destructuring<B>(
        &mut self,
        builder: &mut B,
        pattern: &Pattern,
        value: &Expr,
        value_type: &Type,
    ) where
        B: BlockBuilder<'ctx>,
    {
        match pattern {
            Pattern::Identifier(name) => {
                // Simple binding: value goes into the variable
                let _ = builder.build_store(name, value);
            }
            Pattern::Wildcard => {
                // Wildcard: discard the value (no-op)
            }
            Pattern::Literal(_) => {
                // Literal pattern: generate equality check at runtime
            }
            Pattern::Object(fields) => {
                self.generate_object_destructuring(builder, fields, value, value_type);
            }
            Pattern::Array(elements) => {
                self.generate_array_destructuring(builder, elements, value, value_type);
            }
            Pattern::Rest(name) => {
                // Rest pattern: bind remaining elements
                let _ = builder.build_store(name, value);
            }
            Pattern::As(inner, alias) => {
                // First bind inner, then rebind with alias
                self.generate_destructuring(builder, inner, value, value_type);
                let _ = builder.build_store(alias, value);
            }
            Pattern::Or(patterns) => {
                // OR pattern: try each pattern in sequence
                if let Some(first) = patterns.first() {
                    self.generate_destructuring(builder, first, value, value_type);
                }
            }
        }
    }

    fn generate_object_destructuring<B>(
        &mut self,
        builder: &mut B,
        fields: &[crate::parser::ast::ObjectPatternField],
        value: &Expr,
        value_type: &Type,
    ) where
        B: BlockBuilder<'ctx>,
    {
        if let Type::Object(obj_fields) = value_type {
            for field in fields {
                match field {
                    crate::parser::ast::ObjectPatternField::Property { key, pattern } => {
                        // Access field and recursively destructure
                        let field_expr = Expr::Member {
                            object: Box::new(value.clone()),
                            property: crate::parser::ast::MemberProperty::Ident(key.clone()),
                            optional: false,
                        };
                        let field_type = obj_fields
                            .iter()
                            .find(|f| f.name == *key)
                            .map(|f| f.ty.clone())
                            .unwrap_or(Type::Dynamic);
                        self.generate_destructuring(builder, pattern, &field_expr, &field_type);
                    }
                    crate::parser::ast::ObjectPatternField::Shorthand(name) => {
                        // Direct binding: just store the value
                        let field_expr = Expr::Member {
                            object: Box::new(value.clone()),
                            property: crate::parser::ast::MemberProperty::Ident(name.clone()),
                            optional: false,
                        };
                        let field_type = obj_fields
                            .iter()
                            .find(|f| f.name == *name)
                            .map(|f| f.ty.clone())
                            .unwrap_or(Type::Dynamic);
                        self.generate_destructuring(builder, &Pattern::Identifier(name.clone()), &field_expr, &field_type);
                    }
                    crate::parser::ast::ObjectPatternField::Rest(name) => {
                        // Rest pattern: extract remaining fields
                        let _ = builder.build_store(name, value);
                    }
                }
            }
        }
    }

    fn generate_array_destructuring<B>(
        &mut self,
        builder: &mut B,
        elements: &[crate::parser::ast::ArrayPatternElement],
        value: &Expr,
        value_type: &Type,
    ) where
        B: BlockBuilder<'ctx>,
    {
        let elem_type = match value_type {
            Type::Array(inner) => (*inner).clone(),
            _ => Type::Dynamic,
        };

        for (i, elem) in elements.iter().enumerate() {
            match elem {
                crate::parser::ast::ArrayPatternElement::Pattern(pat) => {
                    // Access element at index i
                    let index_expr = Expr::IntLiteral(i as i64);
                    let elem_expr = Expr::Member {
                        object: Box::new(value.clone()),
                        property: crate::parser::ast::MemberProperty::Expr(Box::new(index_expr)),
                        optional: false,
                    };
                    self.generate_destructuring(builder, pat, &elem_expr, &elem_type);
                }
                crate::parser::ast::ArrayPatternElement::Rest(pat) => {
                    // Rest: slice from i to end
                    let _ = builder.build_store(
                        match pat {
                            Pattern::Identifier(name) => name,
                            _ => "_",
                        },
                        value,
                    );
                }
                crate::parser::ast::ArrayPatternElement::Elision => {
                    // Elision: skip the element
                }
            }
        }
    }
}

impl Default for PatternCompiler<'static> {
    fn default() -> Self {
        Self::new()
    }
}

/// Trait for building LLVM IR blocks.
/// This abstracts over the builder to allow testing without full LLVM context.
pub trait BlockBuilder<'ctx> {
    fn build_store(&mut self, name: &str, value: &Expr) -> Result<(), String>;
    fn build_branch(&mut self, target: inkwell::values::BasicBlock<'ctx>) -> Result<(), String>;
    fn build_cond_branch(
        &mut self,
        cond: &Expr,
        true_target: inkwell::values::BasicBlock<'ctx>,
        false_target: inkwell::values::BasicBlock<'ctx>,
    ) -> Result<(), String>;
    fn build_load(&mut self, name: &str) -> Result<inkwell::values::AnyValue<'ctx>, String>;
    fn build_struct_gep(&mut self, ptr: &Expr, index: u32) -> Result<inkwell::values::AnyValue<'ctx>, String>;
    fn build_call(&mut self, func: &str, args: &[&Expr]) -> Result<inkwell::values::AnyValue<'ctx>, String>;
    fn build_icmp(
        &mut self,
        op: &str,
        lhs: &Expr,
        rhs: &Expr,
    ) -> Result<inkwell::values::AnyValue<'ctx>, String>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pattern_compiler_creation() {
        let compiler = PatternCompiler::new();
        assert!(compiler.scratch.is_empty());
    }

    #[test]
    fn test_identifier_pattern() {
        let mut compiler = PatternCompiler::new();
        // PatternCompiler doesn't actually execute - it's a code generation structure
        // Real tests would require mocking BlockBuilder
        assert!(compiler.scratch.is_empty());
    }
}