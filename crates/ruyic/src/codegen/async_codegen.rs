use super::builtins::is_gc_managed;
use super::expr::compile_expr;
use super::generator::CodegenContext;
use super::stmt::compile_block;
use super::types::{function_type_from_ruyi, ruyi_type_to_llvm};
use crate::parser::ast::{Expr, Statement};
use crate::typechecker::types::Type;
use inkwell::types::BasicTypeEnum;
use inkwell::values::BasicValueEnum;

/// Count await expressions in a statement list.
fn count_awaits_in_statements(stmts: &[Statement]) -> usize {
    stmts.iter().map(count_awaits_in_stmt).sum()
}

fn count_awaits_in_stmt(stmt: &Statement) -> usize {
    match stmt {
        Statement::Expression(expr) | Statement::Return(Some(expr)) => count_awaits_in_expr(expr),
        Statement::Block(stmts) => count_awaits_in_statements(stmts),
        Statement::If {
            condition,
            then_branch,
            else_branch,
        } => {
            count_awaits_in_expr(condition)
                + count_awaits_in_stmt(then_branch)
                + else_branch
                    .as_ref()
                    .map(|e| count_awaits_in_stmt(e))
                    .unwrap_or(0)
        }
        Statement::While { condition, body } => {
            count_awaits_in_expr(condition) + count_awaits_in_stmt(body)
        }
        Statement::For {
            init,
            condition,
            update,
            body,
        } => {
            let init_count = match init {
                Some(crate::parser::ast::ForInit::Expr(e)) => count_awaits_in_expr(e),
                Some(crate::parser::ast::ForInit::VarDecl(decl)) => count_awaits_in_decl(decl),
                None => 0,
            };
            let cond_count = condition
                .as_ref()
                .map(|e| count_awaits_in_expr(e))
                .unwrap_or(0);
            let update_count = update
                .as_ref()
                .map(|e| count_awaits_in_expr(e))
                .unwrap_or(0);
            init_count + cond_count + update_count + count_awaits_in_stmt(body)
        }
        Statement::ForIn { iterable, body, .. } | Statement::ForOf { iterable, body, .. } => {
            count_awaits_in_expr(iterable) + count_awaits_in_stmt(body)
        }
        Statement::Try {
            body,
            catch,
            finally,
        } => {
            let body_count = count_awaits_in_statements(body);
            let catch_count: usize = catch
                .iter()
                .map(|c| count_awaits_in_statements(&c.body))
                .sum();
            let finally_count = finally
                .as_ref()
                .map(|f| count_awaits_in_statements(f))
                .unwrap_or(0);
            body_count + catch_count + finally_count
        }
        Statement::Match { value, arms } => {
            let val_count = count_awaits_in_expr(value);
            let arms_count: usize = arms
                .iter()
                .map(|arm| count_awaits_in_statements(&arm.body))
                .sum();
            val_count + arms_count
        }
        Statement::Declaration(decl) => count_awaits_in_decl(decl),
        Statement::Labeled { body, .. } => count_awaits_in_stmt(body),
        _ => 0,
    }
}

fn count_awaits_in_decl(decl: &crate::parser::ast::Declaration) -> usize {
    match decl {
        crate::parser::ast::Declaration::Let(bindings)
        | crate::parser::ast::Declaration::Const(bindings) => bindings
            .iter()
            .map(|b| {
                b.init
                    .as_ref()
                    .map(|e| count_awaits_in_expr(e))
                    .unwrap_or(0)
            })
            .sum(),
        crate::parser::ast::Declaration::Function { body, .. } => count_awaits_in_statements(body),
        _ => 0,
    }
}

