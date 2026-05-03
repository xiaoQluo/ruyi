//! Exception types and object layout for the Ruyi runtime.
//!
//! Defines the exception type hierarchy and the C-compatible object layout
//! used by the LLVM exception handling machinery.

use crate::exception::{builtin_type_ids, TypeId};

/// Built-in exception types in Ruyi.
///
/// All user-defined exceptions extend `Error`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ExceptionType {
    /// Base exception type.
    Error,
    /// Type mismatch or invalid operation.
    TypeError,
    /// Index or value out of range.
    RangeError,
    /// General runtime failure.
    RuntimeError,
}

impl ExceptionType {
    /// Return the compile-time type ID for this exception type.
    pub fn type_id(&self) -> TypeId {
        match self {
            ExceptionType::Error => builtin_type_ids::ERROR,
            ExceptionType::TypeError => builtin_type_ids::TYPE_ERROR,
            ExceptionType::RangeError => builtin_type_ids::RANGE_ERROR,
            ExceptionType::RuntimeError => builtin_type_ids::RUNTIME_ERROR,
        }
    }

    /// Return the human-readable name of this exception type.
    pub fn type_name(&self) -> &'static str {
        match self {
            ExceptionType::Error => "Error",
            ExceptionType::TypeError => "TypeError",
            ExceptionType::RangeError => "RangeError",
            ExceptionType::RuntimeError => "RuntimeError",
        }
    }

    /// Map a runtime type ID back to an `ExceptionType`.
    pub fn from_type_id(id: TypeId) -> Option<Self> {
        match id {
            builtin_type_ids::ERROR => Some(ExceptionType::Error),
            builtin_type_ids::TYPE_ERROR => Some(ExceptionType::TypeError),
            builtin_type_ids::RANGE_ERROR => Some(ExceptionType::RangeError),
            builtin_type_ids::RUNTIME_ERROR => Some(ExceptionType::RuntimeError),
            _ => None,
        }
    }

    /// Check whether `self` is the same as or a supertype of `other`.
    ///
    /// In Ruyi's flat built-in hierarchy, `Error` matches all built-in
    /// types; otherwise only exact matches succeed.
    pub fn matches(&self, thrown_type: ExceptionType) -> bool {
        *self == ExceptionType::Error || *self == thrown_type
    }
}

/// C-compatible layout of a Ruyi exception object.
///
/// This is the payload that follows the Itanium ABI `_Unwind_Exception`
/// header when an exception is thrown. The GC header is **not** part of
/// this struct; it is managed by the runtime allocator.
#[repr(C)]
pub struct ExceptionObject {
    /// Exception type tag used for catch filtering.
    pub type_tag: TypeId,
    /// Pointer to a null-terminated UTF-8 message string.
    pub message: *mut u8,
    /// Number of frames in the stack trace.
    pub stack_trace_len: usize,
    /// Pointer to an array of [`StackFrame`](crate::exception::StackFrame).
    pub stack_trace: *mut crate::exception::StackFrame,
}

impl ExceptionObject {
    /// Return the type ID of this exception.
    pub fn type_id(&self) -> TypeId {
        self.type_tag
    }

    /// Return a raw pointer to the message string.
    pub fn message_ptr(&self) -> *mut u8 {
        self.message
    }
}

/// Exception class magic number for Itanium ABI.
///
/// Ruyi exceptions are tagged with this class so that the personality
/// routine can distinguish them from C++ or other language exceptions.
pub const KLANG_EXCEPTION_CLASS: u64 = 0x4B6C616E67000000;

/// Header that precedes every thrown exception in the Itanium ABI.
///
/// In a full implementation this would mirror `_Unwind_Exception`.
/// The private words are opaque to the language runtime.
#[repr(C)]
pub struct UnwindException {
    pub exception_class: u64,
    pub exception_cleanup: Option<unsafe extern "C" fn(u64, *mut UnwindException)>,
    /// Private unwinder state (opaque to the language runtime).
    pub private: [u64; 6],
    /// Ruyi-specific payload follows this header.
    pub payload: ExceptionObject,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exception_type_ids() {
        assert_eq!(ExceptionType::Error.type_id(), builtin_type_ids::ERROR);
        assert_eq!(
            ExceptionType::TypeError.type_id(),
            builtin_type_ids::TYPE_ERROR
        );
        assert_eq!(
            ExceptionType::RangeError.type_id(),
            builtin_type_ids::RANGE_ERROR
        );
        assert_eq!(
            ExceptionType::RuntimeError.type_id(),
            builtin_type_ids::RUNTIME_ERROR
        );
    }

    #[test]
    fn test_exception_type_matching() {
        assert!(ExceptionType::Error.matches(ExceptionType::TypeError));
        assert!(ExceptionType::Error.matches(ExceptionType::Error));
        assert!(ExceptionType::TypeError.matches(ExceptionType::TypeError));
        assert!(!ExceptionType::TypeError.matches(ExceptionType::RangeError));
    }

    #[test]
    fn test_exception_type_roundtrip() {
        for ty in [
            ExceptionType::Error,
            ExceptionType::TypeError,
            ExceptionType::RangeError,
            ExceptionType::RuntimeError,
        ] {
            assert_eq!(ExceptionType::from_type_id(ty.type_id()), Some(ty));
        }
        assert_eq!(ExceptionType::from_type_id(9999), None);
    }

    #[test]
    fn test_exception_object_layout() {
        let exc = ExceptionObject {
            type_tag: ExceptionType::RuntimeError.type_id(),
            message: std::ptr::null_mut(),
            stack_trace_len: 0,
            stack_trace: std::ptr::null_mut(),
        };
        assert_eq!(exc.type_id(), ExceptionType::RuntimeError.type_id());
        assert!(exc.message_ptr().is_null());
    }
}
