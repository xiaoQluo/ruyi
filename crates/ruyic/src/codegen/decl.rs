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
use super::generator::{ruyi_type_to_zero, CodegenContext};
use super::stmt::compile_block;
use super::types::{function_type_from_ruyi, ruyi_type_to_llvm};
use crate::parser::ast::{Binding, ClassElement, Declaration, Expr, Pattern, PropertyName};
use crate::typechecker::types::Type;
use ruyi_exception::landing_pad::LandingPadGenerator;

/// Compile a declaration.
pub fn compile_declaration<'ctx>(
    ctx: &mut CodegenContext<'ctx, '_, '_>,
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
                compile_function(ctx, name, params, return_type.as_ref(), None, None, body)
            }
        }
        Declaration::Class {
            name,
            type_params,
            extends,
            body,
            ..
        } => compile_class(ctx, name, type_params, extends.as_deref(), body),
        Declaration::Impl {
            type_params,
            trait_name,
            for_type,
            body,
            ..
        } => compile_impl(ctx, type_params, trait_name, for_type, body),
        Declaration::Trait { .. } => Ok(()),
        Declaration::Macro { .. } => Ok(()),
        Declaration::TypeAlias { .. } => Ok(()),
        Declaration::ExternFn { .. } => Ok(()),
    }
}

fn compile_simple_binding<'ctx>(
    ctx: &mut CodegenContext<'ctx, '_, '_>,
    name: &str,
    binding: &Binding,
) -> Result<(), String> {
    let (ty, _llvm_ty, ptr) = if let Some(annotation) = &binding.ty {
        let ty = Type::from_annotation(annotation);
        let llvm_ty = ruyi_type_to_llvm(ctx.context, &ty);
        let ptr = ctx.builder().build_alloca(llvm_ty, name);
        (ty, llvm_ty, ptr)
    } else if let Some(init) = &binding.init {
        let init_result = compile_expr(ctx, init)?;
        let ty = init_result.ty;
        let llvm_ty = ruyi_type_to_llvm(ctx.context, &ty);
        let ptr = ctx.builder().build_alloca(llvm_ty, name);
        ctx.builder().build_store(ptr, init_result.value);
        ctx.define_variable(name.to_string(), (ptr, ty.clone()));
        if is_gc_managed(&ty) {
            ctx.add_gc_root(ptr, ty);
        }
        return Ok(());
    } else {
        let ty = Type::Dynamic;
        let llvm_ty = ruyi_type_to_llvm(ctx.context, &ty);
        let ptr = ctx.builder().build_alloca(llvm_ty, name);
        (ty, llvm_ty, ptr)
    };

    if let Some(init) = &binding.init {
        let prev_expected = ctx.expected_expr_type().cloned();
        ctx.set_expected_expr_type(Some(ty.clone()));
        let init_result = super::expr::compile_expr(ctx, init)?;
        ctx.set_expected_expr_type(prev_expected);
        ctx.builder().build_store(ptr, init_result.value);
    }

    if is_gc_managed(&ty) {
        ctx.add_gc_root(ptr, ty.clone());
    }

    ctx.define_variable(name.to_string(), (ptr, ty));
    Ok(())
}

fn compile_array_destructure<'ctx>(
    ctx: &mut CodegenContext<'ctx, '_, '_>,
    binding: &Binding,
    elements: &[crate::parser::ast::ArrayPatternElement],
) -> Result<(), String> {
    let init = binding
        .init
        .as_ref()
        .ok_or("Array destructuring requires an initializer")?;
    let result = compile_expr(ctx, init)?;
    let arr_ptr = match result.value {
        BasicValueEnum::PointerValue(p) => p,
        _ => return Err("Array destructuring requires an array value".to_string()),
    };
    let get_fn = ctx
        .module
        .get_function("__builtin_array_get")
        .ok_or("__builtin_array_get not declared")?;
    let mut i = 0u64;
    for element in elements.iter() {
        match element {
            crate::parser::ast::ArrayPatternElement::Elision => {
                i += 1;
                continue;
            }
            crate::parser::ast::ArrayPatternElement::Pattern(pat)
            | crate::parser::ast::ArrayPatternElement::Default(pat, _) => {
                let name = match pat {
                    crate::parser::ast::Pattern::Identifier(n) => n.clone(),
                    _ => return Err("Nested destructuring patterns not yet supported".to_string()),
                };
                let idx_val = ctx.context.i64_type().const_int(i, false);
                let elem_val = ctx
                    .builder()
                    .build_call(get_fn, &[arr_ptr.into(), idx_val.into()], "destr_elem")
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_int_value();
                let llvm_ty = ctx.context.i64_type();
                let ptr = ctx.builder().build_alloca(llvm_ty, &name);
                ctx.builder().build_store(ptr, elem_val);
                ctx.define_variable(name, (ptr, Type::Int));
                i += 1;
            }
            _ => return Err("Nested destructuring patterns not yet supported".to_string()),
        }
    }
    Ok(())
}

