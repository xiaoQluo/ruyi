use crate::typechecker::types::Type;
/**
 * LLVM type mapping for Ruyi types.
 *
 * Maps Ruyi's gradual type system to LLVM IR types.
 *
 * | Ruyi  | LLVM                |
 * |--------|---------------------|
 * | int    | i64                 |
 * | float  | f64                 |
 * | bool   | i1                  |
 * | string | *i8                 |
 * | null   | *i8 (null)          |
 * | void   | void                |
 * | dyn    | { i64, i8* }        |
 * | dyn T  | { i8*, i8* }        |
 * | never  | void (poison)       |
 *
 * @author Ruyi Team
 * @date 2026-05-01
 */
use inkwell::context::Context;
use inkwell::types::{
    BasicType, BasicTypeEnum, FloatType, FunctionType, IntType, PointerType, StructType, VoidType,
};

/// Map a Ruyi `Type` to its LLVM `BasicTypeEnum` equivalent.
pub fn ruyi_type_to_llvm<'ctx>(context: &'ctx Context, ty: &Type) -> BasicTypeEnum<'ctx> {
    match ty {
        Type::Int => BasicTypeEnum::IntType(context.i64_type()),
        Type::Float => BasicTypeEnum::FloatType(context.f64_type()),
        Type::Bool => BasicTypeEnum::IntType(context.bool_type()),
        Type::String => BasicTypeEnum::PointerType(context.i8_type().ptr_type(Default::default())),
        Type::Null => BasicTypeEnum::PointerType(context.i8_type().ptr_type(Default::default())),
        Type::Void | Type::Never => {
            // void/never cannot be BasicTypeEnum; use i8 as placeholder
            BasicTypeEnum::IntType(context.i8_type())
        }
        Type::BigInt => BasicTypeEnum::PointerType(context.i8_type().ptr_type(Default::default())),
        Type::Nullable(inner) => ruyi_type_to_llvm(context, inner),
        Type::Array(_) => {
            BasicTypeEnum::PointerType(context.i8_type().ptr_type(Default::default()))
        }
        Type::Object(_) => {
            BasicTypeEnum::PointerType(context.i8_type().ptr_type(Default::default()))
        }
        Type::Function { .. } => {
            BasicTypeEnum::PointerType(context.i8_type().ptr_type(Default::default()))
        }
        Type::Named(_) => {
            BasicTypeEnum::PointerType(context.i8_type().ptr_type(Default::default()))
        }
        Type::Generic { .. } => {
            BasicTypeEnum::PointerType(context.i8_type().ptr_type(Default::default()))
        }
        Type::TypeVar(_) => {
            BasicTypeEnum::PointerType(context.i8_type().ptr_type(Default::default()))
        }
        Type::Trait(_) => {
            let trait_obj_type = context.struct_type(
                &[
                    context.i8_type().ptr_type(Default::default()).into(),
                    context.i8_type().ptr_type(Default::default()).into(),
                ],
                false,
            );
            BasicTypeEnum::StructType(trait_obj_type)
        }
        Type::Future(_) => {
            BasicTypeEnum::PointerType(context.i8_type().ptr_type(Default::default()))
        }
        Type::Dynamic => {
            let dyn_type = context.struct_type(
                &[
                    context.i64_type().into(),
                    context.i8_type().ptr_type(Default::default()).into(),
                ],
                false,
            );
            BasicTypeEnum::StructType(dyn_type)
        }
        Type::Error => BasicTypeEnum::IntType(context.i8_type()),
    }
}

/// Get the LLVM type for a Ruyi function signature.
pub fn function_type_from_ruyi<'ctx>(
    context: &'ctx Context,
    params: &[Type],
    return_type: &Type,
) -> FunctionType<'ctx> {
    let param_types: Vec<BasicTypeEnum<'ctx>> = params
        .iter()
        .map(|p| ruyi_type_to_llvm(context, p))
        .collect();
    let param_refs: Vec<_> = param_types.iter().map(|t| (*t).into()).collect();

    match return_type {
        Type::Void | Type::Never => context.void_type().fn_type(&param_refs, false),
        other => {
            let ret = ruyi_type_to_llvm(context, other);
            ret.fn_type(&param_refs, false)
        }
    }
}

/// Convenience accessors for common LLVM types used in codegen.
pub struct LlvmTypes<'ctx> {
    pub context: &'ctx Context,
}

impl<'ctx> LlvmTypes<'ctx> {
    pub fn new(context: &'ctx Context) -> Self {
        Self { context }
    }

    pub fn int_type(&self) -> IntType<'ctx> {
        self.context.i64_type()
    }

    pub fn float_type(&self) -> FloatType<'ctx> {
        self.context.f64_type()
    }

    pub fn bool_type(&self) -> IntType<'ctx> {
        self.context.bool_type()
    }

    pub fn ptr_type(&self) -> PointerType<'ctx> {
        self.context.i8_type().ptr_type(Default::default())
    }

    pub fn void_type(&self) -> VoidType<'ctx> {
        self.context.void_type()
    }

    pub fn dyn_type(&self) -> StructType<'ctx> {
        self.context.struct_type(
            &[
                self.context.i64_type().into(),
                self.context.i8_type().ptr_type(Default::default()).into(),
            ],
            false,
        )
    }

    /// Get the LLVM type for a Ruyi object (class instance).
    /// Struct layout: [type_tag: i64, field_count: i64, field_storage: i8*]
    pub fn ruyi_object_type(&self, field_count: u32) -> StructType<'ctx> {
        self.context.struct_type(
            &[
                self.context.i64_type().into(), // type_tag (gc vtable pointer)
                self.context.i64_type().into(), // field_count
                self.context.i8_type().ptr_type(Default::default()).into(), // field storage (opaque)
            ],
            false,
        )
    }

    /// Get the LLVM type for a Ruyi array.
    /// Struct layout: [length: i64, capacity: i64, elements: i8*]
    pub fn ruyi_array_type(&self) -> StructType<'ctx> {
        self.context.struct_type(
            &[
                self.context.i64_type().into(),                             // length
                self.context.i64_type().into(),                             // capacity
                self.context.i8_type().ptr_type(Default::default()).into(), // element storage
            ],
            false,
        )
    }

    /// Get the LLVM type for a Ruyi bigint (arbitrary precision).
    /// Currently modeled as a pointer to opaque bigint data.
    pub fn ruyi_bigint_type(&self) -> PointerType<'ctx> {
        self.context.i8_type().ptr_type(Default::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_primitive_types() {
        let context = Context::create();
        assert!(matches!(
            ruyi_type_to_llvm(&context, &Type::Int),
            BasicTypeEnum::IntType(_)
        ));
        assert!(matches!(
            ruyi_type_to_llvm(&context, &Type::Float),
            BasicTypeEnum::FloatType(_)
        ));
        assert!(matches!(
            ruyi_type_to_llvm(&context, &Type::Bool),
            BasicTypeEnum::IntType(_)
        ));
        assert!(matches!(
            ruyi_type_to_llvm(&context, &Type::String),
            BasicTypeEnum::PointerType(_)
        ));
    }

    #[test]
    fn test_dynamic_type() {
        let context = Context::create();
        let ty = ruyi_type_to_llvm(&context, &Type::Dynamic);
        assert!(matches!(ty, BasicTypeEnum::StructType(_)));
    }

    #[test]
    fn test_function_type() {
        let context = Context::create();
        let fn_ty = function_type_from_ruyi(&context, &[Type::Int, Type::Int], &Type::Int);
        assert_eq!(fn_ty.count_param_types(), 2);
    }
}
