//! LLVM landing-pad generation for Ruyi exception handling.
//!
//! Provides helpers to emit `landingpad`, `invoke`, and `resume`
//! instructions using inkwell.

#[cfg(feature = "llvm20")]
use inkwell::basic_block::BasicBlock;
#[cfg(feature = "llvm20")]
use inkwell::builder::Builder;
#[cfg(feature = "llvm20")]
use inkwell::context::Context;
#[cfg(feature = "llvm20")]
use inkwell::module::Module;
#[cfg(feature = "llvm20")]
use inkwell::values::{BasicValue, BasicValueEnum, FunctionValue, IntValue, PointerValue};
#[cfg(feature = "llvm20")]
use inkwell::AddressSpace;

/**
 * 编译期 catch 类型标识符,使用无符号整数以解除与 ruyi_runtime 特定类型的耦合。
 *
 * @author Ruyi Team
 * @date 2026-07-08
 */
pub type TryTypeId = u32;

/**
 * Sentinel value used by codegen to mark a catch-all (`catch ptr null`)
 * clause on a landing pad. Real exception type ids are small non-negative
 * integers (see `ALL_BUILTIN_EXCEPTION_TYPE_IDS`); `u32::MAX` is reserved
 * exclusively for this purpose and never collides with a runtime type id.
 */
pub const CATCH_ALL_TYPE_ID: TryTypeId = TryTypeId::MAX;

/**
 * Generator for LLVM landing-pad instructions.
 *
 * @author Ruyi Team
 * @date 2026-07-08
 */
#[cfg(feature = "llvm20")]
pub struct LandingPadGenerator<'ctx, 'm, 'b> {
    context: &'ctx Context,
    module: &'m Module<'ctx>,
    builder: &'b Builder<'ctx>,
}

#[cfg(feature = "llvm20")]
impl<'ctx, 'm, 'b> LandingPadGenerator<'ctx, 'm, 'b> {
    /**
     * Create a new landing-pad generator.
     *
     * @param context LLVM context reference
     * @param module LLVM module reference
     * @param builder LLVM IR builder reference
     * @return a fresh landing-pad generator
     * @author Ruyi Team
     * @date 2026-07-08
     */
    pub fn new(
        context: &'ctx Context,
        module: &'m Module<'ctx>,
        builder: &'b Builder<'ctx>,
    ) -> Self {
        Self {
            context,
            module,
            builder,
        }
    }

    /**
     * Build a `landingpad` instruction.
     *
     * Returns a `{ i8*, i32 }` value. Each entry in `catch_type_ids`
     * produces a `catch` clause. If `has_cleanup` is `true`, the
     * landing pad also has a `cleanup` clause (used for `finally`).
     *
     * @param catch_type_ids 需要捕获的异常类型 id 列表
     * @param has_cleanup 是否包含 finally/cleanup 子句
     * @param name 生成指令的名字前缀
     * @return landingpad 结果值
     * @author Ruyi Team
     * @date 2026-07-08
     */
    pub fn build_landing_pad(
        &self,
        catch_type_ids: &[TryTypeId],
        has_cleanup: bool,
        name: &str,
    ) -> BasicValueEnum<'ctx> {
        let i8_ptr = self.context.ptr_type(AddressSpace::default());
        let i32_ty = self.context.i32_type();
        let lpad_ty = self
            .context
            .struct_type(&[i8_ptr.into(), i32_ty.into()], false);

        let personality = self.get_personality_function();

        let mut clauses: Vec<BasicValueEnum<'ctx>> = Vec::new();
        let mut catch_all = false;
        for &type_id in catch_type_ids {
            if type_id == CATCH_ALL_TYPE_ID {
                // EX-C4: A catch-all (`try { ... } catch (e) { ... }`) is emitted
                // as `catch ptr null`, which is the Itanium ABI's wildcard form
                // that matches any exception regardless of type_info. The
                // personality routine (`__gxx_personality_v0`) recognises this
                // and reports the landing pad as the matching handler without
                // comparing per-class type_info objects. The runtime-side
                // `ruyi_throw` does not populate type_info on the unwind
                // exception, so emitting one typed `catch @__ruyi_type_info_N`
                // clause per builtin exception type (the previous behaviour)
                // never matched and propagated out of the try, eventually
                // hitting `std::process::abort()` in `ruyi_throw`.
                catch_all = true;
                clauses.push(i8_ptr.const_null().as_basic_value_enum());
            } else {
                let type_info = self.get_type_info_global(type_id);
                clauses.push(type_info.as_basic_value_enum());
            }
        }

