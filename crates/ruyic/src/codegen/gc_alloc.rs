/**
 * GC 分配函数分发表。
 *
 * Ruyi 编译器支持两种 GC 模式，对应不同的 LLVM-IR 分配函数：
 * - **Stub**（默认）：emit `call @cc_alloc`，外部声明；
 *   runtime 由 stub 链接脚本提供，无需链入真实 GC runtime。
 * - **Real**：emit `call @ruyi_gc_alloc`，外部声明，从
 *   `ruyi_runtime` 链入真实的 generational GC。
 *
 * 调用方通过 `GcAllocFn::for_mode(gc_mode)` 拿到本模块全局唯一的 dispatcher，
 * 然后在每个堆分配点调用 `dispatcher.emit(builder, module, size)` 生成
 * 对应的 LLVM `call` 指令。
 *
 * 单元测试覆盖 `for_mode` 的纯映射行为以及两种模式下 emit 的 IR 是否正确
 * 指向 `@cc_alloc` / `@ruyi_gc_alloc`。
 *
 * @author luozegang
 * @date 2026-07-10
 */
use inkwell::builder::Builder;
use inkwell::module::Module;
use inkwell::values::{IntValue, PointerValue};

use crate::cli::gc_mode::GcMode;

/// GC 分配函数 dispatcher。
///
/// 在 codegen 期间，`GcAllocFn` 决定堆分配走哪条路（cc_alloc stub 还是
/// ruyi_gc_alloc 真实 GC）。所有 5 个堆分配点（4 个在 expr.rs + 1 个
/// 在 async_codegen.rs）都通过本 enum 的 `emit` 方法发出 LLVM `call`。
pub enum GcAllocFn {
    /// Stub 模式：`call @cc_alloc`，占位分配器（默认）。
    Stub,
    /// Real 模式：`call @ruyi_gc_alloc`，真实 generational GC。
    Real,
}

impl GcAllocFn {
    /// 根据当前 GC 模式拿到对应的 dispatcher。
    ///
    /// 这是 codegen 阶段唯一构造 `GcAllocFn` 的入口，保证调用方不会
    /// 误用任何未注册的 variant。
    pub fn for_mode(mode: GcMode) -> Self {
        match mode {
            GcMode::Stub => GcAllocFn::Stub,
            GcMode::Real => GcAllocFn::Real,
        }
    }

    /// 当前 dispatcher 在 LLVM-IR 中暴露的函数名。
    ///
    /// Stub → `cc_alloc`
    /// Real → `ruyi_gc_alloc`
    pub fn fn_name(&self) -> &'static str {
        match self {
            GcAllocFn::Stub => "cc_alloc",
            GcAllocFn::Real => "ruyi_gc_alloc",
        }
    }

    /// Emit `call @<fn_name>(size)`，返回分配出的指针。
    ///
    /// `# Panics`
    ///
    /// 当对应函数尚未在本 module 中 declare 时会 panic。
    /// `declare_builtins` 必须在 codegen 之前调用。
    pub fn emit<'ctx>(
        &self,
        builder: &Builder<'ctx>,
        module: &Module<'ctx>,
        size: IntValue<'ctx>,
    ) -> PointerValue<'ctx> {
        let fn_val = module
            .get_function(self.fn_name())
            .unwrap_or_else(|| panic!("{} not declared", self.fn_name()));
        builder
            .build_call(fn_val, &[size.into()], "gc_alloc")
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_pointer_value()
    }
}