fn compile_binding<'ctx>(
    ctx: &mut CodegenContext<'ctx, '_, '_>,
    binding: &Binding,
) -> Result<(), String> {
    match &binding.pattern {
        Pattern::Identifier(n) => {
            let name = n.clone();
            compile_simple_binding(ctx, &name, binding)?;
        }
        Pattern::Array(elements) => {
            compile_array_destructure(ctx, binding, elements)?;
        }
        _ => return Err("Complex patterns not yet supported".to_string()),
    }
    Ok(())
}

pub fn predeclare_function<'ctx>(
    ctx: &mut CodegenContext<'ctx, '_, '_>,
    name: &str,
    params: &[crate::parser::ast::Param],
    return_type: Option<&crate::parser::ast::TypeAnnotation>,
) {
    let param_types: Vec<Type> = params
        .iter()
        .map(|p| {
            let mut ty = p
                .ty.as_ref()
                .map(Type::from_annotation)
                .unwrap_or(Type::Dynamic);
            // Wrap rest params as Array<T> so the definition-side parameter
            // type matches the call-site packaging (compile_rest_args_to_array
            // passes an array pointer) and the typechecker's inference.
            if p.is_rest && !matches!(&ty, Type::Array(_)) {
                ty = Type::Array(Box::new(ty));
            }
            ty
        })
        .collect();

    let ret_type = return_type.map(Type::from_annotation).unwrap_or(Type::Void);

    let fn_type = function_type_from_ruyi(ctx.context, &param_types, &ret_type);

    let ruyi_fn_type = Type::Function {
        params: param_types,
        return_type: Box::new(ret_type),
    };
    ctx.function_types.insert(name.to_string(), ruyi_fn_type);

    ctx.module.add_function(name, fn_type, None);
}

