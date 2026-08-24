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
 * Order: array (6) → map (7) → set (4) → string (18) → math (28) → time (4) → json (2) → path (8) → io (17) → process (20).
 *
 * @author Ruyi Team
 * @date 2026-07-12
 */
use inkwell::context::Context;
use inkwell::types::{BasicMetadataTypeEnum, BasicType};

use crate::typechecker::types::Type;

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
    /// `i8` — unsigned byte.
    Byte,
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

/// All 115 FFI entries known to the compiler.
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
        name: "__string_equals",
        ret: BuiltinSig::Bool,
        params: &[BuiltinSig::Ptr, BuiltinSig::Ptr],
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
        name: "__string_to_string",
        ret: BuiltinSig::Ptr,
        params: &[BuiltinSig::Ptr],
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
    // __math_* (28)
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
    // Inverse trigonometric
    BuiltinDecl {
        name: "__math_acos",
        ret: BuiltinSig::Float,
        params: &[BuiltinSig::Float],
    },
    BuiltinDecl {
        name: "__math_asin",
        ret: BuiltinSig::Float,
        params: &[BuiltinSig::Float],
    },
    BuiltinDecl {
        name: "__math_atan",
        ret: BuiltinSig::Float,
        params: &[BuiltinSig::Float],
    },
    BuiltinDecl {
        name: "__math_atan2",
        ret: BuiltinSig::Float,
        params: &[BuiltinSig::Float, BuiltinSig::Float],
    },
    // Logarithmic and exponential
    BuiltinDecl {
        name: "__math_log2",
        ret: BuiltinSig::Float,
        params: &[BuiltinSig::Float],
    },
    BuiltinDecl {
        name: "__math_log10",
        ret: BuiltinSig::Float,
        params: &[BuiltinSig::Float],
    },
    BuiltinDecl {
        name: "__math_exp",
        ret: BuiltinSig::Float,
        params: &[BuiltinSig::Float],
    },
    // Sign and truncation
    BuiltinDecl {
        name: "__math_sign",
        ret: BuiltinSig::Float,
        params: &[BuiltinSig::Float],
    },
    BuiltinDecl {
        name: "__math_trunc",
        ret: BuiltinSig::Float,
        params: &[BuiltinSig::Float],
    },
    // Hyperbolic
    BuiltinDecl {
        name: "__math_sinh",
        ret: BuiltinSig::Float,
        params: &[BuiltinSig::Float],
    },
    BuiltinDecl {
        name: "__math_cosh",
        ret: BuiltinSig::Float,
        params: &[BuiltinSig::Float],
    },
    BuiltinDecl {
        name: "__math_tanh",
        ret: BuiltinSig::Float,
        params: &[BuiltinSig::Float],
    },
    // Miscellaneous
    BuiltinDecl {
        name: "__math_hypot",
        ret: BuiltinSig::Float,
        params: &[BuiltinSig::Float, BuiltinSig::Float],
    },
    BuiltinDecl {
        name: "__math_cbrt",
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
    // __io_* (24)
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
    // __io_* (6) — extended
    BuiltinDecl {
        name: "__io_read_dir",
        ret: BuiltinSig::Ptr,
        params: &[BuiltinSig::String],
    },
    BuiltinDecl {
        name: "__io_file_size",
        ret: BuiltinSig::Int,
        params: &[BuiltinSig::String],
    },
    BuiltinDecl {
        name: "__io_file_mtime",
        ret: BuiltinSig::Int,
        params: &[BuiltinSig::String],
    },
    BuiltinDecl {
        name: "__io_rename",
        ret: BuiltinSig::Bool,
        params: &[BuiltinSig::String, BuiltinSig::String],
    },
    BuiltinDecl {
        name: "__io_remove_dir",
        ret: BuiltinSig::Bool,
        params: &[BuiltinSig::String, BuiltinSig::Bool],
    },
    BuiltinDecl {
        name: "__io_file_append_text",
        ret: BuiltinSig::Void,
        params: &[BuiltinSig::String, BuiltinSig::String],
    },
    // __io_read_random (1)
    BuiltinDecl {
        name: "__io_read_random",
        ret: BuiltinSig::Ptr,
        params: &[BuiltinSig::Int],
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
    // ============================================================
    // __net_tcp_* (9)
    // ============================================================
    BuiltinDecl {
        name: "__net_tcp_connect",
        ret: BuiltinSig::Int,
        params: &[BuiltinSig::String, BuiltinSig::Int],
    },
    BuiltinDecl {
        name: "__net_tcp_read",
        ret: BuiltinSig::String,
        params: &[BuiltinSig::Int, BuiltinSig::Int],
    },
    BuiltinDecl {
        name: "__net_tcp_write",
        ret: BuiltinSig::Int,
        params: &[BuiltinSig::Int, BuiltinSig::String],
    },
    BuiltinDecl {
        name: "__net_tcp_write_raw",
        ret: BuiltinSig::Int,
        params: &[BuiltinSig::Int, BuiltinSig::Ptr],
    },
    BuiltinDecl {
        name: "__net_tcp_read_raw",
        ret: BuiltinSig::Int,
        params: &[BuiltinSig::Int, BuiltinSig::Ptr],
    },
    BuiltinDecl {
        name: "__net_tcp_close",
        ret: BuiltinSig::Void,
        params: &[BuiltinSig::Int],
    },
    BuiltinDecl {
        name: "__net_tcp_set_timeout",
        ret: BuiltinSig::Int,
        params: &[BuiltinSig::Int, BuiltinSig::Int],
    },
    BuiltinDecl {
        name: "__net_tcp_listen",
        ret: BuiltinSig::Int,
        params: &[BuiltinSig::String, BuiltinSig::Int],
    },
    BuiltinDecl {
        name: "__net_tcp_accept",
        ret: BuiltinSig::Int,
        params: &[BuiltinSig::Int],
    },
    BuiltinDecl {
        name: "__net_tcp_server_close",
        ret: BuiltinSig::Void,
        params: &[BuiltinSig::Int],
    },
    // __net_udp_* (7)
    BuiltinDecl {
        name: "__net_udp_socket",
        ret: BuiltinSig::Int,
        params: &[],
    },
    BuiltinDecl {
        name: "__net_udp_bind",
        ret: BuiltinSig::Int,
        params: &[BuiltinSig::Int, BuiltinSig::String, BuiltinSig::Int],
    },
    BuiltinDecl {
        name: "__net_udp_send_to",
        ret: BuiltinSig::Int,
        params: &[
            BuiltinSig::Int,
            BuiltinSig::String,
            BuiltinSig::Int,
            BuiltinSig::String,
        ],
    },
    BuiltinDecl {
        name: "__net_udp_recv_from",
        ret: BuiltinSig::String,
        params: &[BuiltinSig::Int, BuiltinSig::Int],
    },
    BuiltinDecl {
        name: "__net_udp_sender_host",
        ret: BuiltinSig::String,
        params: &[BuiltinSig::Int],
    },
    BuiltinDecl {
        name: "__net_udp_sender_port",
        ret: BuiltinSig::Int,
        params: &[BuiltinSig::Int],
    },
    BuiltinDecl {
        name: "__net_udp_close",
        ret: BuiltinSig::Void,
        params: &[BuiltinSig::Int],
    },
    // ============================================================
    // __random_* (5) — added for stdlib/random.ry and stdlib/uuid.ry
    // ============================================================
    BuiltinDecl {
        name: "__random_new",
        ret: BuiltinSig::Int,
        params: &[BuiltinSig::Int],
    },
    BuiltinDecl {
        name: "__random_int",
        ret: BuiltinSig::Int,
        params: &[BuiltinSig::Int, BuiltinSig::Int, BuiltinSig::Int],
    },
    BuiltinDecl {
        name: "__random_float",
        ret: BuiltinSig::Float,
        params: &[BuiltinSig::Int],
    },
    BuiltinDecl {
        name: "__random_bool",
        ret: BuiltinSig::Bool,
        params: &[BuiltinSig::Int],
    },
    BuiltinDecl {
        name: "__random_bytes",
        ret: BuiltinSig::Ptr,
        params: &[BuiltinSig::Int, BuiltinSig::Int],
    },
    // ============================================================
    // float_ffi (2) — f64 bit-level conversion
    // ============================================================
    BuiltinDecl {
        name: "__f64_to_bits",
        ret: BuiltinSig::Int,
        params: &[BuiltinSig::Float],
    },
    BuiltinDecl {
        name: "__f64_from_bits",
        ret: BuiltinSig::Float,
        params: &[BuiltinSig::Int],
    },
    // ============================================================
    // atomic_ffi (8) — thread-safe AtomicI64
    // ============================================================
    BuiltinDecl {
        name: "__atomic_i64_new",
        ret: BuiltinSig::Ptr,
        params: &[BuiltinSig::Int],
    },
    BuiltinDecl {
        name: "__atomic_i64_load",
        ret: BuiltinSig::Int,
        params: &[BuiltinSig::Ptr],
    },
    BuiltinDecl {
        name: "__atomic_i64_store",
        ret: BuiltinSig::Void,
        params: &[BuiltinSig::Ptr, BuiltinSig::Int],
    },
    BuiltinDecl {
        name: "__atomic_i64_cas",
        ret: BuiltinSig::Int,
        params: &[BuiltinSig::Ptr, BuiltinSig::Int, BuiltinSig::Int],
    },
    BuiltinDecl {
        name: "__atomic_i64_fetch_add",
        ret: BuiltinSig::Int,
        params: &[BuiltinSig::Ptr, BuiltinSig::Int],
    },
    BuiltinDecl {
        name: "__atomic_i64_fetch_sub",
        ret: BuiltinSig::Int,
        params: &[BuiltinSig::Ptr, BuiltinSig::Int],
    },
    BuiltinDecl {
        name: "__atomic_i64_swap",
        ret: BuiltinSig::Int,
        params: &[BuiltinSig::Ptr, BuiltinSig::Int],
    },
    BuiltinDecl {
        name: "__atomic_i64_free",
        ret: BuiltinSig::Void,
        params: &[BuiltinSig::Ptr],
    },
    // ============================================================
    // mutex_ffi (5) — std::sync::Mutex<()>
    // ============================================================
    BuiltinDecl {
        name: "__mutex_new",
        ret: BuiltinSig::Ptr,
        params: &[],
    },
    BuiltinDecl {
        name: "__mutex_lock",
        ret: BuiltinSig::Ptr,
        params: &[BuiltinSig::Ptr],
    },
    BuiltinDecl {
        name: "__mutex_unlock",
        ret: BuiltinSig::Void,
        params: &[BuiltinSig::Ptr],
    },
    BuiltinDecl {
        name: "__mutex_try_lock",
        ret: BuiltinSig::Ptr,
        params: &[BuiltinSig::Ptr],
    },
    BuiltinDecl {
        name: "__mutex_free",
        ret: BuiltinSig::Void,
        params: &[BuiltinSig::Ptr],
    },
    // ============================================================
    // crypto_ffi (14) — SHA/HMAC/AES-GCM/X25519
    // ============================================================
    BuiltinDecl {
        name: "__crypto_sha256",
        ret: BuiltinSig::String,
        params: &[BuiltinSig::String],
    },
    BuiltinDecl {
        name: "__crypto_sha512",
        ret: BuiltinSig::String,
        params: &[BuiltinSig::String],
    },
    BuiltinDecl {
        name: "__crypto_sha1",
        ret: BuiltinSig::String,
        params: &[BuiltinSig::String],
    },
    BuiltinDecl {
        name: "__crypto_hmac_sha256",
        ret: BuiltinSig::String,
        params: &[BuiltinSig::String, BuiltinSig::String],
    },
    BuiltinDecl {
        name: "__crypto_aes_gcm_encrypt_hex",
        ret: BuiltinSig::String,
        params: &[BuiltinSig::String, BuiltinSig::String, BuiltinSig::String],
    },
    BuiltinDecl {
        name: "__crypto_aes_gcm_decrypt_hex",
        ret: BuiltinSig::String,
        params: &[BuiltinSig::String, BuiltinSig::String, BuiltinSig::String],
    },
    BuiltinDecl {
        name: "__crypto_x25519_keypair_hex",
        ret: BuiltinSig::String,
        params: &[],
    },
    BuiltinDecl {
        name: "__crypto_x25519_dh_hex",
        ret: BuiltinSig::String,
        params: &[BuiltinSig::String, BuiltinSig::String],
    },
    BuiltinDecl {
        name: "__crypto_x25519_pubkey_hex",
        ret: BuiltinSig::String,
        params: &[BuiltinSig::String],
    },
    BuiltinDecl {
        name: "__crypto_aes_gcm_encrypt_raw",
        ret: BuiltinSig::Int,
        params: &[
            BuiltinSig::Ptr,
            BuiltinSig::Ptr,
            BuiltinSig::Ptr,
            BuiltinSig::Int,
            BuiltinSig::Ptr,
            BuiltinSig::Ptr,
        ],
    },
    BuiltinDecl {
        name: "__crypto_aes_gcm_decrypt_raw",
        ret: BuiltinSig::Int,
        params: &[
            BuiltinSig::Ptr,
            BuiltinSig::Ptr,
            BuiltinSig::Ptr,
            BuiltinSig::Int,
            BuiltinSig::Ptr,
            BuiltinSig::Ptr,
        ],
    },
    BuiltinDecl {
        name: "__crypto_x25519_keypair_raw",
        ret: BuiltinSig::Void,
        params: &[BuiltinSig::Ptr, BuiltinSig::Ptr],
    },
    BuiltinDecl {
        name: "__crypto_x25519_dh_raw",
        ret: BuiltinSig::Void,
        params: &[BuiltinSig::Ptr, BuiltinSig::Ptr, BuiltinSig::Ptr],
    },
    BuiltinDecl {
        name: "__crypto_x25519_pubkey_raw",
        ret: BuiltinSig::Void,
        params: &[BuiltinSig::Ptr, BuiltinSig::Ptr],
    },
    // ============================================================
    // tls_ffi (5) — rustls TLS sessions
    // ============================================================
    BuiltinDecl {
        name: "__tls_connect",
        ret: BuiltinSig::Ptr,
        params: &[BuiltinSig::Int, BuiltinSig::String],
    },
    BuiltinDecl {
        name: "__tls_read_cstr",
        ret: BuiltinSig::String,
        params: &[BuiltinSig::Ptr, BuiltinSig::Int],
    },
    BuiltinDecl {
        name: "__tls_write",
        ret: BuiltinSig::Int,
        params: &[BuiltinSig::Ptr, BuiltinSig::String],
    },
    BuiltinDecl {
        name: "__tls_write_raw",
        ret: BuiltinSig::Int,
        params: &[BuiltinSig::Ptr, BuiltinSig::Ptr, BuiltinSig::Int],
    },
    BuiltinDecl {
        name: "__tls_read_raw",
        ret: BuiltinSig::Int,
        params: &[BuiltinSig::Ptr, BuiltinSig::Ptr],
    },
    BuiltinDecl {
        name: "__tls_close",
        ret: BuiltinSig::Void,
        params: &[BuiltinSig::Ptr],
    },
    BuiltinDecl {
        name: "__tls_free",
        ret: BuiltinSig::Void,
        params: &[BuiltinSig::Ptr],
    },
    // ============================================================
    // tls server (7) — rustls ServerConnection
    // ============================================================
    BuiltinDecl {
        name: "__tls_server_config_new",
        ret: BuiltinSig::Ptr,
        params: &[BuiltinSig::String, BuiltinSig::String],
    },
    BuiltinDecl {
        name: "__tls_server_accept",
        ret: BuiltinSig::Ptr,
        params: &[BuiltinSig::Ptr, BuiltinSig::Int],
    },
    BuiltinDecl {
        name: "__tls_server_read_cstr",
        ret: BuiltinSig::String,
        params: &[BuiltinSig::Ptr, BuiltinSig::Int],
    },
    BuiltinDecl {
        name: "__tls_server_write",
        ret: BuiltinSig::Int,
        params: &[BuiltinSig::Ptr, BuiltinSig::String],
    },
    BuiltinDecl {
        name: "__tls_server_write_raw",
        ret: BuiltinSig::Int,
        params: &[BuiltinSig::Ptr, BuiltinSig::Ptr, BuiltinSig::Int],
    },
    BuiltinDecl {
        name: "__tls_server_read_raw",
        ret: BuiltinSig::Int,
        params: &[BuiltinSig::Ptr, BuiltinSig::Ptr],
    },
    BuiltinDecl {
        name: "__tls_server_close",
        ret: BuiltinSig::Void,
        params: &[BuiltinSig::Ptr],
    },
    BuiltinDecl {
        name: "__tls_server_free",
        ret: BuiltinSig::Void,
        params: &[BuiltinSig::Ptr],
    },
    BuiltinDecl {
        name: "__tls_config_free",
        ret: BuiltinSig::Void,
        params: &[BuiltinSig::Ptr],
    },
    // ============================================================
    // compress_ffi (6) — gzip / zlib / raw deflate (base64 bridge)
    // ============================================================
    BuiltinDecl {
        name: "__compress_gzip",
        ret: BuiltinSig::String,
        params: &[BuiltinSig::String],
    },
    BuiltinDecl {
        name: "__decompress_gzip",
        ret: BuiltinSig::String,
        params: &[BuiltinSig::String],
    },
    BuiltinDecl {
        name: "__compress_zlib",
        ret: BuiltinSig::String,
        params: &[BuiltinSig::String],
    },
    BuiltinDecl {
        name: "__decompress_zlib",
        ret: BuiltinSig::String,
        params: &[BuiltinSig::String],
    },
    BuiltinDecl {
        name: "__compress_deflate",
        ret: BuiltinSig::String,
        params: &[BuiltinSig::String],
    },
    BuiltinDecl {
        name: "__decompress_deflate",
        ret: BuiltinSig::String,
        params: &[BuiltinSig::String],
    },
    // ============================================================
    // compress streaming (6) — new / write / finish × 2
    // ============================================================
    BuiltinDecl {
        name: "__compress_new",
        ret: BuiltinSig::Ptr,
        params: &[BuiltinSig::Int],
    },
    BuiltinDecl {
        name: "__compress_write",
        ret: BuiltinSig::String,
        params: &[BuiltinSig::Ptr, BuiltinSig::String],
    },
    BuiltinDecl {
        name: "__compress_finish",
        ret: BuiltinSig::String,
        params: &[BuiltinSig::Ptr],
    },
    BuiltinDecl {
        name: "__decompress_new",
        ret: BuiltinSig::Ptr,
        params: &[BuiltinSig::Int],
    },
    BuiltinDecl {
        name: "__decompress_write",
        ret: BuiltinSig::String,
        params: &[BuiltinSig::Ptr, BuiltinSig::String],
    },
    BuiltinDecl {
        name: "__decompress_finish",
        ret: BuiltinSig::String,
        params: &[BuiltinSig::Ptr],
    },
    // ============================================================
    // file streaming (8) — open / close / read_raw / write_raw / seek / tell / flush
    // ============================================================
    BuiltinDecl {
        name: "__fs_open_read",
        ret: BuiltinSig::Int,
        params: &[BuiltinSig::String],
    },
    BuiltinDecl {
        name: "__fs_open_write",
        ret: BuiltinSig::Int,
        params: &[BuiltinSig::String],
    },
    BuiltinDecl {
        name: "__fs_open_append",
        ret: BuiltinSig::Int,
        params: &[BuiltinSig::String],
    },
    BuiltinDecl {
        name: "__fs_close",
        ret: BuiltinSig::Void,
        params: &[BuiltinSig::Int],
    },
    BuiltinDecl {
        name: "__fs_read_raw",
        ret: BuiltinSig::Int,
        params: &[BuiltinSig::Int, BuiltinSig::Ptr],
    },
    BuiltinDecl {
        name: "__fs_write_raw",
        ret: BuiltinSig::Int,
        params: &[BuiltinSig::Int, BuiltinSig::Ptr],
    },
    BuiltinDecl {
        name: "__fs_seek",
        ret: BuiltinSig::Int,
        params: &[BuiltinSig::Int, BuiltinSig::Int, BuiltinSig::Int],
    },
    BuiltinDecl {
        name: "__fs_tell",
        ret: BuiltinSig::Int,
        params: &[BuiltinSig::Int],
    },
    // ============================================================
    // stdio streaming (3) — stdin_read / stdout_write / flush
    // ============================================================
    BuiltinDecl {
        name: "__io_read_raw",
        ret: BuiltinSig::Int,
        params: &[BuiltinSig::Ptr],
    },
    BuiltinDecl {
        name: "__io_write_raw",
        ret: BuiltinSig::Int,
        params: &[BuiltinSig::Ptr],
    },
    BuiltinDecl {
        name: "__io_write_stderr_raw",
        ret: BuiltinSig::Int,
        params: &[BuiltinSig::Ptr],
    },
    BuiltinDecl {
        name: "__io_flush",
        ret: BuiltinSig::Int,
        params: &[],
    },
    // ============================================================
    // channel_ffi (14) — bounded/unbounded MPSC + select
    // ============================================================
    BuiltinDecl {
        name: "__channel_new",
        ret: BuiltinSig::Ptr,
        params: &[BuiltinSig::Int],
    },
    BuiltinDecl {
        name: "__channel_send",
        ret: BuiltinSig::Int,
        params: &[BuiltinSig::Ptr, BuiltinSig::Int],
    },
    BuiltinDecl {
        name: "__channel_try_send",
        ret: BuiltinSig::Int,
        params: &[BuiltinSig::Ptr, BuiltinSig::Int],
    },
    BuiltinDecl {
        name: "__channel_recv",
        ret: BuiltinSig::Int,
        params: &[BuiltinSig::Ptr],
    },
    BuiltinDecl {
        name: "__channel_recv_timeout",
        ret: BuiltinSig::Int,
        params: &[BuiltinSig::Ptr, BuiltinSig::Int],
    },
    BuiltinDecl {
        name: "__channel_try_recv",
        ret: BuiltinSig::Int,
        params: &[BuiltinSig::Ptr],
    },
    BuiltinDecl {
        name: "__channel_is_closed",
        ret: BuiltinSig::Bool,
        params: &[BuiltinSig::Ptr],
    },
    BuiltinDecl {
        name: "__channel_clone",
        ret: BuiltinSig::Ptr,
        params: &[BuiltinSig::Ptr],
    },
    BuiltinDecl {
        name: "__channel_clone_send",
        ret: BuiltinSig::Int,
        params: &[BuiltinSig::Ptr, BuiltinSig::Int],
    },
    BuiltinDecl {
        name: "__channel_clone_free",
        ret: BuiltinSig::Void,
        params: &[BuiltinSig::Ptr],
    },
    BuiltinDecl {
        name: "__channel_free",
        ret: BuiltinSig::Void,
        params: &[BuiltinSig::Ptr],
    },
    BuiltinDecl {
        name: "__channel_select_new",
        ret: BuiltinSig::Ptr,
        params: &[],
    },
    BuiltinDecl {
        name: "__channel_select_add",
        ret: BuiltinSig::Int,
        params: &[BuiltinSig::Ptr, BuiltinSig::Ptr],
    },
    BuiltinDecl {
        name: "__channel_select_wait",
        ret: BuiltinSig::Int,
        params: &[BuiltinSig::Ptr],
    },
    BuiltinDecl {
        name: "__channel_select_free",
        ret: BuiltinSig::Void,
        params: &[BuiltinSig::Ptr],
    },
    // ============================================================
    // rwlock_ffi (8) — read-write lock
    // ============================================================
    BuiltinDecl {
        name: "__rwlock_new",
        ret: BuiltinSig::Ptr,
        params: &[],
    },
    BuiltinDecl {
        name: "__rwlock_read_lock",
        ret: BuiltinSig::Ptr,
        params: &[BuiltinSig::Ptr],
    },
    BuiltinDecl {
        name: "__rwlock_try_read_lock",
        ret: BuiltinSig::Ptr,
        params: &[BuiltinSig::Ptr],
    },
    BuiltinDecl {
        name: "__rwlock_read_unlock",
        ret: BuiltinSig::Void,
        params: &[BuiltinSig::Ptr],
    },
    BuiltinDecl {
        name: "__rwlock_write_lock",
        ret: BuiltinSig::Ptr,
        params: &[BuiltinSig::Ptr],
    },
    BuiltinDecl {
        name: "__rwlock_try_write_lock",
        ret: BuiltinSig::Ptr,
        params: &[BuiltinSig::Ptr],
    },
    BuiltinDecl {
        name: "__rwlock_write_unlock",
        ret: BuiltinSig::Void,
        params: &[BuiltinSig::Ptr],
    },
    BuiltinDecl {
        name: "__rwlock_free",
        ret: BuiltinSig::Void,
        params: &[BuiltinSig::Ptr],
    },
    // ============================================================
    // thread_ffi (6) — OS thread spawning and management
    // ============================================================
    BuiltinDecl {
        name: "__thread_spawn",
        ret: BuiltinSig::Int,
        params: &[BuiltinSig::Ptr, BuiltinSig::Ptr],
    },
    BuiltinDecl {
        name: "__thread_spawn_named",
        ret: BuiltinSig::Int,
        params: &[BuiltinSig::Ptr, BuiltinSig::Ptr, BuiltinSig::Ptr],
    },
    BuiltinDecl {
        name: "__thread_join",
        ret: BuiltinSig::Int,
        params: &[BuiltinSig::Int],
    },
    BuiltinDecl {
        name: "__thread_join_timeout",
        ret: BuiltinSig::Int,
        params: &[BuiltinSig::Int, BuiltinSig::Int],
    },
    BuiltinDecl {
        name: "__thread_is_finished",
        ret: BuiltinSig::Int,
        params: &[BuiltinSig::Int],
    },
    BuiltinDecl {
        name: "__thread_detach",
        ret: BuiltinSig::Int,
        params: &[BuiltinSig::Int],
    },
    BuiltinDecl {
        name: "__thread_id",
        ret: BuiltinSig::Int,
        params: &[],
    },
    BuiltinDecl {
        name: "__thread_cpu_count",
        ret: BuiltinSig::Int,
        params: &[],
    },
    BuiltinDecl {
        name: "__thread_sleep",
        ret: BuiltinSig::Void,
        params: &[BuiltinSig::Int],
    },
    // ============================================================
    // fiber_ffi (7) — lightweight fiber (纎程) concurrency
    // ============================================================
    BuiltinDecl {
        name: "__fiber_spawn",
        ret: BuiltinSig::Int,
        params: &[BuiltinSig::Ptr, BuiltinSig::Ptr],
    },
    BuiltinDecl {
        name: "__fiber_join",
        ret: BuiltinSig::Int,
        params: &[BuiltinSig::Int],
    },
    BuiltinDecl {
        name: "__fiber_is_finished",
        ret: BuiltinSig::Int,
        params: &[BuiltinSig::Int],
    },
    BuiltinDecl {
        name: "__fiber_detach",
        ret: BuiltinSig::Int,
        params: &[BuiltinSig::Int],
    },
    BuiltinDecl {
        name: "__fiber_id",
        ret: BuiltinSig::Int,
        params: &[],
    },
    BuiltinDecl {
        name: "__fiber_sleep",
        ret: BuiltinSig::Void,
        params: &[BuiltinSig::Int],
    },
    BuiltinDecl {
        name: "__fiber_yield",
        ret: BuiltinSig::Void,
        params: &[],
    },
    // ============================================================
    // spawn_blocking (3) — offload blocking work to thread pool
    // ============================================================
    BuiltinDecl {
        name: "ruyi_spawn_blocking",
        ret: BuiltinSig::Ptr,
        params: &[BuiltinSig::Ptr, BuiltinSig::Int],
    },
    BuiltinDecl {
        name: "ruyi_spawn_blocking_poll",
        ret: BuiltinSig::Int,
        params: &[BuiltinSig::Ptr, BuiltinSig::Ptr],
    },
    BuiltinDecl {
        name: "ruyi_spawn_blocking_free",
        ret: BuiltinSig::Void,
        params: &[BuiltinSig::Ptr],
    },
    // ============================================================
    // tls_store_ffi (5) — per-thread key-value storage
    // ============================================================
    BuiltinDecl {
        name: "__tls_store",
        ret: BuiltinSig::Int,
        params: &[BuiltinSig::Int, BuiltinSig::Int],
    },
    BuiltinDecl {
        name: "__tls_load",
        ret: BuiltinSig::Int,
        params: &[BuiltinSig::Int],
    },
    BuiltinDecl {
        name: "__tls_remove",
        ret: BuiltinSig::Int,
        params: &[BuiltinSig::Int],
    },
    BuiltinDecl {
        name: "__tls_contains",
        ret: BuiltinSig::Bool,
        params: &[BuiltinSig::Int],
    },
    BuiltinDecl {
        name: "__tls_clear",
        ret: BuiltinSig::Void,
        params: &[],
    },
    // ============================================================
    // barrier_ffi (3) — thread synchronization barrier
    // ============================================================
    BuiltinDecl {
        name: "__barrier_new",
        ret: BuiltinSig::Ptr,
        params: &[BuiltinSig::Int],
    },
    BuiltinDecl {
        name: "__barrier_wait",
        ret: BuiltinSig::Int,
        params: &[BuiltinSig::Ptr],
    },
    BuiltinDecl {
        name: "__barrier_free",
        ret: BuiltinSig::Void,
        params: &[BuiltinSig::Ptr],
    },
    // ============================================================
    // once_ffi (5) — one-time initialisation guard
    // ============================================================
    BuiltinDecl {
        name: "__once_new",
        ret: BuiltinSig::Ptr,
        params: &[],
    },
    BuiltinDecl {
        name: "__once_do",
        ret: BuiltinSig::Int,
        params: &[BuiltinSig::Ptr, BuiltinSig::Ptr, BuiltinSig::Int],
    },
    BuiltinDecl {
        name: "__once_is_completed",
        ret: BuiltinSig::Int,
        params: &[BuiltinSig::Ptr],
    },
    BuiltinDecl {
        name: "__once_reset",
        ret: BuiltinSig::Void,
        params: &[BuiltinSig::Ptr],
    },
    BuiltinDecl {
        name: "__once_free",
        ret: BuiltinSig::Void,
        params: &[BuiltinSig::Ptr],
    },
    // ============================================================
    // semaphore_ffi (6) — counting semaphore
    // ============================================================
    BuiltinDecl {
        name: "__semaphore_new",
        ret: BuiltinSig::Ptr,
        params: &[BuiltinSig::Int],
    },
    BuiltinDecl {
        name: "__semaphore_acquire",
        ret: BuiltinSig::Void,
        params: &[BuiltinSig::Ptr],
    },
    BuiltinDecl {
        name: "__semaphore_try_acquire",
        ret: BuiltinSig::Int,
        params: &[BuiltinSig::Ptr],
    },
    BuiltinDecl {
        name: "__semaphore_release",
        ret: BuiltinSig::Void,
        params: &[BuiltinSig::Ptr],
    },
    BuiltinDecl {
        name: "__semaphore_available",
        ret: BuiltinSig::Int,
        params: &[BuiltinSig::Ptr],
    },
    BuiltinDecl {
        name: "__semaphore_free",
        ret: BuiltinSig::Void,
        params: &[BuiltinSig::Ptr],
    },
    // ============================================================
    // condvar_ffi (5) — condition variable
    // ============================================================
    BuiltinDecl {
        name: "__condvar_new",
        ret: BuiltinSig::Ptr,
        params: &[],
    },
    BuiltinDecl {
        name: "__condvar_wait",
        ret: BuiltinSig::Ptr,
        params: &[BuiltinSig::Ptr, BuiltinSig::Ptr],
    },
    BuiltinDecl {
        name: "__condvar_notify_one",
        ret: BuiltinSig::Void,
        params: &[BuiltinSig::Ptr],
    },
    BuiltinDecl {
        name: "__condvar_notify_all",
        ret: BuiltinSig::Void,
        params: &[BuiltinSig::Ptr],
    },
    BuiltinDecl {
        name: "__condvar_free",
        ret: BuiltinSig::Void,
        params: &[BuiltinSig::Ptr],
    },
    // ============================================================
    // __net_async_* (9) — Reactor-based async TCP I/O futures
    // ============================================================
    BuiltinDecl {
        name: "__net_async_read",
        ret: BuiltinSig::Ptr,
        params: &[BuiltinSig::Int, BuiltinSig::Int],
    },
    BuiltinDecl {
        name: "__net_async_read_result",
        ret: BuiltinSig::String,
        params: &[BuiltinSig::Ptr],
    },
    BuiltinDecl {
        name: "__net_async_read_free",
        ret: BuiltinSig::Void,
        params: &[BuiltinSig::Ptr],
    },
    BuiltinDecl {
        name: "__net_async_write",
        ret: BuiltinSig::Ptr,
        params: &[BuiltinSig::Ptr, BuiltinSig::String],
    },
    BuiltinDecl {
        name: "__net_async_write_result",
        ret: BuiltinSig::Int,
        params: &[BuiltinSig::Ptr],
    },
    BuiltinDecl {
        name: "__net_async_write_free",
        ret: BuiltinSig::Void,
        params: &[BuiltinSig::Ptr],
    },
    BuiltinDecl {
        name: "__net_async_accept",
        ret: BuiltinSig::Ptr,
        params: &[BuiltinSig::Int],
    },
    BuiltinDecl {
        name: "__net_async_accept_result",
        ret: BuiltinSig::Int,
        params: &[BuiltinSig::Ptr],
    },
    BuiltinDecl {
        name: "__net_async_accept_free",
        ret: BuiltinSig::Void,
        params: &[BuiltinSig::Ptr],
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
        BuiltinSig::Byte => context.i8_type().as_basic_type_enum(),
        BuiltinSig::String | BuiltinSig::Ptr => context
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

/// Look up the return signature of a builtin by its symbol name.
///
/// Returns `None` for names not present in the `BUILTINS` table (e.g.
/// user-defined functions), letting callers fall back to their default
/// return-type inference.
pub fn builtin_ret_sig(name: &str) -> Option<BuiltinSig> {
    BUILTINS.iter().find(|d| d.name == name).map(|d| d.ret)
}

/// Convert a `BuiltinSig` return signature into the compiler's `Type`.
///
/// Text pointers (`String`) map to `Type::String`. Opaque pointers (`Ptr`)
/// default to `Type::Dynamic`, but string builtins are refined: they return
/// UTF-8 text (`Type::String`), with `__string_split` as the exception which
/// produces an `Array<string>`. Keeping precise result types (e.g. `Int` for
/// `__string_char_code_at`) instead of collapsing every builtin call to
/// `Dynamic` lets codegen compile arithmetic and method chains on results.
pub fn sig_to_type(name: &str, sig: BuiltinSig) -> Type {
    match sig {
        BuiltinSig::Void => Type::Void,
        BuiltinSig::Int => Type::Int,
        BuiltinSig::Float => Type::Float,
        BuiltinSig::Bool => Type::Bool,
        BuiltinSig::Byte => Type::Byte,
        BuiltinSig::String => Type::String,
        BuiltinSig::Ptr => {
            if name == "__string_split" {
                Type::Array(Box::new(Type::String))
            } else if name.starts_with("__string_") {
                Type::String
            } else if name == "__io_read_random" {
                // Returns a null-terminated buffer of random bytes, i.e. a
                // string; callers index it with charCodeAt.
                Type::String
            } else {
                Type::Dynamic
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtins_count_is_289() {
        assert_eq!(
            BUILTINS.len(),
            289,
            "expected exactly 289 FFI entries (279 before + 9 net_async_ffi + 1 tls)"
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
