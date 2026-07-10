//! LLVM landing-pad generation for Ruyi exception handling.
//!
//! This module is now a thin re-export of the shared
//! `ruyi_exception::landing_pad` implementation.

#[cfg(feature = "inkwell")]
pub use ruyi_exception::landing_pad::*;

#[cfg(feature = "inkwell")]
pub mod llvm {
    //! Backward-compatible `llvm` submodule alias.
    //!
    //! Preserves the historical import path
    //! `ruyi_runtime::exception::landing_pad::llvm::LandingPadGenerator`.

    pub use super::*;
}
