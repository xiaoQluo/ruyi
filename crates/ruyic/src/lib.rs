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