fn count_awaits_in_expr(expr: &Expr) -> usize {
    match expr {
        Expr::Await(inner) => 1 + count_awaits_in_expr(inner),
        Expr::Binary { left, right, .. } => {
            count_awaits_in_expr(left) + count_awaits_in_expr(right)
        }
        Expr::Unary { operand, .. } => count_awaits_in_expr(operand),
        Expr::Call { callee, args } => {
            count_awaits_in_expr(callee)
                + args
                    .iter()
                    .map(|a| match a {
                        crate::parser::ast::Argument::Expr(e) => count_awaits_in_expr(e),
                        _ => 0,
                    })
                    .sum::<usize>()
        }
        Expr::Conditional {
            condition,
            then_branch,
            else_branch,
        } => {
            count_awaits_in_expr(condition)
                + count_awaits_in_expr(then_branch)
                + count_awaits_in_expr(else_branch)
        }
        Expr::Assignment { left, right, .. } => {
            count_awaits_in_expr(left) + count_awaits_in_expr(right)
        }
        Expr::Member { object, .. } => count_awaits_in_expr(object),
        Expr::OptionalCall { callee, args } => {
            count_awaits_in_expr(callee)
                + args
                    .iter()
                    .map(|a| match a {
                        crate::parser::ast::Argument::Expr(e) => count_awaits_in_expr(e),
                        _ => 0,
                    })
                    .sum::<usize>()
        }
        Expr::ArrayLiteral(elements) => elements
            .iter()
            .map(|e| match e {
                crate::parser::ast::ArrayElement::Expr(e) => count_awaits_in_expr(e),
                crate::parser::ast::ArrayElement::Spread(e) => count_awaits_in_expr(e),
                _ => 0,
            })
            .sum(),
        Expr::ObjectLiteral(props) => props
            .iter()
            .map(|p| match p {
                crate::parser::ast::ObjectProperty::Property { value, .. } => {
                    count_awaits_in_expr(value)
                }
                crate::parser::ast::ObjectProperty::Spread(e) => count_awaits_in_expr(e),
                _ => 0,
            })
            .sum(),
        Expr::New { callee, args } => {
            count_awaits_in_expr(callee)
                + args
                    .iter()
                    .map(|a| match a {
                        crate::parser::ast::Argument::Expr(e) => count_awaits_in_expr(e),
                        _ => 0,
                    })
                    .sum::<usize>()
        }
        Expr::Match { value, arms } => {
            let val_count = count_awaits_in_expr(value);
            let arms_count: usize = arms
                .iter()
                .map(|arm| count_awaits_in_statements(&arm.body))
                .sum();
            val_count + arms_count
        }
        Expr::If {
            condition,
            then_branch,
            else_branch,
        } => {
            count_awaits_in_expr(condition)
                + count_awaits_in_expr(then_branch)
                + else_branch
                    .as_ref()
                    .map(|e| count_awaits_in_expr(e))
                    .unwrap_or(0)
        }
        Expr::Grouping(inner) => count_awaits_in_expr(inner),
        Expr::Sequence(inner) => inner.iter().map(count_awaits_in_expr).sum(),
        Expr::Block(stmts) => count_awaits_in_statements(stmts),
        Expr::ArrowFunction { body, .. } => match body {
            crate::parser::ast::ArrowBody::Expr(e) => count_awaits_in_expr(e),
            crate::parser::ast::ArrowBody::Block(stmts) => count_awaits_in_statements(stmts),
        },
        Expr::Function { body, .. } => count_awaits_in_statements(body),
        _ => 0,
    }
}