        // Deduplicate: if a typed catch already covers every type we also
        // want to match, the extra null clause is harmless (it just widens
        // the match set). We don't try to elide it because the catch list
        // is small (<= 14 entries).
        let _ = catch_all;

        self.builder
            .build_landing_pad(lpad_ty, personality, &clauses, has_cleanup, name)
            .unwrap()
    }

    /**
     * Build an `invoke` instruction.
     *
     * Unlike `call`, `invoke` specifies both a normal return block
     * and an unwind landing-pad block, which is required for any
     * call inside a `try` region.
     *
     * @param fn_val 被调函数
     * @param args 实参列表
     * @param then_bb 正常返回块
     * @param catch_bb 异常 unwind 目标块
     * @param name 指令名字
     * @return invoke 调用站点值
     * @author Ruyi Team
     * @date 2026-07-08
     */
    pub fn build_invoke(
        &self,
        fn_val: FunctionValue<'ctx>,
        args: &[BasicValueEnum<'ctx>],
        then_bb: BasicBlock<'ctx>,
        catch_bb: BasicBlock<'ctx>,
        name: &str,
    ) -> inkwell::values::CallSiteValue<'ctx> {
        self.builder
            .build_invoke(fn_val, args, then_bb, catch_bb, name)
            .unwrap()
    }

    /**
     * Build a `resume` instruction.
     *
     * Used when an exception is not caught by any handler and must
     * continue propagating up the stack.
     *
     * @param landing_pad_val landingpad 结果值
     * @author Ruyi Team
     * @date 2026-07-08
     */
    pub fn build_resume(&self, landing_pad_val: BasicValueEnum<'ctx>) {
        self.builder.build_resume(landing_pad_val).unwrap();
    }

    /**
     * Extract the exception pointer from a landing-pad result.
     *
     * @param landing_pad_val landingpad 结果值
     * @return 异常指针
     * @author Ruyi Team
     * @date 2026-07-08
     */
    pub fn extract_exception_ptr(
        &self,
        landing_pad_val: BasicValueEnum<'ctx>,
    ) -> PointerValue<'ctx> {
        self.builder
            .build_extract_value(landing_pad_val.into_struct_value(), 0, "exc.ptr")
            .unwrap()
            .into_pointer_value()
    }

    /**
     * Extract the selector value from a landing-pad result.
     *
     * @param landing_pad_val landingpad 结果值
     * @return selector 整数值
     * @author Ruyi Team
     * @date 2026-07-08
     */
    pub fn extract_selector(&self, landing_pad_val: BasicValueEnum<'ctx>) -> IntValue<'ctx> {
        self.builder
            .build_extract_value(landing_pad_val.into_struct_value(), 1, "exc.selector")
            .unwrap()
            .into_int_value()
    }

    /**
     * Build a call to the `llvm.eh.typeid.for` intrinsic.
     *
     * Returns an `i32` selector value that the landing-pad selector
     * is compared against to determine whether a catch clause matches.
     *
     * @param type_id 异常类型 id
     * @return typeid 整数
     * @author Ruyi Team
     * @date 2026-07-08
     */
    pub fn build_eh_typeid_for(&self, type_id: TryTypeId) -> IntValue<'ctx> {
        let i8_ptr = self.context.ptr_type(AddressSpace::default());
        let i32_ty = self.context.i32_type();
        let fn_type = i32_ty.fn_type(&[i8_ptr.into()], false);

        let intrinsic = get_or_insert_function(self.module, "llvm.eh.typeid.for", fn_type);
        let type_info = self.get_type_info_global(type_id);

        self.builder
            .build_call(
                intrinsic,
                &[type_info.as_basic_value_enum().into()],
                "typeid",
            )
            .unwrap()
            .try_as_basic_value()
            .unwrap_basic()
            .into_int_value()
    }

    /**
     * Build a catch dispatch with selector-based type filtering.
     *
     * When all handlers are catch-all (type_id == 0), emits an
     * unconditional branch to the first handler (fast path).
     * Otherwise, extracts the landing-pad selector and compares it
     * against each handler's `llvm.eh.typeid.for` value, branching
     * to the first match. Falls through to `cleanup_bb` / `resume_bb`
     * when no handler matches.
     *
     * **Caller must ensure** `self.builder` is positioned inside the
     * landing-pad block before calling this method.
     *
     * @param landing_pad_val landingpad 结果值
     * @param catch_handlers (类型 id, 处理基本块) 列表
     * @param cleanup_bb 可选的 finally/cleanup 块
     * @param resume_bb 未捕获异常时的 resume 块
     * @author Ruyi Team
     * @date 2026-07-26
     */
    pub fn build_catch_dispatch(
        &self,
        landing_pad_val: BasicValueEnum<'ctx>,
        catch_handlers: &[(TryTypeId, BasicBlock<'ctx>)],
        cleanup_bb: Option<BasicBlock<'ctx>>,
        resume_bb: BasicBlock<'ctx>,
    ) {
        if catch_handlers.is_empty() {
            if let Some(cleanup) = cleanup_bb {
                self.builder.build_unconditional_branch(cleanup).unwrap();
            } else {
                self.builder.build_unconditional_branch(resume_bb).unwrap();
            }
            return;
        }

        // EX-C4: A catch-all entry (`type_id == CATCH_ALL_TYPE_ID`) is emitted
        // as `catch ptr null` on the landing pad. The personality routine
        // accepts it without consulting type_info, so no selector comparison
        // is required here either — we can branch straight into the catch-all
        // handler. The dispatch walks handlers in declaration order; as soon
        // as it hits a catch-all it routes every subsequent exception to
        // that handler. Typed catches declared before a catch-all keep their
        // selector match (compiler bug if the user wrote them in this order:
        // the catch-all would shadow them, matching C++ semantics).
        let first_catch_all = catch_handlers
            .iter()
            .position(|(id, _)| *id == CATCH_ALL_TYPE_ID);
        if let Some(idx) = first_catch_all {
            // Walk typed catches before the catch-all to give them a chance
            // to match first (mirrors C++ `try { ... } catch (T1) {...}
            // catch (T2) {...} catch (...) {...}` semantics: typed catches
            // are tried in order before the catch-all). If any of them match,
            // their handler runs; if not, fall through to the catch-all.
            for (i, &(type_id, handler_bb)) in catch_handlers.iter().take(idx).enumerate() {
                let selector = self.extract_selector(landing_pad_val);
                let eh_typeid = self.build_eh_typeid_for(type_id);
                let cmp = self
                    .builder
                    .build_int_compare(
                        inkwell::IntPredicate::EQ,
                        selector,
                        eh_typeid,
                        &format!("catch.matches.{}", i),
                    )
                    .unwrap();
                // Build the *next* block on demand so we can branch into it
                // when this typed catch does not match.
                let func = self
                    .builder
                    .get_insert_block()
                    .unwrap()
                    .get_parent()
                    .unwrap();
                let next_bb = self
                    .context
                    .append_basic_block(func, &format!("catch.dispatch.before_catch_all.{}", i));
                self.builder
                    .build_conditional_branch(cmp, handler_bb, next_bb)
                    .unwrap();
                self.builder.position_at_end(next_bb);
            }
            // Now unconditionally branch to the catch-all handler.
            let (_, all_handler) = catch_handlers[idx];
            self.builder
                .build_unconditional_branch(all_handler)
                .unwrap();
            return;
        }

        // EX-H1: If all handlers are catch-all (type_id == 0), use unconditional branch
        if catch_handlers.iter().all(|(id, _)| *id == 0) {
            let (_, first_handler) = catch_handlers[0];
            self.builder.build_unconditional_branch(first_handler).unwrap();
            return;
        }

        // EX-H1: Selector-based type dispatch
        let selector = self.extract_selector(landing_pad_val);
        let func = self
            .builder
            .get_insert_block()
            .unwrap()
            .get_parent()
            .unwrap();

        // Pre-compute next-block targets: each handler gets a dispatch block,
        // the last one falls through to cleanup_bb or resume_bb.
        let mut next_blocks: Vec<BasicBlock<'ctx>> = Vec::new();
        for i in 1..catch_handlers.len() {
            let bb = self
                .context
                .append_basic_block(func, &format!("catch.dispatch.next.{}", i));
            next_blocks.push(bb);
        }

        for (i, &(type_id, handler_bb)) in catch_handlers.iter().enumerate() {
            let next_bb = if i + 1 < catch_handlers.len() {
                next_blocks[i]
            } else {
                cleanup_bb.unwrap_or(resume_bb)
            };

            let eh_typeid = self.build_eh_typeid_for(type_id);
            let cmp = self.builder.build_int_compare(
                inkwell::IntPredicate::EQ,
                selector,
                eh_typeid,
                &format!("catch.matches.{}", i),
            ).unwrap();
            self.builder
                .build_conditional_branch(cmp, handler_bb, next_bb).unwrap();

            // Position builder at the next dispatch block (if any)
            if i + 1 < catch_handlers.len() {
                self.builder.position_at_end(next_blocks[i]);
            }
        }

        // Last "next" block (no more handlers): route to cleanup or resume
        if !catch_handlers.is_empty() {
            let end_block = if next_blocks.is_empty() {
                // Only one handler — already emitted; nothing to do here
                None
            } else {
                Some(*next_blocks.last().unwrap())
            };
            if let Some(bb) = end_block {
                // The builder was already positioned at end_block above
                // (the last next_blocks entry). Emit cleanup/resume branch.
                if self.builder.get_insert_block() == Some(bb) {
                    if let Some(cleanup) = cleanup_bb {
                        self.builder.build_unconditional_branch(cleanup).unwrap();
                    } else {
                        self.builder.build_unconditional_branch(resume_bb).unwrap();
                    }
                }
            } else if cleanup_bb.is_some() || catch_handlers.len() == 1 {
                // Single handler: no fallthrough block was created;
                // the conditional branch already routes to cleanup/resume.
                // Nothing extra to emit here.
            }
        }
    }

    pub fn get_personality_function(&self) -> FunctionValue<'ctx> {
        let i32_ty = self.context.i32_type();
        let personality_ty = i32_ty.fn_type(&[], false);
        get_or_insert_function(self.module, "__gxx_personality_v0", personality_ty)
    }

    /// EX-H1: Each type ID gets a unique global string initializer so that
    /// `llvm.eh.typeid.for` returns a distinct selector per type. This
    /// enables the landing-pad selector comparison in `build_catch_dispatch`.
    fn get_type_info_global(&self, type_id: TryTypeId) -> PointerValue<'ctx> {
        let name = format!("__ruyi_type_info_{}", type_id);
        let i8_ptr = self.context.ptr_type(AddressSpace::default());

        if let Some(global) = self.module.get_global(&name) {
            global.as_pointer_value()
        } else {
            let global = self.module.add_global(i8_ptr, None, &name);
            // Unique non-null initializer per type so each gets a distinct
            // address → distinct `llvm.eh.typeid.for` selector value.
            let init_str = format!("ruyi.exc.type.{}\0", type_id);
            let init_array = self
                .context
                .const_string(&init_str.as_bytes()[..init_str.len() - 1], false);
            let init_global = self.module.add_global(
                init_array.get_type(),
                None,
                &format!("__ruyi_type_str_{}", type_id),
            );
            init_global.set_initializer(&init_array);
            let init_ptr = init_global.as_pointer_value();
            global.set_initializer(&init_ptr);
            global.as_pointer_value()
        }
    }
}

