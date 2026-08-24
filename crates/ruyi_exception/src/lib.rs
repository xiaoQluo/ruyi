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

#[cfg(feature = "llvm20")]
pub use landing_pad::LandingPadGenerator;

/// Predefined exception type ids, mirrored from
/// `crates/ruyi_runtime/src/exception.rs::builtin_type_ids`.
///
/// The codegen must know these at compile time to expand `catch (e) {}`
/// (no annotation) into a catch-all landing-pad that matches every
/// concrete throw. Keeping the list here avoids a circular dependency
/// on `ruyi_runtime` while preserving the same numeric identifiers.
pub const ALL_BUILTIN_EXCEPTION_TYPE_IDS: &[TryTypeId] = &[
    0,  // builtin_type_ids::ANY       (catch-all sentinel)
    1,  // builtin_type_ids::ERROR
    2,  // builtin_type_ids::TYPE_ERROR
    3,  // builtin_type_ids::RANGE_ERROR
    4,  // builtin_type_ids::RUNTIME_ERROR
    5,  // builtin_type_ids::LOGIC_ERROR
    6,  // builtin_type_ids::ASSERTION_ERROR
    7,  // builtin_type_ids::ARGUMENT_ERROR
    8,  // builtin_type_ids::NULL_ERROR
    9,  // builtin_type_ids::ARITHMETIC_ERROR
    10, // builtin_type_ids::ITERATOR_ERROR
    11, // builtin_type_ids::PARSE_ERROR
    12, // builtin_type_ids::NULL_ASSERTION_ERROR
    13, // builtin_type_ids::IO_ERROR
];
