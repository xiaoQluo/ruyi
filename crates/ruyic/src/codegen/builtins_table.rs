/**
 * Table-driven FFI declaration dispatch (v0.5.9-stdlib-cleanup / R5).
 *
 * Each `BuiltinDecl` entry is converted to an LLVM `declare` instruction at
 * `declare_builtins()` time, replacing the original 55 hand-written
 * `fn declare_xxx` wrappers. The same table is walked by
 * `typechecker::inference::resolve_builtin_name` so typecheck and codegen
 * agree on every FFI name.
 *
 * `BuiltinSig::Bool` is needed because the runtime exports predicates as
 * `i1` (`__builtin_map_has`, `__builtin_set_has`, `__builtin_set_delete`,
 * `__string_contains`, `__string_starts_with`, `__string_ends_with`).
 * The typecheck layer collapses these to `Type::Bool` via the
 * `builtin_sig_to_type` helper in `inference.rs`.
 *
 * Order: array (6) → map (7) → set (4) → string (18) → math (14) → time (4) → json (2) → path (8) → io (17) → process (20).
 *
 * @author Ruyi Team
 * @date 2026-07-12
 */
use inkwell::context::Context;
use inkwell::types::{BasicMetadataTypeEnum, BasicType};

/// LLVM ABI signature kind for a builtin return or parameter.
#[derive(Clone, Copy)]
pub enum BuiltinSig {
    /// `void` — only valid as a return type.
    Void,
    /// `i64` — 64-bit signed integer.
    Int,
    /// `f64` — 64-bit IEEE 754 float.
    Float,
    /// `i1` — boolean predicate.
    Bool,
    /// `*mut i8` — null-terminated C string (UTF-8 text input/output).
    String,
    /// `*mut i8` — opaque pointer (collections, refs, FFI handles).
    Ptr,
}

/// One FFI entry declared from the static `BUILTINS` table.
pub struct BuiltinDecl {
    /// Symbol name as it appears in LLVM IR (matches the runtime export).
    pub name: &'static str,
    /// Return type signature.
    pub ret: BuiltinSig,
    /// Parameter type signatures, in declaration order.
    pub params: &'static [BuiltinSig],
}

