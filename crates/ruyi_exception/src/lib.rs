/*!
 * ruyi_exception
 *
 * 供 Ruyi 编译器(ruyic)与运行时(ruyi_runtime)共用的异常处理基础 crate。
 * 当前核心为 `LandingPadGenerator`,用于生成 LLVM `landingpad`/`invoke`/
 * `resume` 指令;启用 `llvm14` feature 时才包含 inkwell 相关实现。
 *
 * @author Ruyi Team
 * @date 2026-07-08
 */

pub mod landing_pad;

pub use landing_pad::TryTypeId;

#[cfg(feature = "llvm14")]
pub use landing_pad::LandingPadGenerator;
