/**
 * Declaration code generation for Ruyi.
 *
 * Lowers Ruyi AST declarations (functions, variables, classes) to LLVM IR.
 *
 * @author Ruyi Team
 * @date 2026-05-01
 */
use inkwell::values::BasicValueEnum;

use super::builtins::is_gc_managed;
use super::expr::compile_expr;
use super::generator::CodegenContext;
use super::stmt::compile_block;
use super::types::{function_type_from_ruyi, ruyi_type_to_llvm};
use crate::parser::ast::{Binding, ClassElement, Declaration, Pattern, PropertyName};
use crate::typechecker::types::Type;

/// Compile a declaration.
pub fn compile_declaration<'ctx>(
    ctx: &mut CodegenContext<'ctx, '_>,
    decl: &Declaration,
) -> Result<(), String> {
    match decl {
        Declaration::Let(bindings) | Declaration::Const(bindings) => {
            for binding in bindings {
                compile_binding(ctx, binding)?;
            }
            Ok(())
        }
        Declaration::Function {
            name,
            params,
            return_type,
            body,
            is_async,
            ..
        } => {
            if *is_async {
                super::async_codegen::compile_async_function(
                    ctx,
                    name,
                    params,
                    return_type.as_ref(),
                    body,
                )
            } else {
                compile_function(ctx, name, params, return_type.as_ref(), body)
            }
        }
        Declaration::Class { name, body, .. } => compile_class(ctx, name, body),
        Declaration::Impl {
            trait_name,
            for_type,
            body,
            ..
        } => compile_impl(ctx, trait_name, for_type, body),
        Declaration::Trait { .. } => Ok(()),
        _ => Err(format!("Unsupported declaration: {:?}", decl)),
    }
}

fn compile_binding<'ctx>(
    ctx: &mut CodegenContext<'ctx, '_>,
    binding: &Binding,
) -> Result<(), String> {
    let name = match &binding.pattern {
        Pattern::Identifier(n) => n.clone(),
        _ => return Err("Complex patterns not yet supported".to_string()),
    };

    // Determine type: use annotation if present, otherwise infer from init expression
    let (ty, llvm_ty, ptr) = if let Some(annotation) = &binding.ty {
        let ty = Type::from_annotation(annotation);
        let llvm_ty = ruyi_type_to_llvm(ctx.context, &ty);
        let ptr = ctx.builder.build_alloca(llvm_ty, &name);
        (ty, llvm_ty, ptr)
    } else if let Some(init) = &binding.init {
        // Infer type from initialization expression
        let init_result = compile_expr(ctx, init)?;
        let ty = init_result.ty;
        let llvm_ty = ruyi_type_to_llvm(ctx.context, &ty);
        let ptr = ctx.builder.build_alloca(llvm_ty, &name);
        ctx.builder.build_store(ptr, init_result.value);
        ctx.variables.insert(name, (ptr, ty));
        return Ok(());
    } else {
        let ty = Type::Dynamic;
        let llvm_ty = ruyi_type_to_llvm(ctx.context, &ty);
        let ptr = ctx.builder.build_alloca(llvm_ty, &name);
        (ty, llvm_ty, ptr)
    };

    if let Some(init) = &binding.init {
        let init_result = compile_expr(ctx, init)?;
        ctx.builder.build_store(ptr, init_result.value);
    }

    if is_gc_managed(&ty) {
        ctx.add_gc_root(ptr, ty.clone());
    }

    ctx.variables.insert(name, (ptr, ty));
    Ok(())
}

fn compile_function<'ctx>(
    ctx: &mut CodegenContext<'ctx, '_>,
    name: &str,
    params: &[crate::parser::ast::Param],
    return_type: Option<&crate::parser::ast::TypeAnnotation>,
    body: &[crate::parser::ast::Statement],
) -> Result<(), String> {
    let param_types: Vec<Type> = params
        .iter()
        .map(|p| {
            p.ty.as_ref()
                .map(Type::from_annotation)
                .unwrap_or(Type::Dynamic)
        })
        .collect();

    let ret_type = return_type.map(Type::from_annotation).unwrap_or(Type::Void);

    let fn_type = function_type_from_ruyi(ctx.context, &param_types, &ret_type);

    let function = ctx.module.add_function(name, fn_type, None);

    // Save current function and builder position
    let prev_function = ctx.current_function;
    ctx.current_function = Some(function);

    // Create entry basic block
    let entry_bb = ctx.context.append_basic_block(function, "entry");
    let prev_block = ctx.builder.get_insert_block();
    ctx.builder.position_at_end(entry_bb);

    // Save previous variables and create new scope
    let mut prev_vars = std::collections::HashMap::new();

    ctx.push_gc_root_scope();

    // Allocate parameters
    for (i, param) in params.iter().enumerate() {
        let param_name = match &param.pattern {
            Pattern::Identifier(n) => n.clone(),
            _ => format!("param_{}", i),
        };

        let param_ty = param_types.get(i).cloned().unwrap_or(Type::Dynamic);
        let llvm_ty = ruyi_type_to_llvm(ctx.context, &param_ty);
        let ptr = ctx.builder.build_alloca(llvm_ty, &param_name);

        let param_value = function
            .get_nth_param(i as u32)
            .ok_or_else(|| format!("Missing parameter {}", i))?;
        ctx.builder.build_store(ptr, param_value);

        if is_gc_managed(&param_ty) {
            ctx.add_gc_root(ptr, param_ty.clone());
        }

        if let Some(old) = ctx.variables.insert(param_name.clone(), (ptr, param_ty)) {
            prev_vars.insert(param_name, old);
        }
    }

    // Compile function body
    let result = compile_block(ctx, body);

    // Ensure the function has a terminator
    let current_bb = ctx.builder.get_insert_block().unwrap();
    if current_bb.get_terminator().is_none() {
        ctx.emit_gc_root_removals();
        if ret_type == Type::Void {
            ctx.builder.build_return(None);
        } else {
            let default_val = match ret_type {
                Type::Int => BasicValueEnum::IntValue(ctx.context.i64_type().const_int(0, true)),
                Type::Float => BasicValueEnum::FloatValue(ctx.context.f64_type().const_float(0.0)),
                Type::Bool => BasicValueEnum::IntValue(ctx.context.bool_type().const_int(0, false)),
                _ => BasicValueEnum::PointerValue(
                    ctx.context
                        .i8_type()
                        .ptr_type(Default::default())
                        .const_null(),
                ),
            };
            ctx.builder.build_return(Some(&default_val));
        }
    }

    ctx.pop_gc_root_scope();

    // Restore previous state
    ctx.current_function = prev_function;
    if let Some(block) = prev_block {
        ctx.builder.position_at_end(block);
    }

    // Restore variables
    for (name, old) in prev_vars {
        ctx.variables.insert(name, old);
    }

    result
}

