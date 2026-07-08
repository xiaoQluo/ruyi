/**
 * Trait-based code generation for Ruyi.
 *
 * Generates vtables, trait objects, and dispatch code for
 * static and dynamic trait method resolution.
 *
 * @author Ruyi Team
 * @date 2026-05-01
 */
use std::collections::HashMap;

use inkwell::types::BasicTypeEnum;
use inkwell::values::{BasicValueEnum, PointerValue};

use crate::codegen::generator::CodegenContext;
use crate::codegen::types::ruyi_type_to_llvm;
use crate::parser::ast::Declaration;
use crate::typechecker::types::Type;

/// VTable info for a trait implementation.
#[derive(Debug, Clone)]
pub struct VTableInfo<'ctx> {
    pub trait_name: String,
    pub for_type: String,
    pub vtable_type: inkwell::types::StructType<'ctx>,
    pub vtable_global: inkwell::values::GlobalValue<'ctx>,
    pub method_indices: HashMap<String, usize>,
}

/// Trait object layout: fat pointer (data + vtable).
#[derive(Debug, Clone)]
pub struct TraitObject<'ctx> {
    pub data: PointerValue<'ctx>,
    pub vtable: PointerValue<'ctx>,
    pub trait_name: String,
}

/// VTable registry for the current module.
#[derive(Debug, Clone, Default)]
pub struct VTableRegistry<'ctx> {
    vtables: HashMap<(String, String), VTableInfo<'ctx>>,
}

impl<'ctx> VTableRegistry<'ctx> {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a vtable for a (trait, concrete type) pair.
    pub fn register_vtable(&mut self, info: VTableInfo<'ctx>) {
        self.vtables
            .insert((info.trait_name.clone(), info.for_type.clone()), info);
    }

    /// Get vtable info for a (trait, type) pair.
    pub fn get_vtable(&self, trait_name: &str, for_type: &str) -> Option<&VTableInfo<'ctx>> {
        self.vtables
            .get(&(trait_name.to_string(), for_type.to_string()))
    }

    /// Check if a vtable exists.
    pub fn has_vtable(&self, trait_name: &str, for_type: &str) -> bool {
        self.vtables
            .contains_key(&(trait_name.to_string(), for_type.to_string()))
    }

    /// Get any vtable registered for a trait (used for method index / type layout).
    pub fn get_trait_vtable(&self, trait_name: &str) -> Option<&VTableInfo<'ctx>> {
        self.vtables.values().find(|v| v.trait_name == trait_name)
    }
}

/// Generate vtables for all impl declarations in a program.
pub fn generate_vtables<'ctx>(
    ctx: &mut CodegenContext<'ctx, '_, '_>,
    program: &crate::parser::ast::Program,
) -> VTableRegistry<'ctx> {
    let mut registry = VTableRegistry::new();

    for item in &program.items {
        if let crate::parser::ast::ModuleItem::Declaration(Declaration::Impl {
            trait_name,
            for_type,
            body,
            ..
        }) = item
        {
            let for_type_str = type_annotation_to_string(for_type);
            if for_type_str.is_empty() {
                continue;
            }

            let method_names: Vec<String> = body
                .iter()
                .filter_map(|elem| {
                    if let crate::parser::ast::ClassElement::Method { name, .. } = elem {
                        match name {
                            crate::parser::ast::PropertyName::Ident(n) => Some(n.clone()),
                            _ => None,
                        }
                    } else {
                        None
                    }
                })
                .collect();

            if method_names.is_empty() {
                continue;
            }

            let i8_ptr = ctx.context.i8_type().ptr_type(Default::default());
            let method_ptr_type = i8_ptr
                .fn_type(&[i8_ptr.into()], false)
                .ptr_type(Default::default());

            let vtable_fields: Vec<BasicTypeEnum<'ctx>> = method_names
                .iter()
                .map(|_| BasicTypeEnum::PointerType(method_ptr_type))
                .collect();

            let vtable_type = ctx.context.struct_type(&vtable_fields, false);
            let global_name = format!("vtable_{}_for_{}", trait_name, for_type_str);
            let vtable_global = ctx.module.add_global(vtable_type, None, &global_name);

            let mut method_indices = HashMap::new();
            for (i, name) in method_names.iter().enumerate() {
                method_indices.insert(name.clone(), i);
            }

            registry.register_vtable(VTableInfo {
                trait_name: trait_name.clone(),
                for_type: for_type_str,
                vtable_type,
                vtable_global,
                method_indices,
            });
        }
    }

    registry
}

