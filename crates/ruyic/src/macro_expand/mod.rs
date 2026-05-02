pub mod expand;
pub mod pattern;
pub mod hygiene;
pub mod builtins;

use crate::parser::ast::{MacroRule, Program};
use crate::lexer::token::Token;
use std::collections::HashMap;

pub const MAX_EXPANSION_DEPTH: usize = 128;

#[derive(Debug, Clone, PartialEq)]
pub enum MacroError {
    NoMatchingRule {
        macro_name: String,
        location: String,
    },
    RepetitionMismatch {
        macro_name: String,
        pattern_var: String,
        location: String,
    },
    ExpansionDepthExceeded {
        macro_name: String,
        depth: usize,
    },
    InvalidInvocation {
        macro_name: String,
        message: String,
    },
    HygieneViolation {
        identifier: String,
        location: String,
    },
    NestedMacroDefinition {
        location: String,
    },
}

impl std::fmt::Display for MacroError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MacroError::NoMatchingRule { macro_name, location } => {
                write!(f, "no matching rule for macro '{}' at {}", macro_name, location)
            }
            MacroError::RepetitionMismatch { macro_name, pattern_var, location } => {
                write!(f, "repetition mismatch for '${}' in macro '{}' at {}", pattern_var, macro_name, location)
            }
            MacroError::ExpansionDepthExceeded { macro_name, depth } => {
                write!(f, "maximum expansion depth ({}) exceeded for macro '{}'", depth, macro_name)
            }
            MacroError::InvalidInvocation { macro_name, message } => {
                write!(f, "invalid macro invocation '{}': {}", macro_name, message)
            }
            MacroError::HygieneViolation { identifier, location } => {
                write!(f, "hygiene violation for identifier '{}' at {}", identifier, location)
            }
            MacroError::NestedMacroDefinition { location } => {
                write!(f, "nested macro definition encountered at {}", location)
            }
        }
    }
}

impl std::error::Error for MacroError {}

impl From<crate::parser::ParseError> for MacroError {
    fn from(err: crate::parser::ParseError) -> Self {
        MacroError::InvalidInvocation {
            macro_name: String::new(),
            message: err.to_string(),
        }
    }
}

pub type MacroResult<T> = Result<T, MacroError>;

#[derive(Debug, Clone)]
pub struct MacroRegistry {
    macros: HashMap<String, Vec<MacroRule>>,
    builtins: HashMap<String, BuiltinMacro>,
}

#[derive(Debug, Clone)]
pub struct BuiltinMacro {
    pub name: String,
    pub hygienic: bool,
    pub expand: fn(&[Token], &dyn hygiene::HygieneContext) -> MacroResult<Vec<Token>>,
}

impl MacroRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_builtins() -> Self {
        let mut registry = Self::new();
        builtins::register_builtins(&mut registry);
        registry
    }

    pub fn add_macro(&mut self, name: String, rules: Vec<MacroRule>) {
        self.macros.insert(name, rules);
    }

    pub fn get_macro(&self, name: &str) -> Option<&[MacroRule]> {
        self.macros.get(name).map(|v| v.as_slice())
    }

    pub fn contains(&self, name: &str) -> bool {
        self.macros.contains_key(name) || self.builtins.contains_key(name)
    }

    pub fn get_builtin(&self, name: &str) -> Option<&BuiltinMacro> {
        self.builtins.get(name)
    }
}

impl Default for MacroRegistry {
    fn default() -> Self {
        Self {
            macros: HashMap::new(),
            builtins: HashMap::new(),
        }
    }
}

pub fn expand_macros(program: &Program, registry: &MacroRegistry) -> MacroResult<Program> {
    let mut expander = expand::MacroExpander::new(registry);
    expander.expand_program(program)
}