/// Compile an async function declaration into a state machine.
///
/// Generates three LLVM entities:
/// 1. `{name}$new` – constructor that allocates and initializes the state struct.
/// 2. `{name}$poll` – poll function that drives the state machine.
/// 3. `{name}` – thin wrapper that calls `$new` and returns the future pointer.
pub fn compile_async_function<'ctx>(
    ctx: &mut CodegenContext<'ctx, '_, '_>,
    name: &str,
    params: &[crate::parser::ast::Param],
    return_type: Option<&crate::parser::ast::TypeAnnotation>,
    body: &[crate::parser::ast::Statement],
) -> Result<(), String> {
    let ret_type = return_type.map(Type::from_annotation).unwrap_or(Type::Void);
    let param_types: Vec<Type> = params
        .iter()
        .map(|p| {
            p.ty.as_ref()
                .map(Type::from_annotation)
                .unwrap_or(Type::Dynamic)
        })
        .collect();

    let await_count = count_awaits_in_statements(body);
    let _ = await_count;

    let i8_ptr = ctx.context.i8_type().ptr_type(Default::default());
    let i32_ty = ctx.context.i32_type();
    let fn_ptr_ty = i32_ty
        .fn_type(&[i8_ptr.into(), i8_ptr.into()], false)
        .ptr_type(Default::default());

    let mut state_field_types: Vec<BasicTypeEnum<'ctx>> = vec![fn_ptr_ty.into()];
    state_field_types.push(i32_ty.into());
    for pt in &param_types {
        state_field_types.push(ruyi_type_to_llvm(ctx.context, pt));
    }
    let result_llvm_type = ruyi_type_to_llvm(ctx.context, &ret_type);
    state_field_types.push(result_llvm_type);

    let state_struct_type = ctx.context.struct_type(&state_field_types, false);
    let state_ptr_type = state_struct_type.ptr_type(Default::default());

    let poll_fn_field_idx: u32 = 0;
    let state_field_idx: u32 = 1;
    let param_field_indices: Vec<u32> = (2..=param_types.len() as u32 + 1).collect();
    let result_field_idx = (param_types.len() + 2) as u32;

    // ── Save current codegen state ─────────────────────────────
    let prev_function = ctx.current_function();
    let prev_block = ctx.builder().get_insert_block();
    let mut prev_vars = std::collections::HashMap::new();

    let saved_async_state_field_ptr = ctx.async_state_field_ptr;
    let saved_async_result_ptr = ctx.async_result_ptr;
    let saved_async_return_bb = ctx.async_return_bb;
    let saved_waker_ptr = ctx.waker_ptr;

    // ── Isolate try-frame / try / loop stacks across function boundaries ──
    // Same rationale as compile_function: nested async functions must not
    // inherit outer landing-pad basic blocks (SIGSEGV 139).
    let saved_try_frame_stack = std::mem::take(&mut ctx.try_frame_stack);
    let saved_try_stack = std::mem::take(&mut ctx.try_stack);
    let saved_loop_stack = std::mem::take(&mut ctx.loop_stack);
    let saved_pending_return_flag = ctx.pending_return_flag.take();
    let saved_pending_return_value = ctx.pending_return_value.take();
    let saved_pending_break_target = ctx.pending_break_target.take();
    let saved_pending_continue_target = ctx.pending_continue_target.take();

    // ── 1. Declare Constructor: {name}$new ─────────────────────
    let new_param_types: Vec<_> = param_types
        .iter()
        .map(|pt| ruyi_type_to_llvm(ctx.context, pt).into())
        .collect();
    let new_fn_type = i8_ptr.fn_type(&new_param_types, false);
    let new_fn = ctx
        .module
        .add_function(&format!("{}$new", name), new_fn_type, None);

    // ── 2. Poll function: {name}$poll ──────────────────────────
    let poll_fn_type = i32_ty.fn_type(&[i8_ptr.into(), i8_ptr.into()], false);
    let poll_fn = ctx
        .module
        .add_function(&format!("{}$poll", name), poll_fn_type, None);

    let poll_entry = ctx.context.append_basic_block(poll_fn, "entry");
    let poll_start = ctx.context.append_basic_block(poll_fn, "start");
    let poll_return = ctx.context.append_basic_block(poll_fn, "async_return");
    let poll_done = ctx.context.append_basic_block(poll_fn, "done");

    ctx.builder().position_at_end(poll_entry);
    let raw_state_ptr = poll_fn.get_nth_param(0).unwrap().into_pointer_value();
    let waker_ptr = poll_fn.get_nth_param(1).unwrap().into_pointer_value();
    let state_ptr_val = ctx
        .builder()
        .build_bitcast(raw_state_ptr, state_ptr_type, "state")
        .into_pointer_value();

    let state_field_ptr_poll = unsafe {
        ctx.builder().build_gep(
            state_ptr_val,
            &[
                i32_ty.const_int(0, false),
                i32_ty.const_int(state_field_idx as u64, false),
            ],
            "state_field",
        )
    };
    let current_state = ctx
        .builder()
        .build_load(state_field_ptr_poll, "current_state")
        .into_int_value();

    ctx.builder().build_switch(
        current_state,
        poll_done,
        &[(i32_ty.const_int(0, false), poll_start)],
    );

    // Start block: load params from state into locals and compile body
    ctx.builder().position_at_end(poll_start);
    ctx.set_current_function(Some(poll_fn));
    // SAFETY: pop_gc_root_scope guaranteed by GcRootScopeGuard Drop.
    // The poll body has no `?` paths between push and pop today, but
    // the guard defends against future additions of fallible calls
    // (e.g. new intrinsics) in the per-param setup loop.
    let _gc_scope_guard = unsafe { ctx.gc_root_scope() };

    for (i, param) in params.iter().enumerate() {
        let param_name = match &param.pattern {
            crate::parser::ast::Pattern::Identifier(n) => n.clone(),
            _ => format!("param_{}", i),
        };
        let param_ty = param_types.get(i).cloned().unwrap_or(Type::Dynamic);
        let llvm_ty = ruyi_type_to_llvm(ctx.context, &param_ty);
        let local_ptr = ctx.builder().build_alloca(llvm_ty, &param_name);

        let field_ptr = unsafe {
            ctx.builder().build_gep(
                state_ptr_val,
                &[
                    i32_ty.const_int(0, false),
                    i32_ty.const_int(param_field_indices[i] as u64, false),
                ],
                &format!("param_{}_slot", i),
            )
        };
        let loaded = ctx
            .builder()
            .build_load(field_ptr, &format!("param_{}_val", i));
        ctx.builder().build_store(local_ptr, loaded);

        if is_gc_managed(&param_ty) {
            ctx.add_gc_root(local_ptr, param_ty.clone());
        }

        if let Some(old) = ctx
            .variables
            .insert(param_name.clone(), (local_ptr, param_ty))
        {
            prev_vars.insert(param_name, old);
        }
    }

    // Pre-compute result field pointer for async return interception
    let result_field_ptr = unsafe {
        ctx.builder().build_gep(
            state_ptr_val,
            &[
                i32_ty.const_int(0, false),
                i32_ty.const_int(result_field_idx as u64, false),
            ],
            "result_field",
        )
    };

    ctx.async_state_field_ptr = Some(state_field_ptr_poll);
    ctx.async_result_ptr = Some(result_field_ptr);
    ctx.async_return_bb = Some(poll_return);
    ctx.waker_ptr = Some(waker_ptr);

    let body_result = compile_block(ctx, body);

    let body_end_bb = ctx.builder().get_insert_block().unwrap();
    if body_end_bb.get_terminator().is_none() {
        ctx.builder().build_unconditional_branch(poll_return);
    }

    // Restore async context fields before building poll_return
    ctx.async_state_field_ptr = None;
    ctx.async_result_ptr = None;
    ctx.async_return_bb = None;
    ctx.waker_ptr = None;

    // Async return block: set state=done and return Ready(1)
    ctx.builder().position_at_end(poll_return);
    ctx.builder()
        .build_store(state_field_ptr_poll, i32_ty.const_int(1, false));
    ctx.builder()
        .build_return(Some(&i32_ty.const_int(1, false)));

    // Done block: return Ready(1)
    ctx.builder().position_at_end(poll_done);
    ctx.builder()
        .build_return(Some(&i32_ty.const_int(1, false)));

    ctx.set_current_function(prev_function);

    for (name, old) in prev_vars {
        ctx.define_variable(name, old);
    }

    // ── Fill in $new body (now that $poll exists) ──────────────
    let new_entry = ctx.context.append_basic_block(new_fn, "entry");
    let prev_builder_block = ctx.builder().get_insert_block();
    ctx.builder().position_at_end(new_entry);

    let struct_size = state_struct_type.size_of().unwrap();
    let alloc_ptr = crate::codegen::gc_alloc::GcAllocFn::for_mode(ctx.gc_mode).emit(
        ctx.builder(),
        ctx.module,
        struct_size,
    );
    let state_ptr = ctx
        .builder()
        .build_bitcast(alloc_ptr, state_ptr_type, "state_ptr")
        .into_pointer_value();

    let poll_fn = ctx
        .module
        .get_function(&format!("{}$poll", name))
        .expect("poll function should exist");
    let poll_fn_ptr_val = poll_fn.as_global_value().as_pointer_value();
    let poll_fn_field_ptr = unsafe {
        ctx.builder().build_gep(
            state_ptr,
            &[
                i32_ty.const_int(0, false),
                i32_ty.const_int(poll_fn_field_idx as u64, false),
            ],
            "poll_fn_field",
        )
    };
    ctx.builder()
        .build_store(poll_fn_field_ptr, poll_fn_ptr_val);

    let state_field_ptr_new = unsafe {
        ctx.builder().build_gep(
            state_ptr,
            &[
                i32_ty.const_int(0, false),
                i32_ty.const_int(state_field_idx as u64, false),
            ],
            "state_field",
        )
    };
    ctx.builder()
        .build_store(state_field_ptr_new, i32_ty.const_int(0, false));

    for (i, _param) in params.iter().enumerate() {
        let param_val = new_fn.get_nth_param(i as u32).unwrap();
        let field_ptr = unsafe {
            ctx.builder().build_gep(
                state_ptr,
                &[
                    i32_ty.const_int(0, false),
                    i32_ty.const_int(param_field_indices[i] as u64, false),
                ],
                &format!("param_{}_field", i),
            )
        };
        ctx.builder().build_store(field_ptr, param_val);
    }

    ctx.builder().build_return(Some(&alloc_ptr));

    if let Some(block) = prev_builder_block {
        ctx.builder().position_at_end(block);
    }

    // ── 3. Wrapper function: name ──────────────────────────────
    let wrapper_name = if name == "main" {
        "_ruyi_async_main"
    } else {
        name
    };
    let wrapper_fn_type = function_type_from_ruyi(
        ctx.context,
        &param_types,
        &Type::Future(Box::new(ret_type.clone())),
    );
    let wrapper_fn = ctx.module.add_function(wrapper_name, wrapper_fn_type, None);

    let wrapper_entry = ctx.context.append_basic_block(wrapper_fn, "entry");
    ctx.builder().position_at_end(wrapper_entry);

    let mut wrapper_args = Vec::new();
    for i in 0..params.len() {
        wrapper_args.push(wrapper_fn.get_nth_param(i as u32).unwrap().into());
    }

    let new_call = ctx.builder().build_call(new_fn, &wrapper_args, "new_call");
    let future_ptr = new_call.try_as_basic_value().left().unwrap();
    ctx.builder().build_return(Some(&future_ptr));

    // ── Restore codegen state ──────────────────────────────────
    ctx.try_frame_stack = saved_try_frame_stack;
    ctx.try_stack = saved_try_stack;
    ctx.loop_stack = saved_loop_stack;
    ctx.pending_return_flag = saved_pending_return_flag;
    ctx.pending_return_value = saved_pending_return_value;
    ctx.pending_break_target = saved_pending_break_target;
    ctx.pending_continue_target = saved_pending_continue_target;

    ctx.async_state_field_ptr = saved_async_state_field_ptr;
    ctx.async_result_ptr = saved_async_result_ptr;
    ctx.async_return_bb = saved_async_return_bb;
    ctx.waker_ptr = saved_waker_ptr;

    if let Some(block) = prev_block {
        ctx.builder().position_at_end(block);
    }

    body_result
}

