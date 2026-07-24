#![allow(clippy::collapsible_match)]
#![allow(clippy::collapsible_if)]
#![allow(clippy::only_used_in_recursion)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::module_inception)]
#![allow(clippy::needless_range_loop)]
#![allow(clippy::type_complexity)]
#![allow(clippy::if_same_then_else)]
#![allow(clippy::match_like_matches_macro)]
#![allow(clippy::unnecessary_to_owned)]

pub mod cli;
pub mod codegen;
pub mod diagnostics;
pub mod driver;
pub mod lexer;
pub mod macro_expand;
pub mod parser;
pub mod runtime;
pub mod typechecker;

use crate::macro_expand::{expand_macros, MacroRegistry};
use crate::parser::Parser;

pub fn compile(source: &str) -> Result<crate::parser::ast::Program, String> {
    let mut parser = Parser::new(source).map_err(|e| e.to_string())?;
    let ast = parser.parse().map_err(|e| e.to_string())?;

    let mut registry = MacroRegistry::with_builtins();
    let expanded = expand_macros(&ast, &mut registry).map_err(|e| e.to_string())?;

    Ok(expanded)
}
