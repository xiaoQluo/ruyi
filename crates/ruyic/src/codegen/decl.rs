/**
 * Declaration code generation for Ruyi.
 *
 * Lowers Ruyi AST declarations (functions, variables, classes) to LLVM IR.
 *
 * @author Ruyi Team
 * @date 2026-05-01
 */
use inkwell::values::BasicValueEnum;
use inkwell::IntPredicate;

use super::builtins::is_gc_managed;
use super::expr::compile_expr;
use super::generator::{ruyi_type_to_zero, CodegenContext};
use super::stmt::{bind_pattern_in_codegen, compile_block};
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
            let (inferred_param_types, inferred_return_type): (Option<&[Type]>, Option<Type>) =
                match ctx.type_environment.and_then(|env| env.lookup(name)) {
                    Some(Type::Function {
                        params: fn_params,
                        return_type: fn_ret,
                    }) => (Some(fn_params.as_slice()), Some((**fn_ret).clone())),
                    _ => (None, None),
                };

            if *is_async {
                super::async_codegen::compile_async_function(
                    ctx,
                    name,
                    params,
                    return_type.as_ref(),
                    body,
                )
            } else {
                compile_function(
                    ctx,
                    name,
                    params,
                    return_type.as_ref(),
                    inferred_param_types,
                    inferred_return_type.as_ref(),
                    body,
                    None,
                )
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
    if ctx.closure_shared_names.contains(name) {
        return compile_shared_binding(ctx, name, binding);
    }
    let (ty, _llvm_ty, ptr) = if let Some(annotation) = &binding.ty {
        let ty = Type::from_annotation(annotation);
        let llvm_ty = ruyi_type_to_llvm(ctx.context, &ty);
        let ptr = ctx.builder().build_alloca(llvm_ty, name).unwrap();
        (ty, llvm_ty, ptr)
    } else if let Some(init) = &binding.init {
        let init_result = compile_expr(ctx, init)?;
        let ty = init_result.ty;
        let llvm_ty = ruyi_type_to_llvm(ctx.context, &ty);
        let ptr = ctx.builder().build_alloca(llvm_ty, name).unwrap();
        // Box the value when the LLVM alloca type differs from the actual
        // value type (e.g., Dynamic struct vs i64 from generic erasure).
        let actual_ty = init_result.value.get_type();
        let store_val = if llvm_ty != actual_ty {
            if llvm_ty.is_struct_type() && actual_ty.is_int_type() {
                super::expr::build_box_dynamic(ctx, init_result.value, &ty)
            } else if llvm_ty.is_struct_type() && actual_ty.is_pointer_type() {
                super::expr::build_box_dynamic(ctx, init_result.value, &ty)
            } else if llvm_ty.is_int_type() && actual_ty.is_struct_type() {
                let sv = init_result.value.into_struct_value();
                let data_ptr = ctx
                    .builder()
                    .build_extract_value(sv, 1, "unbox_data")
                    .unwrap()
                    .into_pointer_value();
                BasicValueEnum::IntValue(
                    ctx.builder()
                        .build_ptr_to_int(data_ptr, llvm_ty.into_int_type(), "unbox_s2i")
                        .unwrap(),
                )
            } else {
                init_result.value
            }
        } else {
            init_result.value
        };
        ctx.builder().build_store(ptr, store_val).unwrap();
        ctx.define_variable(name.to_string(), (ptr, ty.clone()));
        if is_gc_managed(&ty) {
            ctx.add_gc_root(ptr, ty);
        }
        return Ok(());
    } else {
        let ty = Type::Dynamic;
        let llvm_ty = ruyi_type_to_llvm(ctx.context, &ty);
        let ptr = ctx.builder().build_alloca(llvm_ty, name).unwrap();
        (ty, llvm_ty, ptr)
    };

    if let Some(init) = &binding.init {
        let prev_expected = ctx.expected_expr_type().cloned();
        ctx.set_expected_expr_type(Some(ty.clone()));
        let init_result = super::expr::compile_expr(ctx, init)?;
        ctx.set_expected_expr_type(prev_expected);

        // Trait object coercion: when the target type is `dyn Trait`,
        // wrap the concrete value into a { data, vtable } fat pointer.
        if let Type::Trait(trait_name) = &ty {
            let trait_obj_val = super::traits::build_trait_object_value(
                ctx,
                init_result.value,
                &init_result.ty,
                trait_name,
            )?;
            ctx.builder().build_store(ptr, trait_obj_val).unwrap();
        } else if ty == Type::Dynamic && init_result.ty != Type::Dynamic {
            // Dynamic boxing: construct {i64, i8*} struct
            let dyn_val = super::expr::build_box_dynamic(ctx, init_result.value, &init_result.ty);
            ctx.builder().build_store(ptr, dyn_val).unwrap();
        } else {
            ctx.builder().build_store(ptr, init_result.value).unwrap();
        }
    }

    if is_gc_managed(&ty) {
        ctx.add_gc_root(ptr, ty.clone());
    }

    ctx.define_variable(name.to_string(), (ptr, ty));
    Ok(())
}