fn compile_class<'ctx>(
    ctx: &mut CodegenContext<'ctx, '_>,
    name: &str,
    body: &[ClassElement],
) -> Result<(), String> {
    let mut fields: Vec<(String, Type)> = Vec::new();
    let mut methods: Vec<&ClassElement> = Vec::new();

    for element in body {
        match element {
            ClassElement::Field {
                name: prop_name,
                ty,
                is_static: false,
                ..
            } => {
                let field_name = match prop_name {
                    PropertyName::Ident(n) => n.clone(),
                    _ => continue,
                };
                let field_ty = ty
                    .as_ref()
                    .map(Type::from_annotation)
                    .unwrap_or(Type::Dynamic);
                fields.push((field_name, field_ty));
            }
            ClassElement::Method {
                is_static: false, ..
            } => {
                methods.push(element);
            }
            _ => {}
        }
    }

    ctx.class_fields.insert(name.to_string(), fields.clone());

    // Create the LLVM struct type for this class
    let field_types: Vec<_> = fields
        .iter()
        .map(|(_, ty)| super::types::ruyi_type_to_llvm(ctx.context, ty))
        .collect();
    let struct_type = ctx.context.struct_type(&field_types, false);
    ctx.class_struct_types.insert(name.to_string(), struct_type);

    for element in methods {
        if let ClassElement::Method {
            name: prop_name,
            params,
            return_type,
            body: method_body,
            is_async,
            ..
        } = element
        {
            let method_name = match prop_name {
                PropertyName::Ident(n) => format!("{}_{}", name, n),
                _ => continue,
            };

            let mut method_params = vec![crate::parser::ast::Param {
                pattern: Pattern::Identifier("self".to_string()),
                ty: Some(crate::parser::ast::TypeAnnotation::Identifier(
                    name.to_string(),
                )),
                init: None,
                is_rest: false,
            }];
            method_params.extend(params.iter().filter(|p| {
                !matches!(&p.pattern, Pattern::Identifier(n) if n == "self")
            }).cloned());

            if *is_async {
                super::async_codegen::compile_async_function(
                    ctx,
                    &method_name,
                    &method_params,
                    return_type.as_ref(),
                    method_body,
                )?;
            } else {
                compile_function(
                    ctx,
                    &method_name,
                    &method_params,
                    return_type.as_ref(),
                    method_body,
                )?;
            }
        }
    }

    Ok(())
}

fn compile_impl<'ctx>(
    ctx: &mut CodegenContext<'ctx, '_>,
    trait_name: &str,
    for_type: &crate::parser::ast::TypeAnnotation,
    body: &[crate::parser::ast::ClassElement],
) -> Result<(), String> {
    let for_type_str = match for_type {
        crate::parser::ast::TypeAnnotation::Identifier(name) => name.clone(),
        crate::parser::ast::TypeAnnotation::Generic { base, .. } => base.clone(),
        _ => "dyn".to_string(),
    };

    for element in body {
        if let crate::parser::ast::ClassElement::Method {
            name: prop_name,
            params,
            return_type,
            body: method_body,
            is_async,
            ..
        } = element
        {
            let method_name = match prop_name {
                crate::parser::ast::PropertyName::Ident(n) => n.clone(),
                _ => continue,
            };
            let mangled_name = format!("{}_{}_for_{}", method_name, trait_name, for_type_str);

            let impl_params: Vec<_> = std::iter::once(crate::parser::ast::Param {
                pattern: Pattern::Identifier("self".to_string()),
                ty: Some(for_type.clone()),
                init: None,
                is_rest: false,
            })
            .chain(params.iter().filter(|p| {
                !matches!(&p.pattern, Pattern::Identifier(n) if n == "self")
            }).cloned())
            .collect();

            if *is_async {
                super::async_codegen::compile_async_function(
                    ctx,
                    &mangled_name,
                    &impl_params,
                    return_type.as_ref(),
                    method_body,
                )?;
            } else {
                compile_function(
                    ctx,
                    &mangled_name,
                    &impl_params,
                    return_type.as_ref(),
                    method_body,
                )?;
            }
        }
    }
    Ok(())
}