/// Compile an `await` expression.
///
/// In synchronous contexts this calls `ruyi_await` as a blocking fallback.
/// Inside an async poll function it calls `ruyi_async_poll` with the waker,
/// then loads the actual result from the future's state struct.
pub fn compile_await<'ctx>(
    ctx: &mut CodegenContext<'ctx, '_, '_>,
    expr: &Expr,
) -> Result<super::expr::ExprResult<'ctx>, String> {
    let inner_result = compile_expr(ctx, expr)?;

    let i8_ptr = ctx.context.i8_type().ptr_type(Default::default());
    let i32_ty = ctx.context.i32_type();

    let waker = ctx.waker_ptr.unwrap_or_else(|| i8_ptr.const_null());

    let future_ptr = inner_result.value.into_pointer_value();

    let poll_result =
        super::builtins::build_ruyi_async_poll(ctx.builder(), ctx.module, future_ptr, waker);

    let result_ty = match &inner_result.ty {
        Type::Future(inner) => *inner.clone(),
        _ => Type::Int,
    };

    // Detect ReactorFuture calls: these futures use FFI _result functions
    // instead of a fixed-offset result field.
    let reactor_result_fn = detect_reactor_future(expr);

    let _result_llvm: inkwell::types::BasicTypeEnum<'ctx> = ctx.context.i64_type().into();
    let result_val = if let Some(result_fn_name) = reactor_result_fn {
        // ReactorFuture: emit call to __net_async_XXX_result(future_ptr)
        let result_fn = ctx.module.get_function(result_fn_name).unwrap_or_else(|| {
            let fn_type = match result_fn_name {
                n if n.contains("_read_result") => i8_ptr.fn_type(&[i8_ptr.into()], false),
                _ => i32_ty.fn_type(&[i8_ptr.into()], false),
            };
            ctx.module.add_function(result_fn_name, fn_type, None)
        });
        let call_result =
            ctx.builder()
                .build_call(result_fn, &[future_ptr.into()], "reactor_result");
        if result_fn_name.contains("_read_result") {
            call_result.try_as_basic_value().left().unwrap()
        } else {
            call_result.try_as_basic_value().left().unwrap()
        }
    } else if matches!(inner_result.ty, Type::Future(_)) {
        // Standard future: read result from fixed offset in state struct.
        let state_as_i8_ptr = ctx
            .builder()
            .build_bitcast(future_ptr, i8_ptr, "state_as_i8")
            .into_pointer_value();

        let result_offset = 8 + 8 + 8;
        let result_ptr = unsafe {
            ctx.builder().build_gep(
                state_as_i8_ptr,
                &[i32_ty.const_int(result_offset as u64, false)],
                "result_ptr",
            )
        };
        let typed_result_ptr = ctx
            .builder()
            .build_bitcast(
                result_ptr,
                ctx.context.i64_type().ptr_type(Default::default()),
                "typed_result_ptr",
            )
            .into_pointer_value();
        ctx.builder().build_load(typed_result_ptr, "await_result")
    } else {
        BasicValueEnum::IntValue(poll_result)
    };

    Ok(super::expr::ExprResult::new(result_val, result_ty))
}

/// Check if the awaited expression is a ReactorFuture call (__net_async_*).
/// Returns the name of the result extraction FFI if it is.
fn detect_reactor_future(expr: &Expr) -> Option<&'static str> {
    match expr {
        Expr::Call { callee, .. } => match callee.as_ref() {
            Expr::Identifier(name) if *name == "__net_async_read" => {
                Some("__net_async_read_result")
            }
            Expr::Identifier(name) if *name == "__net_async_write" => {
                Some("__net_async_write_result")
            }
            Expr::Identifier(name) if *name == "__net_async_accept" => {
                Some("__net_async_accept_result")
            }
            _ => None,
        },
        _ => None,
    }
}