/// Emit vtable initializers after all functions have been compiled.
pub fn emit_vtable_initializers<'ctx>(
    ctx: &mut CodegenContext<'ctx, '_, '_>,
    registry: &VTableRegistry<'ctx>,
) {
    for vtable in registry.vtables.values() {
        let func_ptrs: Vec<BasicValueEnum<'ctx>> = vtable
            .method_indices
            .keys()
            .map(|method_name| {
                let func_name = format!(
                    "{}_{}_for_{}",
                    method_name, vtable.trait_name, vtable.for_type
                );
                if let Some(func) = ctx.module.get_function(&func_name) {
                    BasicValueEnum::PointerValue(func.as_global_value().as_pointer_value())
                } else {
                    BasicValueEnum::PointerValue(
                        ctx.context
                            .i8_type()
                            .ptr_type(Default::default())
                            .const_null(),
                    )
                }
            })
            .collect();

        if !func_ptrs.is_empty() {
            let struct_val = vtable.vtable_type.const_named_struct(&func_ptrs);
            vtable.vtable_global.set_initializer(&struct_val);
        }
    }
}

/// Create a trait object (fat pointer) from a concrete value.
pub fn create_trait_object<'ctx>(
    ctx: &mut CodegenContext<'ctx, '_, '_>,
    registry: &VTableRegistry<'ctx>,
    value: BasicValueEnum<'ctx>,
    value_ty: &Type,
    trait_name: &str,
) -> Option<TraitObject<'ctx>> {
    let type_name = match value_ty {
        Type::Named(name, _) | Type::Generic { base: name, .. } => name.clone(),
        _ => return None,
    };

    let vtable = registry.get_vtable(trait_name, &type_name)?;

    let data_ptr = if value.is_pointer_value() {
        value.into_pointer_value()
    } else {
        let alloca = ctx
            .builder
            .build_alloca(ruyi_type_to_llvm(ctx.context, value_ty), "trait_data");
        ctx.builder().build_store(alloca, value);
        alloca
    };

    let void_data = ctx.builder().build_bitcast(
        data_ptr,
        ctx.context.i8_type().ptr_type(Default::default()),
        "data_void",
    );

    let vtable_ptr = ctx.builder().build_bitcast(
        vtable.vtable_global.as_pointer_value(),
        ctx.context.i8_type().ptr_type(Default::default()),
        "vtable_ptr",
    );

    Some(TraitObject {
        data: void_data.into_pointer_value(),
        vtable: vtable_ptr.into_pointer_value(),
        trait_name: trait_name.to_string(),
    })
}

/// Perform a dynamic dispatch call through a trait object.
pub fn build_dynamic_dispatch<'ctx>(
    ctx: &mut CodegenContext<'ctx, '_, '_>,
    registry: &VTableRegistry<'ctx>,
    trait_obj: &TraitObject<'ctx>,
    method_name: &str,
    args: &[BasicValueEnum<'ctx>],
) -> Result<BasicValueEnum<'ctx>, String> {
    let vtable = registry
        .get_trait_vtable(&trait_obj.trait_name)
        .ok_or_else(|| format!("No vtable registered for trait {}", trait_obj.trait_name))?;

    let idx = vtable.method_indices.get(method_name).ok_or_else(|| {
        format!(
            "Method {} not found in trait {}",
            method_name, trait_obj.trait_name
        )
    })?;

    let vtable_ptr_type = vtable.vtable_type.ptr_type(Default::default());
    let vtable_typed = ctx
        .builder
        .build_bitcast(trait_obj.vtable, vtable_ptr_type, "vtable_typed");

    let vtable_loaded = ctx
        .builder
        .build_load(vtable_typed.into_pointer_value(), "vtable_loaded");

    let method_ptr = match ctx.builder().build_extract_value(
        vtable_loaded.into_struct_value(),
        *idx as u32,
        &format!("method_{}", method_name),
    ) {
        Some(val) => val,
        None => {
            return Err(format!(
                "Failed to extract method {} from vtable",
                method_name
            ))
        }
    };

    let mut call_args: Vec<inkwell::values::BasicMetadataValueEnum<'ctx>> =
        vec![trait_obj.data.into()];
    for arg in args {
        call_args.push((*arg).into());
    }

    let call_site = {
        let fn_ptr = method_ptr.into_pointer_value();
        let callable: inkwell::values::CallableValue<'ctx> = fn_ptr
            .try_into()
            .map_err(|_| format!("Failed to create callable for method {}", method_name))?;
        ctx.builder()
            .build_call(callable, &call_args, &format!("dyn_call_{}", method_name))
    };

    let value = call_site.try_as_basic_value().left();
    match value {
        Some(v) => Ok(v),
        None => Ok(BasicValueEnum::IntValue(
            ctx.context.i8_type().const_int(0, false),
        )),
    }
}

fn type_annotation_to_string(annotation: &crate::parser::ast::TypeAnnotation) -> String {
    match annotation {
        crate::parser::ast::TypeAnnotation::Identifier(name) => name.clone(),
        crate::parser::ast::TypeAnnotation::Generic { base, .. } => base.clone(),
        _ => String::new(),
    }
}
