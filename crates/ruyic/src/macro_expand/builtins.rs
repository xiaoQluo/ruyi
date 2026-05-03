use crate::lexer::token::Token;
use crate::macro_expand::hygiene::HygieneContext;
use crate::macro_expand::{BuiltinMacro, MacroRegistry, MacroResult};

pub fn register_builtins(registry: &mut MacroRegistry) {
    registry.builtins.insert(
        "todo".to_string(),
        BuiltinMacro {
            name: "todo".to_string(),
            hygienic: true,
            expand: expand_todo,
        },
    );

    registry.builtins.insert(
        "unreachable".to_string(),
        BuiltinMacro {
            name: "unreachable".to_string(),
            hygienic: true,
            expand: expand_unreachable,
        },
    );

    registry.builtins.insert(
        "stringify".to_string(),
        BuiltinMacro {
            name: "stringify".to_string(),
            hygienic: false,
            expand: expand_stringify,
        },
    );

    registry.builtins.insert(
        "file".to_string(),
        BuiltinMacro {
            name: "file".to_string(),
            hygienic: true,
            expand: expand_file,
        },
    );

    registry.builtins.insert(
        "line".to_string(),
        BuiltinMacro {
            name: "line".to_string(),
            hygienic: true,
            expand: expand_line,
        },
    );

    registry.builtins.insert(
        "column".to_string(),
        BuiltinMacro {
            name: "column".to_string(),
            hygienic: true,
            expand: expand_column,
        },
    );
}

fn expand_todo(_args: &[Token], _ctx: &dyn HygieneContext) -> MacroResult<Vec<Token>> {
    Ok(vec![Token::Ident("__todo_macro".to_string())])
}

fn expand_unreachable(_args: &[Token], _ctx: &dyn HygieneContext) -> MacroResult<Vec<Token>> {
    Ok(vec![Token::Ident("__unreachable_macro".to_string())])
}

fn expand_stringify(args: &[Token], _ctx: &dyn HygieneContext) -> MacroResult<Vec<Token>> {
    let source = format!("{:?}", args);
    Ok(vec![Token::String(source)])
}

fn expand_file(_args: &[Token], _ctx: &dyn HygieneContext) -> MacroResult<Vec<Token>> {
    Ok(vec![Token::String("\"<unknown>\"".to_string())])
}

fn expand_line(_args: &[Token], _ctx: &dyn HygieneContext) -> MacroResult<Vec<Token>> {
    Ok(vec![Token::Int(0)])
}

fn expand_column(_args: &[Token], _ctx: &dyn HygieneContext) -> MacroResult<Vec<Token>> {
    Ok(vec![Token::Int(0)])
}