pub fn compile_function<'ctx>(
    ctx: &mut CodegenContext<'ctx, '_, '_>,
    name: &str,
    params: &[crate::parser::ast::Param],
    return_type: Option<&crate::parser::ast::TypeAnnotation>,
    inferred_param_types: Option<&[Type]>,
    inferred_return_type: Option<&Type>,
    body: &[crate::parser::ast::Statement],
) -> Result<(), String> {
    let param_types: Vec<Type> = inferred_param_types
        .map(|types| types.to_vec())
        .unwrap_or_else(|| {
            params
                .iter()
                .map(|p| {
                    let mut ty = p
                        .ty.as_ref()
                        .map(Type::from_annotation)
                        .unwrap_or(Type::Dynamic);
                    // Wrap rest params as Array<T> so the definition-side
                    // parameter type matches the call-site packaging
                    // (compile_rest_args_to_array passes an array pointer)
                    // and the typechecker's inference.
                    if p.is_rest && !matches!(&ty, Type::Array(_)) {
                        ty = Type::Array(Box::new(ty));
                    }
                    ty
                })
                .collect()
        });

    let ret_type = inferred_return_type
        .cloned()
        .or_else(|| return_type.map(Type::from_annotation))
        .unwrap_or(Type::Void);

    let fn_type = function_type_from_ruyi(ctx.context, &param_types, &ret_type);

    let ruyi_fn_type = Type::Function {
        params: param_types.clone(),
        return_type: Box::new(ret_type.clone()),
    };
    ctx.function_types.insert(name.to_string(), ruyi_fn_type);

    // Register rest parameter info for call-site argument packaging
    for (i, param) in params.iter().enumerate() {
        if param.is_rest {
            let elem_ty = match &param.ty {
                Some(crate::parser::ast::TypeAnnotation::Generic { base, args })
                    if base == "Array" && args.len() == 1 =>
                {
                    Type::from_annotation(&args[0])
                }
                _ => Type::Dynamic,
            };
            ctx.rest_params.insert(name.to_string(), (i, elem_ty));
            break;
        }
    }

    let function = if let Some(existing) = ctx.module.get_function(name) {
        existing
    } else {
        ctx.module.add_function(name, fn_type, None)
    };

    // Set personality function so the C++ exception runtime can unwind through
    // every Ruyi function frame. Without this, functions without a landingpad
    // instruction cannot be unwound through when a callee throws.
    use inkwell::attributes::{Attribute, AttributeLoc};
    let lp_gen = LandingPadGenerator::new(ctx.context, ctx.module, ctx.builder());
    let personality = lp_gen.get_personality_function();
    function.set_personality_function(personality);

    // uwtable forces LLVM to emit full .eh_frame with personality/LSDA,
    // which the platform unwinder needs to match landingpad handlers.
    let uwtable_id = Attribute::get_named_enum_kind_id("uwtable");
    let uwtable_attr = ctx.context.create_enum_attribute(uwtable_id, 0);
    function.add_attribute(AttributeLoc::Function, uwtable_attr);

    // Save current function and builder position
    let prev_function = ctx.current_function();
    let prev_return_type = ctx.current_return_type().cloned();
    ctx.set_current_function(Some(function));
    ctx.set_current_return_type(Some(ret_type.clone()));

    // Create entry basic block
    let entry_bb = ctx.context.append_basic_block(function, "entry");
    let prev_block = ctx.builder().get_insert_block();
    ctx.builder().position_at_end(entry_bb);

    // Snapshot the variable map: parameters and body lets must not leak
    // into later compilations (locals shadow globals, so a leaked binding
    // would mask a same-named global with a dangling alloca).
    let saved_vars = ctx.variables.clone();

    // SAFETY: pop_gc_root_scope guaranteed by GcRootGuard Drop.
    // The previous bare `?` on `get_nth_param` could leak the scope on
    // the defensive error path; the RAII guard closes that gap and any
    // future `?` propagation introduced between push and pop.
    let _gc_scope_guard = unsafe { ctx.gc_root_scope() };

    // Allocate parameters
    for (i, param) in params.iter().enumerate() {
        let param_name = match &param.pattern {
            Pattern::Identifier(n) => n.clone(),
            _ => format!("param_{}", i),
        };

        let param_ty = param_types.get(i).cloned().unwrap_or(Type::Dynamic);
        let llvm_ty = ruyi_type_to_llvm(ctx.context, &param_ty);
        let ptr = ctx.builder().build_alloca(llvm_ty, &param_name);

        let param_value = function
            .get_nth_param(i as u32)
            .ok_or_else(|| format!("Missing parameter {}", i))?;
        ctx.builder().build_store(ptr, param_value);

        if is_gc_managed(&param_ty) {
            ctx.add_gc_root(ptr, param_ty.clone());
        }

        ctx.define_variable(param_name, (ptr, param_ty));
    }

    // Compile function body
    let result = compile_block(ctx, body);

    // Ensure ALL basic blocks in the function have terminators.
    // When compilation fails mid-way (e.g., unsupported codegen features),
    // some intermediate basic blocks may lack terminators.
    for bb in function.get_basic_blocks() {
        if bb.get_terminator().is_none() {
            ctx.builder().position_at_end(bb);
            if ret_type == Type::Void {
                ctx.builder().build_return(None);
            } else {
                let default_val = match ret_type {
                    Type::Int => {
                        BasicValueEnum::IntValue(ctx.context.i64_type().const_int(0, true))
                    }
                    Type::Float => {
                        BasicValueEnum::FloatValue(ctx.context.f64_type().const_float(0.0))
                    }
                    Type::Bool => {
                        BasicValueEnum::IntValue(ctx.context.bool_type().const_int(0, false))
                    }
                    Type::Byte => {
                        BasicValueEnum::IntValue(ctx.context.i8_type().const_int(0, false))
                    }
                    _ => BasicValueEnum::PointerValue(
                        ctx.context
                            .i8_type()
                            .ptr_type(Default::default())
                            .const_null(),
                    ),
                };
                ctx.builder().build_return(Some(&default_val));
            }
        }
    }

    // Restore previous state
    ctx.set_current_function(prev_function);
    ctx.set_current_return_type(prev_return_type);
    if let Some(block) = prev_block {
        ctx.builder().position_at_end(block);
    }

    // Restore the pre-function variable map (see snapshot above).
    ctx.variables = saved_vars;

    result
}