/// Compile a `let` binding that some closure in the module assigns to.
/// The value lives in a GC-heap cell; the local slot holds the cell
/// pointer and `lookup_variable` resolves reads/writes to the cell, so
/// the enclosing scope and every closure capturing the name share one
/// location. Cell size follows `capture_slot_size` so closure
/// environment slots can carry the cell pointer uniformly.
fn compile_shared_binding<'ctx>(
    ctx: &mut CodegenContext<'ctx, '_, '_>,
    name: &str,
    binding: &Binding,
) -> Result<(), String> {
    let declared_ty = binding.ty.as_ref().map(Type::from_annotation);

    let init_result = match &binding.init {
        Some(init) => {
            let prev_expected = ctx.expected_expr_type().cloned();
            ctx.set_expected_expr_type(declared_ty.clone());
            let result = compile_expr(ctx, init)?;
            ctx.set_expected_expr_type(prev_expected);
            Some(result)
        }
        None => None,
    };

    let ty = declared_ty
        .or_else(|| init_result.as_ref().map(|r| r.ty.clone()))
        .unwrap_or(Type::Dynamic);

    let ptr_ty = ctx.context.ptr_type(Default::default());
    let slot = ctx.builder().build_alloca(ptr_ty, name).unwrap();

    let slot_size = super::capture::capture_slot_size(&ty);
    let cell = super::gc_alloc::GcAllocFn::for_mode(ctx.gc_mode).emit(
        ctx.builder(),
        ctx.module,
        ctx.context.i64_type().const_int(slot_size, false),
    );
    // Zero the cell so a read before the first assignment never sees
    // malloc garbage (a garbage Dynamic tag would crash the dispatcher).
    let zero = ctx.context.i64_type().const_zero();
    for w in 0..(slot_size / 8) {
        let off = ctx.context.i32_type().const_int(w * 8, false);
        let word_ptr = unsafe {
            ctx.builder()
                .build_gep(ctx.context.i8_type(), cell, &[off], "cell_zero")
                .unwrap()
        };
        ctx.builder().build_store(word_ptr, zero).unwrap();
    }

    if let Some(result) = init_result {
        let stored = match &ty {
            Type::Trait(trait_name) => {
                super::traits::build_trait_object_value(ctx, result.value, &result.ty, trait_name)?
                    .into()
            }
            _ => adjust_init_for_slot(ctx, result.value, &ty),
        };
        ctx.builder().build_store(cell, stored).unwrap();
    }

    ctx.builder().build_store(slot, cell).unwrap();
    ctx.add_gc_cell_root(slot);
    ctx.define_variable(name.to_string(), (slot, ty));
    Ok(())
}

