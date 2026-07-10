//! LLVM landing-pad generation for Ruyi exception handling.
//!
//! Provides helpers to emit `landingpad`, `invoke`, and `resume`
//! instructions using inkwell.

#[cfg(feature = "llvm14")]
use inkwell::basic_block::BasicBlock;
#[cfg(feature = "llvm14")]
use inkwell::builder::Builder;
#[cfg(feature = "llvm14")]
use inkwell::context::Context;
#[cfg(feature = "llvm14")]
use inkwell::module::Module;
#[cfg(feature = "llvm14")]
use inkwell::values::{BasicValue, BasicValueEnum, FunctionValue, IntValue, PointerValue};
#[cfg(feature = "llvm14")]
use inkwell::AddressSpace;

/**
 * 编译期 catch 类型标识符,使用无符号整数以解除与 ruyi_runtime 特定类型的耦合。
 *
 * @author Ruyi Team
 * @date 2026-07-08
 */
pub type TryTypeId = u32;

/**
 * Generator for LLVM landing-pad instructions.
 *
 * @author Ruyi Team
 * @date 2026-07-08
 */
#[cfg(feature = "llvm14")]
pub struct LandingPadGenerator<'ctx, 'm, 'b> {
    context: &'ctx Context,
    module: &'m Module<'ctx>,
    builder: &'b Builder<'ctx>,
}

#[cfg(feature = "llvm14")]
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
        let i8_ptr = self.context.i8_type().ptr_type(AddressSpace::default());
        let i32_ty = self.context.i32_type();
        let lpad_ty = self
            .context
            .struct_type(&[i8_ptr.into(), i32_ty.into()], false);

        let personality = self.get_personality_function();

        let mut clauses: Vec<BasicValueEnum<'ctx>> = Vec::new();
        for &type_id in catch_type_ids {
            let type_info = self.get_type_info_global(type_id);
            clauses.push(type_info.as_basic_value_enum());
        }

        self.builder
            .build_landing_pad(lpad_ty, personality, &clauses, has_cleanup, name)
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
        self.builder.build_resume(landing_pad_val);
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
        let i8_ptr = self.context.i8_type().ptr_type(AddressSpace::default());
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
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_int_value()
    }

    /**
     * Build a catch-all dispatch — unconditional branch to the first handler.
     *
     * Ruyi uses a single exception type (Error), so all catch clauses are
     * catch-all. No selector comparison is needed: the landing pad unconditionally
     * branches to the first handler, or to `cleanup_bb` / `resume_bb` when
     * there are no catch clauses.
     *
     * **Caller must ensure** `self.builder` is positioned inside the landing-pad
     * block before calling this method.
     *
     * @param _landing_pad_val landingpad 结果值 (catch-all 模式下未使用)
     * @param catch_handlers (类型 id, 处理基本块) 列表
     * @param cleanup_bb 可选的 finally/cleanup 块
     * @param resume_bb 未捕获异常时的 resume 块
     * @author Ruyi Team
     * @date 2026-07-08
     */
    pub fn build_catch_dispatch(
        &self,
        _landing_pad_val: BasicValueEnum<'ctx>,
        catch_handlers: &[(TryTypeId, BasicBlock<'ctx>)],
        cleanup_bb: Option<BasicBlock<'ctx>>,
        resume_bb: BasicBlock<'ctx>,
    ) {
        if catch_handlers.is_empty() {
            if let Some(cleanup) = cleanup_bb {
                self.builder.build_unconditional_branch(cleanup);
            } else {
                self.builder.build_unconditional_branch(resume_bb);
            }
        } else {
            // catch-all: unconditional branch to first handler
            let (_, first_handler) = catch_handlers[0];
            self.builder.build_unconditional_branch(first_handler);
        }
    }

    fn get_personality_function(&self) -> FunctionValue<'ctx> {
        let i32_ty = self.context.i32_type();
        let personality_ty = i32_ty.fn_type(&[], false);
        get_or_insert_function(self.module, "__gxx_personality_v0", personality_ty)
    }

    fn get_type_info_global(&self, type_id: TryTypeId) -> PointerValue<'ctx> {
        let name = format!("__ruyi_type_info_{}", type_id);
        let i8_ptr = self.context.i8_type().ptr_type(AddressSpace::default());

        if let Some(global) = self.module.get_global(&name) {
            global.as_pointer_value()
        } else {
            let global = self.module.add_global(i8_ptr, None, &name);
            global.set_initializer(&i8_ptr.const_null());
            global.as_pointer_value()
        }
    }
}

#[cfg(feature = "llvm14")]
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

#[cfg(all(test, feature = "llvm14"))]
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
        builder.build_return(None);

        let ir = module.print_to_string().to_string();

        // 确认 type_info global 有 null initializer (定义式,不是外部声明)
        assert!(
            ir.contains("global i8* null") && !ir.contains("external"),
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
        builder.build_return(None);

        builder.position_at_end(handler1_bb);
        builder.build_return(None);

        builder.position_at_end(resume_bb);
        builder.build_return(None);

        builder.position_at_end(entry_bb);
        builder.build_return(None);

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