fn build_combined_fields<'ctx>(
    ctx: &CodegenContext<'ctx, '_, '_>,
    class_name: &str,
    own_fields: &[(String, Type)],
) -> Vec<(String, Type)> {
    let mut combined: Vec<(String, Type)> = Vec::new();

    // The hidden __typeid field is always at index 0 (added by compile_class).
    // When inheriting parent fields, skip the parent's __typeid to avoid
    // duplication — the child's own __typeid takes its place.
    if let Some(parent_name) = ctx.class_extends.get(class_name) {
        if let Some(parent_fields) = ctx.class_fields.get(parent_name) {
            combined.extend(
                parent_fields
                    .iter()
                    .filter(|(n, _)| n != "__typeid")
                    .cloned(),
            );
        }
    }

    combined.extend(own_fields.iter().cloned());
    combined
}

fn compile_class<'ctx>(
    ctx: &mut CodegenContext<'ctx, '_, '_>,
    name: &str,
    type_params: &[crate::parser::ast::TypeParam],
    extends: Option<&Expr>,
    body: &[ClassElement],
) -> Result<(), String> {
    let is_generic = !type_params.is_empty();
    if is_generic {
        // Register the template so call sites can instantiate specialized
        // method copies on demand (see codegen::specialize).
        ctx.generic_classes.insert(
            name.to_string(),
            (
                type_params.iter().map(|tp| tp.name.clone()).collect(),
                body.to_vec(),
            ),
        );
    }

    let mut fields: Vec<(String, Type)> = Vec::new();
    let mut methods: Vec<&ClassElement> = Vec::new();
    let mut static_methods: Vec<&ClassElement> = Vec::new();

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
            ClassElement::Method {
                is_static: true, ..
            } => {
                static_methods.push(element);
            }
            _ => {}
        }
    }

    if let Some(extends_expr) = extends {
        if let crate::parser::ast::Expr::Identifier(parent_name) = extends_expr {
            ctx.class_extends
                .insert(name.to_string(), parent_name.clone());
        }
    }

    // Prepend the hidden __typeid field (always at struct index 0) for
    // runtime `instanceof` support. build_combined_fields skips the
    // parent's __typeid so each class has exactly one.
    fields.insert(0, ("__typeid".to_string(), Type::Int));

    let combined_fields = build_combined_fields(ctx, name, &fields);
    ctx.class_fields
        .insert(name.to_string(), combined_fields.clone());

    let field_types: Vec<_> = combined_fields
        .iter()
        .map(|(_, ty)| super::types::ruyi_type_to_llvm(ctx.context, ty))
        .collect();
    let struct_type = ctx.context.struct_type(&field_types, false);
    ctx.class_struct_types.insert(name.to_string(), struct_type);

    // Compile static fields as module-level globals (e.g. `Signal.TERM`).
    // Each static field becomes a global variable named `{Class}_{field}`.
    for element in body {
        if let ClassElement::Field {
            name: prop_name,
            ty,
            init,
            is_static: true,
        } = element
        {
            let field_name = match prop_name {
                PropertyName::Ident(n) => n.clone(),
                _ => continue,
            };
            let field_ty = ty
                .as_ref()
                .map(Type::from_annotation)
                .unwrap_or(Type::Dynamic);
            let global_name = format!("{}_{}", name, field_name);
            let llvm_ty = ruyi_type_to_llvm(ctx.context, &field_ty);
            let global = ctx.module.add_global(llvm_ty, None, &global_name);
            global.set_linkage(inkwell::module::Linkage::Internal);
            let zero = ruyi_type_to_zero(ctx.context, &field_ty);
            global.set_initializer(&zero);
            // Evaluate the initializer (if any) and store into the global.
            if let Some(init_expr) = init {
                if let Ok(init_result) = compile_expr(ctx, init_expr) {
                    ctx.builder()
                        .build_store(global.as_pointer_value(), init_result.value);
                }
            }
            ctx.static_fields.insert(global_name, field_ty);
        }
    }

    // First pass: predeclare all methods to allow forward references
    for element in &methods {
        if let ClassElement::Method {
            name: prop_name,
            params,
            return_type,
            is_async,
            is_setter,
            ..
        } = element
        {
            let method_name = match prop_name {
                PropertyName::Ident(n) => {
                    if *is_setter {
                        format!("{}_set_{}", name, n)
                    } else {
                        format!("{}_{}", name, n)
                    }
                }
                _ => continue,
            };
            let mut method_params = vec![crate::parser::ast::Param {
                pattern: Pattern::Identifier("self".to_string()),
                ty: Some(crate::parser::ast::TypeAnnotation::Identifier(
                    name.to_string(),
                )),
                init: None,
                is_rest: false,
                is_optional: false,
            }];
            method_params.extend(
                params
                    .iter()
                    .filter(|p| !matches!(&p.pattern, Pattern::Identifier(n) if n == "self"))
                    .cloned(),
            );
            if !*is_async {
                predeclare_function(ctx, &method_name, &method_params, return_type.as_ref());
            }
        }
    }

    // Predeclare static methods as well so instance method bodies can
    // forward-reference them (e.g. an instance method calling
    // `Date.fromParts` before the static method is compiled).
    for element in &static_methods {
        if let ClassElement::Method {
            name: prop_name,
            params,
            return_type,
            is_async,
            ..
        } = element
        {
            let method_name = match prop_name {
                PropertyName::Ident(n) => format!("{}_{}", name, n),
                _ => continue,
            };
            let method_params = params
                .iter()
                .filter(|p| !matches!(&p.pattern, Pattern::Identifier(n) if n == "self"))
                .cloned()
                .collect::<Vec<_>>();
            if !*is_async {
                predeclare_function(ctx, &method_name, &method_params, return_type.as_ref());
            }
        }
    }

    for element in methods {
        if let ClassElement::Method {
            name: prop_name,
            params,
            return_type,
            body: method_body,
            is_async,
            is_setter,
            ..
        } = element
        {
            let method_name = match prop_name {
                PropertyName::Ident(n) => {
                    if *is_setter {
                        format!("{}_set_{}", name, n)
                    } else {
                        format!("{}_{}", name, n)
                    }
                }
                _ => continue,
            };

            let mut method_params = vec![crate::parser::ast::Param {
                pattern: Pattern::Identifier("self".to_string()),
                ty: Some(crate::parser::ast::TypeAnnotation::Identifier(
                    name.to_string(),
                )),
                init: None,
                is_rest: false,
                is_optional: false,
            }];
            method_params.extend(
                params
                    .iter()
                    .filter(|p| !matches!(&p.pattern, Pattern::Identifier(n) if n == "self"))
                    .cloned(),
            );

            if *is_async {
                super::async_codegen::compile_async_function(
                    ctx,
                    &method_name,
                    &method_params,
                    return_type.as_ref(),
                    method_body,
                )?;
            } else if let Err(e) = compile_function(
                ctx,
                &method_name,
                &method_params,
                return_type.as_ref(),
                None,
                None,
                method_body,
            ) {
                // Never leave a half-compiled body behind: calls would
                // silently produce garbage. Reset to a declaration so
                // uses fail loudly at link time instead.
                reset_to_declaration(ctx, &method_name);
                if is_generic {
                    // Generic method bodies are templates; the erased
                    // compilation may legitimately fail (e.g. trait
                    // method calls on a type-parameter receiver).
                    // Specialized copies are instantiated at call sites.
                    log::warn!(
                        "Deferring generic method {} to call-site specialization: {}",
                        method_name,
                        e
                    );
                } else {
                    return Err(e);
                }
            }
        }
    }

    for element in static_methods {
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

            let method_params = params
                .iter()
                .filter(|p| !matches!(&p.pattern, Pattern::Identifier(n) if n == "self"))
                .cloned()
                .collect::<Vec<_>>();

            if *is_async {
                super::async_codegen::compile_async_function(
                    ctx,
                    &method_name,
                    &method_params,
                    return_type.as_ref(),
                    method_body,
                )?;
            } else if let Err(e) = compile_function(
                ctx,
                &method_name,
                &method_params,
                return_type.as_ref(),
                None,
                None,
                method_body,
            ) {
                reset_to_declaration(ctx, &method_name);
                if is_generic {
                    log::warn!(
                        "Deferring generic static method {} to call-site specialization: {}",
                        method_name,
                        e
                    );
                } else {
                    return Err(e);
                }
            }
        }
    }

    Ok(())
}