/// Match the initializer's runtime value to the declared slot type,
/// mirroring the adjust logic in `compile_simple_binding`.
fn adjust_init_for_slot<'ctx>(
    ctx: &mut CodegenContext<'ctx, '_, '_>,
    value: BasicValueEnum<'ctx>,
    ty: &Type,
) -> BasicValueEnum<'ctx> {
    let llvm_ty = ruyi_type_to_llvm(ctx.context, ty);
    let actual_ty = value.get_type();
    if llvm_ty == actual_ty {
        return value;
    }
    if llvm_ty.is_struct_type() && (actual_ty.is_int_type() || actual_ty.is_pointer_type()) {
        super::expr::build_box_dynamic(ctx, value, ty)
    } else if llvm_ty.is_int_type() && actual_ty.is_struct_type() {
        let sv = value.into_struct_value();
        let data_ptr = ctx
            .builder()
            .build_extract_value(sv, 1, "unbox_data")
            .unwrap()
            .into_pointer_value();
        BasicValueEnum::IntValue(
            ctx.builder()
                .build_ptr_to_int(data_ptr, llvm_ty.into_int_type(), "unbox_s2i")
                .unwrap(),
        )
    } else {
        value
    }
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
    let arr_ty = result.ty.clone();
    let arr_ptr = match result.value {
        BasicValueEnum::PointerValue(p) => p,
        _ => return Err("Array destructuring requires an array value".to_string()),
    };

    let elem_ty = match &arr_ty {
        Type::Array(inner) => *inner.clone(),
        _ => Type::Dynamic,
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
            crate::parser::ast::ArrayPatternElement::Pattern(pat) => {
                let idx_val = ctx.context.i64_type().const_int(i, false);
                let elem_val = ctx
                    .builder()
                    .build_call(get_fn, &[arr_ptr.into(), idx_val.into()], "destr_elem")
                    .unwrap()
                    .try_as_basic_value()
                    .unwrap_basic();
                let elem_result = super::expr::ExprResult {
                    value: elem_val,
                    ty: elem_ty.clone(),
                };
                bind_pattern_in_codegen(ctx, pat, &elem_result)?;
                i += 1;
            }
            crate::parser::ast::ArrayPatternElement::Default(pat, _default_expr) => {
                let idx_val = ctx.context.i64_type().const_int(i, false);
                let elem_val = ctx
                    .builder()
                    .build_call(get_fn, &[arr_ptr.into(), idx_val.into()], "destr_elem")
                    .unwrap()
                    .try_as_basic_value()
                    .unwrap_basic();
                let elem_result = super::expr::ExprResult {
                    value: elem_val,
                    ty: elem_ty.clone(),
                };
                bind_pattern_in_codegen(ctx, pat, &elem_result)?;
                i += 1;
            }
            crate::parser::ast::ArrayPatternElement::Rest(pat) => {
                let rest_name = match pat {
                    Pattern::Identifier(n) => n.clone(),
                    _ => return Err("Rest pattern must be an identifier".to_string()),
                };

                let len_fn = ctx
                    .module
                    .get_function("__builtin_array_length")
                    .ok_or("__builtin_array_length not declared")?;
                let create_fn = ctx
                    .module
                    .get_function("__builtin_array_create")
                    .ok_or("__builtin_array_create not declared")?;
                let push_fn = ctx
                    .module
                    .get_function("__builtin_array_push")
                    .ok_or("__builtin_array_push not declared")?;

                let i64_ty = ctx.context.i64_type();
                let i8_ptr_ty = ctx.context.ptr_type(Default::default());

                // 获取数组长度
                let arr_len = ctx
                    .builder()
                    .build_call(len_fn, &[arr_ptr.into()], "arr_len")
                    .unwrap()
                    .try_as_basic_value()
                    .unwrap_basic()
                    .into_int_value();

                // 创建新的空数组
                let rest_arr = ctx
                    .builder()
                    .build_call(create_fn, &[], "rest_arr")
                    .unwrap()
                    .try_as_basic_value()
                    .unwrap_basic();

                let func = ctx.current_function().ok_or("No current function")?;
                let cond_bb = ctx.context.append_basic_block(func, "rest_cond");
                let body_bb = ctx.context.append_basic_block(func, "rest_body");
                let end_bb = ctx.context.append_basic_block(func, "rest_end");

                // 分配循环计数器并初始化
                let idx_ptr = ctx.builder().build_alloca(i64_ty, "rest_idx_ptr").unwrap();
                ctx.builder()
                    .build_store(idx_ptr, i64_ty.const_int(i, false))
                    .unwrap();
                // 分配 rest 数组指针（可变）
                let rest_arr_ptr = ctx
                    .builder()
                    .build_alloca(i8_ptr_ty, "rest_arr_ptr")
                    .unwrap();
                ctx.builder().build_store(rest_arr_ptr, rest_arr).unwrap();

                ctx.builder().build_unconditional_branch(cond_bb).unwrap();

                // 条件块：idx < len
                ctx.builder().position_at_end(cond_bb);
                let cur_idx = ctx
                    .builder()
                    .build_load(i64_ty, idx_ptr, "cur_idx")
                    .unwrap()
                    .into_int_value();
                let cond = ctx
                    .builder()
                    .build_int_compare(IntPredicate::SLT, cur_idx, arr_len, "rest_cond")
                    .unwrap();
                ctx.builder()
                    .build_conditional_branch(cond, body_bb, end_bb)
                    .unwrap();

                // 循环体：获取元素并 push
                ctx.builder().position_at_end(body_bb);
                let loop_idx = ctx
                    .builder()
                    .build_load(i64_ty, idx_ptr, "loop_idx")
                    .unwrap()
                    .into_int_value();
                let elem = ctx
                    .builder()
                    .build_call(get_fn, &[arr_ptr.into(), loop_idx.into()], "rest_elem")
                    .unwrap()
                    .try_as_basic_value()
                    .unwrap_basic();
                let cur_rest = ctx
                    .builder()
                    .build_load(i8_ptr_ty, rest_arr_ptr, "cur_rest")
                    .unwrap();
                let new_rest = ctx
                    .builder()
                    .build_call(push_fn, &[cur_rest.into(), elem.into()], "rest_push")
                    .unwrap()
                    .try_as_basic_value()
                    .unwrap_basic();
                ctx.builder().build_store(rest_arr_ptr, new_rest).unwrap();
                let next_idx = ctx
                    .builder()
                    .build_int_add(loop_idx, i64_ty.const_int(1, false), "next_idx")
                    .unwrap();
                ctx.builder().build_store(idx_ptr, next_idx).unwrap();
                ctx.builder().build_unconditional_branch(cond_bb).unwrap();

                // 结束块：绑定 rest 变量
                ctx.builder().position_at_end(end_bb);
                let final_rest = ctx
                    .builder()
                    .build_load(i8_ptr_ty, rest_arr_ptr, "final_rest")
                    .unwrap();
                let rest_ty = Type::Array(Box::new(elem_ty.clone()));
                let llvm_rest_ty = ruyi_type_to_llvm(ctx.context, &rest_ty);
                let var_ptr = ctx
                    .builder()
                    .build_alloca(llvm_rest_ty, &rest_name)
                    .unwrap();
                ctx.builder().build_store(var_ptr, final_rest).unwrap();
                ctx.define_variable(rest_name, (var_ptr, rest_ty));

                // Rest 必须是最后一个元素
                break;
            }
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
        Pattern::Object(_) => {
            let init = binding
                .init
                .as_ref()
                .ok_or("Object destructuring requires an initializer")?;
            let result = compile_expr(ctx, init)?;
            bind_pattern_in_codegen(ctx, &binding.pattern, &result)?;
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
            let mut ty =
                p.ty.as_ref()
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

    // Prefer inferred return type over explicit annotation (annotation may be absent).
    let ret_type = return_type
        .map(Type::from_annotation)
        .or_else(|| {
            ctx.type_environment
                .and_then(|env| env.lookup(name))
                .and_then(|ty| match ty {
                    Type::Function { return_type, .. } => Some((**return_type).clone()),
                    _ => None,
                })
        })
        .unwrap_or(Type::Void);

    let fn_type = function_type_from_ruyi(ctx.context, &param_types, &ret_type);

    let ruyi_fn_type = Type::Function {
        params: param_types,
        return_type: Box::new(ret_type),
    };
    ctx.function_types.insert(name.to_string(), ruyi_fn_type);

    let function = ctx.module.add_function(name, fn_type, None);
    // Keep user-defined symbols out of the global namespace: a Ruyi
    // function named like a libc/libm symbol (e.g. `pow`, `sqrt` in
    // stdlib/math.ry) would otherwise capture the runtime's reference
    // to the real libc function at link time (Rust's `f64::powf`
    // lowers to a `pow` call), producing infinite recursion. Internal
    // linkage keeps all calls module-local; `main` stays external as
    // the process entry point.
    if name != "main" {
        function.set_linkage(inkwell::module::Linkage::Internal);
    }
}

#[allow(clippy::too_many_arguments)]
pub fn compile_function<'ctx>(
    ctx: &mut CodegenContext<'ctx, '_, '_>,
    name: &str,
    params: &[crate::parser::ast::Param],
    return_type: Option<&crate::parser::ast::TypeAnnotation>,
    inferred_param_types: Option<&[Type]>,
    inferred_return_type: Option<&Type>,
    body: &[crate::parser::ast::Statement],
    env_captures: Option<&[(String, Type)]>,
) -> Result<(), String> {
    let param_types: Vec<Type> = inferred_param_types
        .map(|types| types.to_vec())
        .unwrap_or_else(|| {
            params
                .iter()
                .map(|p| {
                    let mut ty =
                        p.ty.as_ref()
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

    let fn_type = if env_captures.is_some() {
        super::types::function_type_with_trailing_env(ctx.context, &param_types, &ret_type)
    } else {
        function_type_from_ruyi(ctx.context, &param_types, &ret_type)
    };

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
                // `...paths: string` means each argument is a string; the
                // signature builder wraps it as Array<string>. Falling
                // back to Dynamic here boxed every argument, so consumers
                // reading the array as raw pointers (runtime FFI such as
                // __path_join) got the box tag instead.
                Some(ann) => Type::from_annotation(ann),
                None => Type::Dynamic,
            };
            ctx.rest_params.insert(name.to_string(), (i, elem_ty));
            break;
        }
    }

    // Record default-parameter expressions so call sites that omit
    // trailing arguments can synthesize them (LLVM has no default
    // arguments; passing too few arguments reads undefined registers).
    if params.iter().any(|p| p.init.is_some()) {
        let defaults: Vec<Option<crate::parser::ast::Expr>> =
            params.iter().map(|p| p.init.as_deref().cloned()).collect();
        ctx.default_params.insert(name.to_string(), defaults);
    }

    let function = if let Some(existing) = ctx.module.get_function(name) {
        existing
    } else {
        ctx.module.add_function(name, fn_type, None)
    };
    // See predeclare_function: internal linkage prevents user functions
    // from shadowing libc/libm symbols at link time.
    if name != "main" {
        function.set_linkage(inkwell::module::Linkage::Internal);
    }

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
    // Frame isolation: drop every frame-backed binding so a nested
    // function (closure/arrow/method) cannot reference an enclosing
    // frame's alloca — that cross-function reference is invalid IR and
    // silently reads garbage at runtime. Outer values reach nested
    // bodies only through explicit captures.
    ctx.variables.clear();

    // ── Isolate try-frame / try / loop stacks across function boundaries ──
    // A nested function (closure, arrow, or class method) compiled while an
    // outer try-block is active must NOT inherit the outer landing-pad basic
    // blocks — those belong to a different LLVM function, and referencing
    // them from `invoke` instructions would produce invalid IR
    // ("Referring to a basic block in another function" → SIGSEGV 139).
    let saved_try_frame_stack = std::mem::take(&mut ctx.try_frame_stack);
    let saved_try_stack = std::mem::take(&mut ctx.try_stack);
    let saved_loop_stack = std::mem::take(&mut ctx.loop_stack);
    let saved_pending_return_flag = ctx.pending_return_flag.take();
    let saved_pending_return_value = ctx.pending_return_value.take();
    let saved_pending_break_target = ctx.pending_break_target.take();
    let saved_pending_continue_target = ctx.pending_continue_target.take();

    // SAFETY: pop_gc_root_scope guaranteed by GcRootGuard Drop.
    // The previous bare `?` on `get_nth_param` could leak the scope on
    // the defensive error path; the RAII guard closes that gap and any
    // future `?` propagation introduced between push and pop.
    let _gc_scope_guard = unsafe { ctx.gc_root_scope() };

    // Materialize the capture environment: the trailing env parameter
    // points to a block laid out like the creation-site struct, one slot
    // per capture in order. Each capture becomes a named local slot so
    // the body compiles against it unchanged.
    if let Some(captures) = env_captures {
        let param_count = function.count_params();
        let env_param = function
            .get_nth_param(param_count - 1)
            .ok_or("Closure environment parameter missing")?
            .into_pointer_value();
        let mut offset: u64 = 0;
        for (cap_name, cap_ty) in captures {
            let llvm_ty = ruyi_type_to_llvm(ctx.context, cap_ty);
            let off = ctx.context.i32_type().const_int(offset, false);
            let slot_ptr = unsafe {
                ctx.builder()
                    .build_gep(ctx.context.i8_type(), env_param, &[off], "cap_slot")
                    .unwrap()
            };
            if ctx.closure_shared_names.contains(cap_name) {
                // Shared mutable capture: the env slot holds the GC cell
                // pointer; keep the local slot indirect so reads and
                // writes hit the cell shared with the enclosing scope.
                let ptr_ty = ctx.context.ptr_type(Default::default());
                let cell = ctx
                    .builder()
                    .build_load(ptr_ty, slot_ptr, cap_name)
                    .unwrap()
                    .into_pointer_value();
                let slot = ctx.builder().build_alloca(ptr_ty, cap_name).unwrap();
                ctx.builder().build_store(slot, cell).unwrap();
                ctx.add_gc_cell_root(slot);
                ctx.define_variable(cap_name.clone(), (slot, cap_ty.clone()));
            } else {
                let value = ctx
                    .builder()
                    .build_load(llvm_ty, slot_ptr, cap_name)
                    .unwrap();
                let slot = ctx.builder().build_alloca(llvm_ty, cap_name).unwrap();
                ctx.builder().build_store(slot, value).unwrap();
                if is_gc_managed(cap_ty) {
                    ctx.add_gc_root(slot, cap_ty.clone());
                }
                ctx.define_variable(cap_name.clone(), (slot, cap_ty.clone()));
            }
            offset += super::capture::capture_slot_size(cap_ty);
        }
    }

    // Allocate parameters
    for (i, param) in params.iter().enumerate() {
        let param_ty = param_types.get(i).cloned().unwrap_or(Type::Dynamic);
        let param_value = function
            .get_nth_param(i as u32)
            .ok_or_else(|| format!("Missing parameter {}", i))?;

        match &param.pattern {
            Pattern::Identifier(n) => {
                let llvm_ty = ruyi_type_to_llvm(ctx.context, &param_ty);
                let ptr = ctx.builder().build_alloca(llvm_ty, n).unwrap();
                ctx.builder().build_store(ptr, param_value).unwrap();
                if is_gc_managed(&param_ty) {
                    ctx.add_gc_root(ptr, param_ty.clone());
                }
                ctx.define_variable(n.clone(), (ptr, param_ty));
            }
            pattern => {
                // 对象/数组解构参数：先存入临时变量，再解构绑定
                let temp_name = format!("_param_{}", i);
                let llvm_ty = ruyi_type_to_llvm(ctx.context, &param_ty);
                let ptr = ctx.builder().build_alloca(llvm_ty, &temp_name).unwrap();
                ctx.builder().build_store(ptr, param_value).unwrap();
                if is_gc_managed(&param_ty) {
                    ctx.add_gc_root(ptr, param_ty.clone());
                }
                let param_result = super::expr::ExprResult {
                    value: ptr.into(),
                    ty: param_ty,
                };
                bind_pattern_in_codegen(ctx, pattern, &param_result)?;
            }
        }
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
                ctx.builder().build_return(None).unwrap();
            } else {
                // Generate a default value matching the LLVM function's actual
                // return type (handles generic type erasure scenarios).
                let llvm_ret_ty = function.get_type().get_return_type();
                let default_val = match llvm_ret_ty {
                    Some(t) if t.is_int_type() => {
                        BasicValueEnum::IntValue(t.into_int_type().const_int(0, false))
                    }
                    Some(t) if t.is_struct_type() => {
                        BasicValueEnum::StructValue(t.into_struct_type().const_zero())
                    }
                    Some(t) if t.is_float_type() => {
                        BasicValueEnum::FloatValue(t.into_float_type().const_float(0.0))
                    }
                    _ => {
                        // Fall back to Ruyi type-based matching
                        match ret_type {
                            Type::Int => {
                                BasicValueEnum::IntValue(ctx.context.i64_type().const_int(0, true))
                            }
                            Type::Float => {
                                BasicValueEnum::FloatValue(ctx.context.f64_type().const_float(0.0))
                            }
                            Type::Bool => BasicValueEnum::IntValue(
                                ctx.context.bool_type().const_int(0, false),
                            ),
                            Type::Byte => {
                                BasicValueEnum::IntValue(ctx.context.i8_type().const_int(0, false))
                            }
                            _ => BasicValueEnum::PointerValue(
                                ctx.context.ptr_type(Default::default()).const_null(),
                            ),
                        }
                    }
                };
                ctx.builder().build_return(Some(&default_val)).unwrap();
            }
        }
    }

    // Restore isolated try/loop/pending state
    ctx.try_frame_stack = saved_try_frame_stack;
    ctx.try_stack = saved_try_stack;
    ctx.loop_stack = saved_loop_stack;
    ctx.pending_return_flag = saved_pending_return_flag;
    ctx.pending_return_value = saved_pending_return_value;
    ctx.pending_break_target = saved_pending_break_target;
    ctx.pending_continue_target = saved_pending_continue_target;

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
    // The hidden __typeid must stay at struct index 0: `compile_new`
    // addresses it with GEP [0, 0] and `instanceof` loads byte offset 0.
    let mut combined: Vec<(String, Type)> = own_fields
        .iter()
        .filter(|(n, _)| n == "__typeid")
        .cloned()
        .collect();

    // Parent fields come next so that a base-class constructor keeps
    // writing the same slots on subclass instances. Putting own fields
    // first aliased them: `super.new()` stored the base field into the
    // first own-field slot (e.g. AppError.code), which the subclass
    // constructor then overwrote, leaving the base field null.
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

    // Own (non-typeid) fields last.
    combined.extend(own_fields.iter().filter(|(n, _)| n != "__typeid").cloned());

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
    // Register the template so call sites can instantiate specialized
    // method copies on demand (see codegen::specialize). Non-generic
    // classes register too, with an empty parameter list: a class can
    // carry generic *methods* without being generic itself (`None` in
    // stdlib/option.ry declares `map<T, U>`), and those methods still
    // need a body to specialize. Every consumer degrades to the
    // non-generic behaviour on an empty parameter list.
    ctx.generic_classes.insert(
        name.to_string(),
        (
            type_params.iter().map(|tp| tp.name.clone()).collect(),
            body.to_vec(),
        ),
    );

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

    let static_method_names: std::collections::HashSet<String> = static_methods
        .iter()
        .filter_map(|e| {
            if let ClassElement::Method {
                name: PropertyName::Ident(n),
                ..
            } = e
            {
                Some(n.clone())
            } else {
                None
            }
        })
        .collect();
    ctx.class_static_methods
        .insert(name.to_string(), static_method_names);

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
                        .build_store(global.as_pointer_value(), init_result.value)
                        .unwrap();
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

    // Set current class name for super.new() resolution
    ctx.current_class_name = Some(name.to_string());

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
                None,
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
                None,
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

    // Clear current class name after all methods are compiled
    ctx.current_class_name = None;

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
                None,
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