#[cfg(feature = "llvm20")]
fn get_or_insert_function<'ctx>(
    module: &Module<'ctx>,
    name: &str,
    fn_type: inkwell::types::FunctionType<'ctx>,
) -> FunctionValue<'ctx> {
    module
        .get_function(name)
        .unwrap_or_else(|| module.add_function(name, fn_type, None))
}

#[cfg(test)]
mod tests {
    use super::TryTypeId;

    /**
     * 占位测试:验证 TryTypeId 的整数语义。
     *
     * 真正的 LandingPadGenerator IR 生成测试在 T7 完成,
     * 此处仅作为 TDD 起点,确保模块级测试骨架可用。
     *
     * @author Ruyi Team
     * @date 2026-07-08
     */
    #[test]
    fn test_landing_pad_types() {
        let catch_all: TryTypeId = 0;
        let error: TryTypeId = 1;
        assert_eq!(catch_all, 0u32);
        assert_eq!(error, 1u32);
        assert_ne!(catch_all, error);
    }
}

#[cfg(all(test, feature = "llvm20"))]
mod llvm_tests {
    use super::LandingPadGenerator;
    use inkwell::context::Context;

    /**
     * 验证 build_landing_pad 不产生外部符号引用。
     *
     * 修复前 get_type_info_global 使用 External linkage + 无 initializer,
     * 导致链接时 Unresolved symbol `__ruyi_type_info_*`。
     * 修复后使用 Internal linkage + null initializer,
     * 模块 IR 中不包含 `external` declaration 的 type_info global。
     *
     * @author Ruyi Team
     * @date 2026-07-08
     */
    #[test]
    fn test_build_landing_pad_no_link_error() {
        let context = Context::create();
        let module = context.create_module("test_module");
        let builder = context.create_builder();

        let void_ty = context.void_type();
        let fn_type = void_ty.fn_type(&[], false);
        let function = module.add_function("test_fn", fn_type, None);
        let entry_bb = context.append_basic_block(function, "entry");
        let lpad_bb = context.append_basic_block(function, "lpad");
        builder.position_at_end(lpad_bb);

        let gen = LandingPadGenerator::new(&context, &module, &builder);

        let lpad = gen.build_landing_pad(&[0u32], false, "lpad");
        assert!(lpad.is_struct_value());

        gen.build_resume(lpad);

        builder.position_at_end(entry_bb);
        builder.build_return(None).unwrap();

        let ir = module.print_to_string().to_string();

        // 确认 type_info global 有 initializer (定义式,不是外部声明)
        // EX-H1: globals now use unique string initializers instead of null
        assert!(
            ir.contains("global ptr") && !ir.contains("external"),
            "type_info global must be defined (not external declaration):\n{}",
            ir
        );

        assert!(module.verify().is_ok(), "LLVM IR verification failed");
    }