/// Strip a function back to a bare declaration, deleting any (possibly
/// half-compiled) body. Existing references are rewired to the fresh
/// declaration so LLVM's use lists stay consistent.
pub fn reset_to_declaration<'ctx>(ctx: &mut CodegenContext<'ctx, '_, '_>, name: &str) {
    if let Some(func) = ctx.module.get_function(name) {
        if func.count_basic_blocks() == 0 {
            return;
        }
        let fn_type = func.get_type();
        let replacement = ctx
            .module
            .add_function(&format!("{}.__reset", name), fn_type, None);
        func.replace_all_uses_with(replacement);
        unsafe {
            func.delete();
        }
        replacement.as_global_value().set_name(name);
    }
}

fn compile_impl<'ctx>(
    ctx: &mut CodegenContext<'ctx, '_, '_>,
    type_params: &[crate::parser::ast::TypeParam],
    trait_name: &str,
    for_type: &crate::parser::ast::TypeAnnotation,
    body: &[crate::parser::ast::ClassElement],
) -> Result<(), String> {
    let for_type_str = match for_type {
        crate::parser::ast::TypeAnnotation::Identifier(name) => name.clone(),
        crate::parser::ast::TypeAnnotation::Builtin(name) => name.clone(),
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

            // Record the impl signature so call sites can substitute type
            // parameters into the declared return type (e.g. `iter` on
            // Array<int> returning ArrayIterator<int> instead of <T>).
            ctx.impl_method_sigs.insert(
                mangled_name.clone(),
                (
                    type_params.iter().map(|tp| tp.name.clone()).collect(),
                    for_type.clone(),
                ),
            );

            let impl_params: Vec<_> = std::iter::once(crate::parser::ast::Param {
                pattern: Pattern::Identifier("self".to_string()),
                ty: Some(for_type.clone()),
                init: None,
                is_rest: false,
                is_optional: false,
            })
            .chain(
                params
                    .iter()
                    .filter(|p| !matches!(&p.pattern, Pattern::Identifier(n) if n == "self"))
                    .cloned(),
            )
            .collect();

            if *is_async {
                if let Err(e) = super::async_codegen::compile_async_function(
                    ctx,
                    &mangled_name,
                    &impl_params,
                    return_type.as_ref(),
                    method_body,
                ) {
                    if ctx.allow_partial_codegen() {
                        log::warn!(
                            "Skipping impl async method codegen for {}: {}",
                            mangled_name,
                            e
                        );
                    } else {
                        return Err(e);
                    }
                }
            } else if let Err(e) = compile_function(
                ctx,
                &mangled_name,
                &impl_params,
                return_type.as_ref(),
                None,
                None,
                method_body,
            ) {
                reset_to_declaration(ctx, &mangled_name);
                if ctx.allow_partial_codegen() {
                    log::warn!("Skipping method codegen for {}: {}", method_name, e);
                } else {
                    return Err(e);
                }
            }
        }
    }
    Ok(())
}