/// All 55 FFI entries known to the compiler.
///
/// Each entry mirrors the body of the corresponding `fn declare_xxx` that
/// previously lived in `codegen/builtins.rs`; the LLVM ABI must be identical.
pub static BUILTINS: &[BuiltinDecl] = &[
    // ============================================================
    // __builtin_array_* (6)
    // ============================================================
    BuiltinDecl {
        name: "__builtin_array_create",
        ret: BuiltinSig::Ptr,
        params: &[],
    },
    BuiltinDecl {
        name: "__builtin_array_get",
        ret: BuiltinSig::Int,
        params: &[BuiltinSig::Ptr, BuiltinSig::Int],
    },
    BuiltinDecl {
        name: "__builtin_array_set",
        ret: BuiltinSig::Void,
        params: &[BuiltinSig::Ptr, BuiltinSig::Int, BuiltinSig::Int],
    },
    BuiltinDecl {
        name: "__builtin_array_push",
        ret: BuiltinSig::Ptr,
        params: &[BuiltinSig::Ptr, BuiltinSig::Int],
    },
    BuiltinDecl {
        name: "__builtin_array_pop",
        ret: BuiltinSig::Int,
        params: &[BuiltinSig::Ptr],
    },
    BuiltinDecl {
        name: "__builtin_array_length",
        ret: BuiltinSig::Int,
        params: &[BuiltinSig::Ptr],
    },
    // ============================================================
    // __builtin_map_* (7)
    // ============================================================
    BuiltinDecl {
        name: "__builtin_map_create",
        ret: BuiltinSig::Ptr,
        params: &[],
    },
    BuiltinDecl {
        name: "__builtin_map_get",
        ret: BuiltinSig::Ptr,
        params: &[BuiltinSig::Ptr, BuiltinSig::Ptr],
    },
    BuiltinDecl {
        name: "__builtin_map_set",
        ret: BuiltinSig::Void,
        params: &[BuiltinSig::Ptr, BuiltinSig::Ptr, BuiltinSig::Ptr],
    },
    BuiltinDecl {
        name: "__builtin_map_delete",
        ret: BuiltinSig::Void,
        params: &[BuiltinSig::Ptr, BuiltinSig::Ptr],
    },
    BuiltinDecl {
        name: "__builtin_map_has",
        ret: BuiltinSig::Bool,
        params: &[BuiltinSig::Ptr, BuiltinSig::Ptr],
    },
    BuiltinDecl {
        name: "__builtin_map_keys",
        ret: BuiltinSig::Ptr,
        params: &[BuiltinSig::Ptr],
    },
    BuiltinDecl {
        name: "__builtin_map_values",
        ret: BuiltinSig::Ptr,
        params: &[BuiltinSig::Ptr],
    },
    // ============================================================
    // __builtin_set_* (4)
    // ============================================================
    BuiltinDecl {
        name: "__builtin_set_create",
        ret: BuiltinSig::Ptr,
        params: &[],
    },
    BuiltinDecl {
        name: "__builtin_set_add",
        ret: BuiltinSig::Void,
        params: &[BuiltinSig::Ptr, BuiltinSig::Ptr],
    },
    BuiltinDecl {
        name: "__builtin_set_delete",
        ret: BuiltinSig::Bool,
        params: &[BuiltinSig::Ptr, BuiltinSig::Ptr],
    },
    BuiltinDecl {
        name: "__builtin_set_has",
        ret: BuiltinSig::Bool,
        params: &[BuiltinSig::Ptr, BuiltinSig::Ptr],
    },
    // ============================================================
    // __string_* (18)
    // ============================================================
    BuiltinDecl {
        name: "__string_join",
        ret: BuiltinSig::Ptr,
        params: &[BuiltinSig::Ptr, BuiltinSig::Ptr],
    },
    BuiltinDecl {
        name: "__string_from_char_code",
        ret: BuiltinSig::Ptr,
        params: &[BuiltinSig::Int],
    },
    BuiltinDecl {
        name: "__string_from_char_codes",
        ret: BuiltinSig::Ptr,
        params: &[BuiltinSig::Ptr],
    },
    BuiltinDecl {
        name: "__string_replace_all_legacy",
        ret: BuiltinSig::Ptr,
        params: &[BuiltinSig::Ptr, BuiltinSig::Ptr, BuiltinSig::Ptr],
    },
    // v0.5.9 / R3: canonical 8-arg bounded-buffer variant. Renamed from
    // `ruyi_string_replace_all` (was in fmt_ffi.rs). Caller supplies
    // output buffer via out/out_cap; function returns bytes written.
    BuiltinDecl {
        name: "__string_replace_all",
        ret: BuiltinSig::Int,
        params: &[
            BuiltinSig::Ptr,
            BuiltinSig::Int,
            BuiltinSig::Ptr,
            BuiltinSig::Int,
            BuiltinSig::Ptr,
            BuiltinSig::Int,
            BuiltinSig::Ptr,
            BuiltinSig::Int,
        ],
    },
    BuiltinDecl {
        name: "__string_length",
        ret: BuiltinSig::Int,
        params: &[BuiltinSig::Ptr],
    },
    BuiltinDecl {
        name: "__string_contains",
        ret: BuiltinSig::Bool,
        params: &[BuiltinSig::Ptr, BuiltinSig::Ptr],
    },
    BuiltinDecl {
        name: "__string_starts_with",
        ret: BuiltinSig::Bool,
        params: &[BuiltinSig::Ptr, BuiltinSig::Ptr],
    },
    BuiltinDecl {
        name: "__string_ends_with",
        ret: BuiltinSig::Bool,
        params: &[BuiltinSig::Ptr, BuiltinSig::Ptr],
    },
    BuiltinDecl {
        name: "__string_index_of",
        ret: BuiltinSig::Int,
        params: &[BuiltinSig::Ptr, BuiltinSig::Ptr],
    },
    BuiltinDecl {
        name: "__string_last_index_of",
        ret: BuiltinSig::Int,
        params: &[BuiltinSig::Ptr, BuiltinSig::Ptr],
    },
    BuiltinDecl {
        name: "__string_char_at",
        ret: BuiltinSig::Ptr,
        params: &[BuiltinSig::Ptr, BuiltinSig::Int],
    },
    BuiltinDecl {
        name: "__string_char_code_at",
        ret: BuiltinSig::Int,
        params: &[BuiltinSig::Ptr, BuiltinSig::Int],
    },
    BuiltinDecl {
        name: "__string_repeat",
        ret: BuiltinSig::Ptr,
        params: &[BuiltinSig::Ptr, BuiltinSig::Int],
    },
    BuiltinDecl {
        name: "__string_substring",
        ret: BuiltinSig::Ptr,
        params: &[BuiltinSig::Ptr, BuiltinSig::Int, BuiltinSig::Int],
    },
    BuiltinDecl {
        name: "__string_to_upper_case",
        ret: BuiltinSig::Ptr,
        params: &[BuiltinSig::Ptr],
    },
    BuiltinDecl {
        name: "__string_to_lower_case",
        ret: BuiltinSig::Ptr,
        params: &[BuiltinSig::Ptr],
    },
    BuiltinDecl {
        name: "__string_trim",
        ret: BuiltinSig::Ptr,
        params: &[BuiltinSig::Ptr],
    },
    BuiltinDecl {
        name: "__string_split",
        ret: BuiltinSig::Ptr,
        params: &[BuiltinSig::Ptr, BuiltinSig::Ptr],
    },
    // ============================================================
    // __math_* (14)
    // ============================================================
    BuiltinDecl {
        name: "__math_pi",
        ret: BuiltinSig::Float,
        params: &[],
    },
    BuiltinDecl {
        name: "__math_e",
        ret: BuiltinSig::Float,
        params: &[],
    },
    BuiltinDecl {
        name: "__math_sqrt",
        ret: BuiltinSig::Float,
        params: &[BuiltinSig::Float],
    },
    BuiltinDecl {
        name: "__math_pow",
        ret: BuiltinSig::Float,
        params: &[BuiltinSig::Float, BuiltinSig::Float],
    },
    BuiltinDecl {
        name: "__math_abs",
        ret: BuiltinSig::Float,
        params: &[BuiltinSig::Float],
    },
    BuiltinDecl {
        name: "__math_min",
        ret: BuiltinSig::Float,
        params: &[BuiltinSig::Float, BuiltinSig::Float],
    },
    BuiltinDecl {
        name: "__math_max",
        ret: BuiltinSig::Float,
        params: &[BuiltinSig::Float, BuiltinSig::Float],
    },
    BuiltinDecl {
        name: "__math_sin",
        ret: BuiltinSig::Float,
        params: &[BuiltinSig::Float],
    },
    BuiltinDecl {
        name: "__math_cos",
        ret: BuiltinSig::Float,
        params: &[BuiltinSig::Float],
    },
    BuiltinDecl {
        name: "__math_tan",
        ret: BuiltinSig::Float,
        params: &[BuiltinSig::Float],
    },
    BuiltinDecl {
        name: "__math_log",
        ret: BuiltinSig::Float,
        params: &[BuiltinSig::Float],
    },
    BuiltinDecl {
        name: "__math_ceil",
        ret: BuiltinSig::Float,
        params: &[BuiltinSig::Float],
    },
    BuiltinDecl {
        name: "__math_floor",
        ret: BuiltinSig::Float,
        params: &[BuiltinSig::Float],
    },
    BuiltinDecl {
        name: "__math_round",
        ret: BuiltinSig::Float,
        params: &[BuiltinSig::Float],
    },
    // ============================================================
    // __time_* (4)
    // ============================================================
    BuiltinDecl {
        name: "__time_now",
        ret: BuiltinSig::Int,
        params: &[],
    },
    BuiltinDecl {
        name: "__time_timestamp",
        ret: BuiltinSig::Int,
        params: &[],
    },
    BuiltinDecl {
        name: "__time_sleep",
        ret: BuiltinSig::Void,
        params: &[BuiltinSig::Float],
    },
    BuiltinDecl {
        name: "__time_format",
        ret: BuiltinSig::String,
        params: &[BuiltinSig::Int],
    },
    // ============================================================
    // __json_* (2)
    // ============================================================
    BuiltinDecl {
        name: "__json_parse",
        ret: BuiltinSig::String,
        params: &[BuiltinSig::String],
    },
    BuiltinDecl {
        name: "__json_stringify",
        ret: BuiltinSig::String,
        params: &[BuiltinSig::String],
    },
    // ============================================================
    // __path_* (8)
    // ============================================================
    BuiltinDecl {
        name: "__path_join",
        ret: BuiltinSig::String,
        params: &[BuiltinSig::Ptr],
    },
    BuiltinDecl {
        name: "__path_basename",
        ret: BuiltinSig::String,
        params: &[BuiltinSig::String],
    },
    BuiltinDecl {
        name: "__path_dirname",
        ret: BuiltinSig::String,
        params: &[BuiltinSig::String],
    },
    BuiltinDecl {
        name: "__path_extname",
        ret: BuiltinSig::String,
        params: &[BuiltinSig::String],
    },
    BuiltinDecl {
        name: "__path_is_absolute",
        ret: BuiltinSig::Bool,
        params: &[BuiltinSig::String],
    },
    BuiltinDecl {
        name: "__path_normalize",
        ret: BuiltinSig::String,
        params: &[BuiltinSig::String],
    },
    BuiltinDecl {
        name: "__path_separator",
        ret: BuiltinSig::String,
        params: &[],
    },
    BuiltinDecl {
        name: "__path_relative",
        ret: BuiltinSig::String,
        params: &[BuiltinSig::String, BuiltinSig::String],
    },
    // ============================================================
    // __io_* (17)
    // ============================================================
    // Sync (9)
    BuiltinDecl {
        name: "__io_read_line",
        ret: BuiltinSig::String,
        params: &[],
    },
    BuiltinDecl {
        name: "__io_file_read_text",
        ret: BuiltinSig::String,
        params: &[BuiltinSig::String],
    },
    BuiltinDecl {
        name: "__io_file_write_text",
        ret: BuiltinSig::Void,
        params: &[BuiltinSig::String, BuiltinSig::String],
    },
    BuiltinDecl {
        name: "__io_file_read_lines",
        ret: BuiltinSig::Ptr,
        params: &[BuiltinSig::String],
    },
    BuiltinDecl {
        name: "__io_file_exists",
        ret: BuiltinSig::Bool,
        params: &[BuiltinSig::String],
    },
    BuiltinDecl {
        name: "__io_is_directory",
        ret: BuiltinSig::Bool,
        params: &[BuiltinSig::String],
    },
    BuiltinDecl {
        name: "__io_is_file",
        ret: BuiltinSig::Bool,
        params: &[BuiltinSig::String],
    },
    BuiltinDecl {
        name: "__io_file_delete",
        ret: BuiltinSig::Void,
        params: &[BuiltinSig::String],
    },
    BuiltinDecl {
        name: "__io_mkdir",
        ret: BuiltinSig::Void,
        params: &[BuiltinSig::String, BuiltinSig::Bool],
    },
    // Async (8)
    BuiltinDecl {
        name: "__io_file_read_text_async",
        ret: BuiltinSig::Ptr,
        params: &[BuiltinSig::String],
    },
    BuiltinDecl {
        name: "__io_file_write_text_async",
        ret: BuiltinSig::Ptr,
        params: &[BuiltinSig::String, BuiltinSig::String],
    },
    BuiltinDecl {
        name: "__io_file_read_lines_async",
        ret: BuiltinSig::Ptr,
        params: &[BuiltinSig::String],
    },
    BuiltinDecl {
        name: "__io_file_exists_async",
        ret: BuiltinSig::Ptr,
        params: &[BuiltinSig::String],
    },
    BuiltinDecl {
        name: "__io_is_directory_async",
        ret: BuiltinSig::Ptr,
        params: &[BuiltinSig::String],
    },
    BuiltinDecl {
        name: "__io_is_file_async",
        ret: BuiltinSig::Ptr,
        params: &[BuiltinSig::String],
    },
    BuiltinDecl {
        name: "__io_file_delete_async",
        ret: BuiltinSig::Ptr,
        params: &[BuiltinSig::String],
    },
    BuiltinDecl {
        name: "__io_mkdir_async",
        ret: BuiltinSig::Ptr,
        params: &[BuiltinSig::String, BuiltinSig::Bool],
    },
    // ============================================================
    // __process_* (20)
    // ============================================================
    // Execution + lifecycle (9)
    BuiltinDecl {
        name: "__process_exec",
        ret: BuiltinSig::Ptr,
        params: &[BuiltinSig::String],
    },
    BuiltinDecl {
        name: "__process_exec_with",
        ret: BuiltinSig::Ptr,
        params: &[
            BuiltinSig::String,
            BuiltinSig::String,
            BuiltinSig::Ptr,
            BuiltinSig::Bool,
        ],
    },
    BuiltinDecl {
        name: "__process_create",
        ret: BuiltinSig::Ptr,
        params: &[
            BuiltinSig::String,
            BuiltinSig::String,
            BuiltinSig::Ptr,
            BuiltinSig::Bool,
        ],
    },
    BuiltinDecl {
        name: "__process_wait",
        ret: BuiltinSig::Int,
        params: &[BuiltinSig::Ptr],
    },
    BuiltinDecl {
        name: "__process_wait_async",
        ret: BuiltinSig::Ptr,
        params: &[BuiltinSig::Ptr],
    },
    BuiltinDecl {
        name: "__process_kill",
        ret: BuiltinSig::Void,
        params: &[BuiltinSig::Ptr, BuiltinSig::Int],
    },
    // I/O pipes (4)
    BuiltinDecl {
        name: "__process_write_input",
        ret: BuiltinSig::Void,
        params: &[BuiltinSig::Ptr, BuiltinSig::String],
    },
    BuiltinDecl {
        name: "__process_close_input",
        ret: BuiltinSig::Void,
        params: &[BuiltinSig::Ptr],
    },
    BuiltinDecl {
        name: "__process_read_output",
        ret: BuiltinSig::String,
        params: &[BuiltinSig::Ptr],
    },
    BuiltinDecl {
        name: "__process_read_error",
        ret: BuiltinSig::String,
        params: &[BuiltinSig::Ptr],
    },
    // Environment (3)
    BuiltinDecl {
        name: "__process_get_env",
        ret: BuiltinSig::String,
        params: &[BuiltinSig::String],
    },
    BuiltinDecl {
        name: "__process_set_env",
        ret: BuiltinSig::Void,
        params: &[BuiltinSig::String, BuiltinSig::String],
    },
    BuiltinDecl {
        name: "__process_get_all_env",
        ret: BuiltinSig::Ptr,
        params: &[],
    },
    // System info (6)
    BuiltinDecl {
        name: "__process_get_pid",
        ret: BuiltinSig::Int,
        params: &[],
    },
    BuiltinDecl {
        name: "__process_get_ppid",
        ret: BuiltinSig::Int,
        params: &[],
    },
    BuiltinDecl {
        name: "__process_get_platform",
        ret: BuiltinSig::String,
        params: &[],
    },
    BuiltinDecl {
        name: "__process_get_cpu_count",
        ret: BuiltinSig::Int,
        params: &[],
    },
    BuiltinDecl {
        name: "__process_get_total_memory",
        ret: BuiltinSig::Int,
        params: &[],
    },
    BuiltinDecl {
        name: "__process_get_free_memory",
        ret: BuiltinSig::Int,
        params: &[],
    },
    // Signal (1)
    BuiltinDecl {
        name: "__process_signal_available",
        ret: BuiltinSig::Bool,
        params: &[BuiltinSig::Int],
    },
];