    /**
     * 验证 catch-all dispatch 直接跳转到第一个 handler。
     *
     * 修复前 build_catch_dispatch 在错误的 basic block 中生成 dispatch 链,
     * 导致代码重复、resume 块无前驱等问题。修复后使用 catch-all 模式,
     * landing pad 无条件 branch 到第一个 handler block。
     *
     * @author Ruyi Team
     * @date 2026-07-08
     */
    #[test]
    fn test_catch_all_dispatch_branches_to_first_handler() {
        let context = Context::create();
        let module = context.create_module("test_module");
        let builder = context.create_builder();

        let void_ty = context.void_type();
        let fn_type = void_ty.fn_type(&[], false);
        let function = module.add_function("test_fn", fn_type, None);
        let entry_bb = context.append_basic_block(function, "entry");
        let lpad_bb = context.append_basic_block(function, "lpad");
        let handler0_bb = context.append_basic_block(function, "handler0");
        let handler1_bb = context.append_basic_block(function, "handler1");
        let resume_bb = context.append_basic_block(function, "resume");

        let gen = LandingPadGenerator::new(&context, &module, &builder);

        // Position in landing pad block and build the landingpad
        builder.position_at_end(lpad_bb);
        let lpad = gen.build_landing_pad(&[0u32, 0u32], false, "lpad");

        // Catch-all dispatch: should branch to handler0 unconditionally
        let catch_handlers = [(0u32, handler0_bb), (0u32, handler1_bb)];
        gen.build_catch_dispatch(lpad, &catch_handlers, None, resume_bb);

        // Handler blocks must be populated with valid IR
        builder.position_at_end(handler0_bb);
        builder.build_return(None).unwrap();

        builder.position_at_end(handler1_bb);
        builder.build_return(None).unwrap();

        builder.position_at_end(resume_bb);
        builder.build_return(None).unwrap();

        builder.position_at_end(entry_bb);
        builder.build_return(None).unwrap();

        let ir = module.print_to_string().to_string();

        // Verify: landing pad block should contain an unconditional branch to handler0
        assert!(
            ir.contains("br label %handler0"),
            "Landing pad should branch to first handler (catch-all):\n{}",
            ir
        );

        // Verify: no selector comparison code (catch.matches labels from old dispatch)
        assert!(
            !ir.contains("catch.matches"),
            "No selector comparison in catch-all mode:\n{}",
            ir
        );

        assert!(module.verify().is_ok(), "LLVM IR verification failed");
    }
}