/// 在 LLVM module 中 declare 当前模式对应的分配器（external linkage）。
///
/// Stub 模式声明 `cc_alloc`，Real 模式声明 `ruyi_gc_alloc`。两个函数签名相同：
/// `i8* (i64)`，参数为字节数，返回值为分配出的对齐指针。
pub fn declare_alloc_fn<'ctx>(
    context: &'ctx inkwell::context::Context,
    module: &Module<'ctx>,
    mode: GcMode,
) {
    let alloc = GcAllocFn::for_mode(mode);
    if module.get_function(alloc.fn_name()).is_some() {
        return;
    }
    let i64_ty = context.i64_type();
    let i8_ptr = context.i8_type().ptr_type(inkwell::AddressSpace::default());
    let fn_type = i8_ptr.fn_type(&[i64_ty.into()], false);
    module.add_function(alloc.fn_name(), fn_type, None);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::gc_mode::GcMode;
    use inkwell::context::Context;

    /// `for_mode(Stub)` 必须返回 `GcAllocFn::Stub`。
    #[test]
    fn for_mode_maps_stub_to_stub() {
        let dispatcher = GcAllocFn::for_mode(GcMode::Stub);
        assert!(matches!(dispatcher, GcAllocFn::Stub));
    }

    /// `for_mode(Real)` 必须返回 `GcAllocFn::Real`。
    #[test]
    fn for_mode_maps_real_to_real() {
        let dispatcher = GcAllocFn::for_mode(GcMode::Real);
        assert!(matches!(dispatcher, GcAllocFn::Real));
    }

    /// Stub 模式的函数名是 `cc_alloc`。
    #[test]
    fn stub_fn_name_is_cc_alloc() {
        assert_eq!(GcAllocFn::Stub.fn_name(), "cc_alloc");
    }

    /// Real 模式的函数名是 `ruyi_gc_alloc`。
    #[test]
    fn real_fn_name_is_ruyi_gc_alloc() {
        assert_eq!(GcAllocFn::Real.fn_name(), "ruyi_gc_alloc");
    }

    /// 在 stub 模式 declare 出 `cc_alloc` 函数。
    #[test]
    fn declare_alloc_fn_stub_creates_cc_alloc() {
        let context = Context::create();
        let module = context.create_module("test_stub_alloc");
        declare_alloc_fn(&context, &module, GcMode::Stub);
        let fn_val = module
            .get_function("cc_alloc")
            .expect("cc_alloc should be declared in stub mode");
        assert_eq!(fn_val.get_name().to_str(), Ok("cc_alloc"));
    }

    /// Real 模式 declare 出 `ruyi_gc_alloc` 函数。
    #[test]
    fn declare_alloc_fn_real_creates_ruyi_gc_alloc() {
        let context = Context::create();
        let module = context.create_module("test_real_alloc");
        declare_alloc_fn(&context, &module, GcMode::Real);
        let fn_val = module
            .get_function("ruyi_gc_alloc")
            .expect("ruyi_gc_alloc should be declared in real mode");
        assert_eq!(fn_val.get_name().to_str(), Ok("ruyi_gc_alloc"));
    }

    /// 重复 declare 必须幂等 —— 不能新建同名函数也不能 panic。
    #[test]
    fn declare_alloc_fn_is_idempotent() {
        let context = Context::create();
        let module = context.create_module("test_idempotent");
        declare_alloc_fn(&context, &module, GcMode::Real);
        declare_alloc_fn(&context, &module, GcMode::Real);
        // 再次 declare 应该不会改变 module 状态。
        assert!(module.get_function("ruyi_gc_alloc").is_some());
    }

    /// Stub 模式 `emit` 生成 `call @cc_alloc`。
    #[test]
    fn emit_stub_produces_call_cc_alloc() {
        let context = Context::create();
        let module = context.create_module("test_emit_stub");
        declare_alloc_fn(&context, &module, GcMode::Stub);
        let builder = context.create_builder();

        // 准备一个 entry block 让 builder 有位置。
        let i64_ty = context.i64_type();
        let fn_type = context.void_type().fn_type(&[], false);
        let func = module.add_function("caller", fn_type, None);
        let bb = context.append_basic_block(func, "entry");
        builder.position_at_end(bb);

        let size = i64_ty.const_int(8, false);
        let _ptr = GcAllocFn::Stub.emit(&builder, &module, size);

        let ir = module.print_to_string().to_string();
        assert!(
            ir.contains("@cc_alloc") && ir.contains("call "),
            "stub emit should reference @cc_alloc via a `call`; got:\n{}",
            ir
        );
        assert!(
            ir.contains("declare i8* @cc_alloc"),
            "stub emit should declare i8* @cc_alloc; got:\n{}",
            ir
        );
    }

    /// Real 模式 `emit` 生成 `call @ruyi_gc_alloc`。
    #[test]
    fn emit_real_produces_call_ruyi_gc_alloc() {
        let context = Context::create();
        let module = context.create_module("test_emit_real");
        declare_alloc_fn(&context, &module, GcMode::Real);
        let builder = context.create_builder();

        let i64_ty = context.i64_type();
        let fn_type = context.void_type().fn_type(&[], false);
        let func = module.add_function("caller", fn_type, None);
        let bb = context.append_basic_block(func, "entry");
        builder.position_at_end(bb);

        let size = i64_ty.const_int(16, false);
        let _ptr = GcAllocFn::Real.emit(&builder, &module, size);

        let ir = module.print_to_string().to_string();
        assert!(
            ir.contains("@ruyi_gc_alloc") && ir.contains("call "),
            "real emit should reference @ruyi_gc_alloc via a `call`; got:\n{}",
            ir
        );
        assert!(
            ir.contains("declare i8* @ruyi_gc_alloc"),
            "real emit should declare i8* @ruyi_gc_alloc; got:\n{}",
            ir
        );
    }
}
