/**
 * Diagnostics module for Ruyi compiler.
 *
 * Provides error codes, rendering, and diagnostic types.
 *
 * @author Ruyi Team
 * @date 2026-05-02
 */
pub mod codes;
pub mod render;

pub use codes::*;
pub use render::*;
