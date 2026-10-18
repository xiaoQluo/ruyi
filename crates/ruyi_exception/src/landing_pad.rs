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
        // EX-C4: Emit a single catch-all clause. The Itanium ABI wildcard
        // form matches any exception regardless of type_info. The runtime
        // `ruyi_throw` doesn't populate the type_info slot on the unwind
        // exception, so emitting one typed `catch @__ruyi_type_info_N`
        // clause per builtin exception type never matches and propagated
        // exceptions escaped the try, eventually aborting in `ruyi_throw`.
        //
        // We deduplicate: even if the source has both typed catches and a
        // catch-all, the personality routine only needs one `catch ptr null`
        // to accept the landing pad. The actual type filtering happens in
        // `build_catch_dispatch` against the runtime `ExceptionObject.type_tag`.
        if !catch_type_ids.is_empty() {
            clauses.push(i8_ptr.const_null().as_basic_value_enum());
        }

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

        // EX-C4 follow-up: A catch-all entry (`type_id == CATCH_ALL_TYPE_ID`) is emitted
        // as `catch ptr null` on the landing pad. The personality routine
        // accepts it without consulting type_info, so no selector comparison
        // is required here either — we can branch straight into the catch-all
        // handler.
        //
        // Since `build_landing_pad` now collapses *every* clause to
        // `catch ptr null` (because `ruyi_throw` doesn't populate the
        // `type_info` slot that typed `catch @__ruyi_type_info_N` would
        // require), the selector value at the landing pad is always the
        // wildcard sentinel and never matches any specific `llvm.eh.typeid.for`
        // selector. We perform the type comparison ourselves by reading
        // the runtime `ExceptionObject.type_tag` field at offset 0 of the
        // thrown exception object and switching through the typed catches
        // in source order. A catch-all (`type_id == CATCH_ALL_TYPE_ID`)
        // acts as the fallthrough handler for any exception that did not
        // match a typed clause — matching C++ semantics where typed catches
        // are tried before the catch-all.
        let func = self
            .builder
            .get_insert_block()
            .unwrap()
            .get_parent()
            .unwrap();
        let exc_ptr = self.extract_exception_ptr(landing_pad_val);
        // The runtime payload is an `ExceptionObject` (layout: type_tag
        // @0, message @8, ...). The user-visible `e` pointer that
        // codegen stores in the catch local is also pointed at the
        // payload start, so reading offset 0 of `exc_ptr` gives us the
        // runtime type_tag directly.
        let i64_ty = self.context.i64_type();
        let i8_ptr_ty = self.context.ptr_type(AddressSpace::default());
        let exc_typed = self
            .builder
            .build_pointer_cast(exc_ptr, i8_ptr_ty, "exc_typed")
            .unwrap();
        let type_tag_ptr = unsafe {
            self.builder
                .build_gep(
                    self.context.i8_type(),
                    exc_typed,
                    &[i64_ty.const_int(0, false)],
                    "type_tag_addr",
                )
                .unwrap()
        };
        let type_tag_ptr_typed = self
            .builder
            .build_pointer_cast(
                type_tag_ptr,
                self.context.ptr_type(AddressSpace::default()),
                "type_tag_typed",
            )
            .unwrap();
        let thrown_type = self
            .builder
            .build_load(i64_ty, type_tag_ptr_typed, "thrown_type_tag")
            .unwrap()
            .into_int_value();

        let mut next_bb: Option<BasicBlock<'ctx>> = None;
        for (i, &(type_id, handler_bb)) in catch_handlers.iter().enumerate() {
            if type_id == CATCH_ALL_TYPE_ID {
                // Catch-all: fall through (or jump directly if this is the
                // first handler).
                if let Some(bb) = next_bb {
                    self.builder.position_at_end(bb);
                }
                self.builder.build_unconditional_branch(handler_bb).unwrap();
                return;
            }
            // Typed catch: emit comparison and branch.
            let fallthrough = next_bb.unwrap_or_else(|| {
                self.context
                    .append_basic_block(func, &format!("catch.dispatch.fallthrough.{}", i))
            });
            let cond = self
                .builder
                .build_int_compare(
                    inkwell::IntPredicate::EQ,
                    thrown_type,
                    i64_ty.const_int(type_id as u64, false),
                    &format!("catch.matches.{}", i),
                )
                .unwrap();
            self.builder
                .build_conditional_branch(cond, handler_bb, fallthrough)
                .unwrap();
            self.builder.position_at_end(fallthrough);
            next_bb = Some(fallthrough);
        }
        // No catch matched; if we still have a next_bb sitting around,
        // branch it to the resume block so the personality routine can
        // propagate the exception out of the try.
        self.builder.build_unconditional_branch(resume_bb).unwrap();
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

        // 当前实现把所有子句折叠为 catch-all (`catch ptr null`)，不再
        // 生成 type_info global；本用例的意图是确认模块 IR 既不引用
        // 外部符号、也不留下未定义的 type_info 声明。
        assert!(
            !ir.contains("external"),
            "module must not reference external type_info declarations:\n{}",
            ir
        );
        assert!(
            ir.contains("catch ptr null"),
            "expected a catch-all landing pad:\n{}",
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

        // Catch-all dispatch: should branch to handler0 unconditionally.
        // CATCH_ALL_TYPE_ID (not 0) marks a catch-all clause.
        let catch_handlers = [
            (super::CATCH_ALL_TYPE_ID, handler0_bb),
            (super::CATCH_ALL_TYPE_ID, handler1_bb),
        ];
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