/// Resolve a `BuiltinSig` to its inkwell `BasicTypeEnum` representation.
///
/// `BuiltinSig::Void` is only ever valid as a *return* type, never as a
/// parameter. Callers that may hit `Void` should branch on it before
/// calling this function (as `declare_builtin_from_table` does).
pub fn sig_to_basic_type<'ctx>(
    context: &'ctx Context,
    sig: BuiltinSig,
) -> inkwell::types::BasicTypeEnum<'ctx> {
    match sig {
        BuiltinSig::Void => unreachable!("Void is not a BasicType; handle it at the call site"),
        BuiltinSig::Int => context.i64_type().as_basic_type_enum(),
        BuiltinSig::Float => context.f64_type().as_basic_type_enum(),
        BuiltinSig::Bool => context.bool_type().as_basic_type_enum(),
        BuiltinSig::String | BuiltinSig::Ptr => context
            .i8_type()
            .ptr_type(inkwell::AddressSpace::default())
            .as_basic_type_enum(),
    }
}

/// Convert each `BuiltinSig` parameter to its inkwell metadata-type enum,
/// matching the ordering expected by `FunctionType::new`.
pub fn params_to_metadata<'ctx>(
    context: &'ctx Context,
    params: &'static [BuiltinSig],
) -> Vec<BasicMetadataTypeEnum<'ctx>> {
    params
        .iter()
        .map(|s| BasicMetadataTypeEnum::from(sig_to_basic_type(context, *s)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtins_count_is_101() {
        assert_eq!(
            BUILTINS.len(),
            101,
            "expected exactly 101 FFI entries (56 base + path 8 + io 17 + process 20)"
        );
    }

    #[test]
    fn builtins_names_are_unique() {
        let mut names: Vec<&'static str> = BUILTINS.iter().map(|d| d.name).collect();
        names.sort();
        let before = names.len();
        names.dedup();
        assert_eq!(names.len(), before, "duplicate FFI name in BUILTINS table");
    }
}
