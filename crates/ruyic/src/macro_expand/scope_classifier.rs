//! 作用域感知的标识符分类器
//!
//! 用于宏卫生系统：在 token 级扫描模板，识别局部绑定位置（let/const/fn 参数/
//! for/catch/while let 引入的标识符），只对宏作者引入的绑定做 mangling，
//! 保留函数调用、类型引用和捕获变量不变。
//!
//! @author ruyi
//! @date 2026-07-26

use crate::lexer::token::Token;
use std::collections::{HashMap, HashSet};

/// 状态机上下文状态
#[derive(Debug, Clone, Copy, PartialEq)]
enum ScopeState {
    /// 普通位置，Ident 是引用
    Normal,
    /// let/const 之后，下一个有效 token 是绑定名或解构开始
    BindNext,
    /// 解构模式内（`{}` 或 `[]`），需要收集绑定名
    BindDestructure,
    /// fn 参数列表内，收集参数名
    InFnParams,
    /// 跳过类型注解（`:` 之后）
    SkipType,
    /// 跳过默认参数值（参数中 `=` 之后）
    SkipDefault,
    /// 解构中属性键后 `:` 之后，下一个 Ident 是绑定名
    BindAfterColon,
    /// 跳过解构中的默认值表达式
    SkipDestructureDefault,
}

/**
 * 扫描 token 流，收集所有局部绑定名。
 *
 * 识别以下语法模式中的绑定标识符：
 * - `let` / `const` 声明（含解构、类型注解、默认值）
 * - `fn` 参数列表（含类型注解、默认值）
 * - `for` 循环中的 `let` / `const` 绑定
 * - `for...in` / `for...of` 中的 `let` 绑定
 * - `while let` 简单标识符模式
 * - `catch` 模式（含解构、类型注解）
 * - `as` 别名模式
 */
pub fn collect_binding_names(template: &[Token]) -> HashSet<String> {
    let mut bindings = HashSet::new();
    let mut state = ScopeState::Normal;
    let mut i = 0;
    // 跟踪括号/花括号/方括号深度
    let mut paren_depth: u32 = 0;
    let mut brace_depth: u32 = 0;
    let mut bracket_depth: u32 = 0;
    // 记录进入当前状态时的深度，用于恢复
    let mut state_paren_depth: u32 = 0;
    // 解构深度栈（嵌套解构时用）
    let mut destructure_stack: Vec<u32> = Vec::new();
    // fn 后等待 ( 进入参数列表
    let mut awaiting_fn_params = false;

    while i < template.len() {
        let token = &template[i];

        // ── 通用规则：跳过捕获变量 ──
        if let Token::Ident(name) = token {
            if name.starts_with('$') {
                i += 1;
                continue;
            }
        }
        if *token == Token::Dollar {
            // legacy 捕获格式：$name 是两个 token
            i += 2;
            continue;
        }

        // ── 通用规则：跳过成员访问 .ident 和 ?.(ident ──
        if *token == Token::Dot || *token == Token::OptChain {
            i += 2; // 跳过 . 或 ?. 和后面的 ident
            continue;
        }

        // ── 跟踪括号深度 ──
        match token {
            Token::LParen => paren_depth += 1,
            Token::RParen => {
                paren_depth = paren_depth.saturating_sub(1);
            }
            Token::LBrace => brace_depth += 1,
            Token::RBrace => {
                brace_depth = brace_depth.saturating_sub(1);
            }
            Token::LBracket => bracket_depth += 1,
            Token::RBracket => {
                bracket_depth = bracket_depth.saturating_sub(1);
            }
            _ => {}
        }

        match state {
            ScopeState::Normal => {
                // fn 后等待 ( —— 最高优先级
                if awaiting_fn_params && *token == Token::LParen {
                    awaiting_fn_params = false;
                    // paren_depth 已在本轮递增，记录进入前的深度
                    state = ScopeState::InFnParams;
                    state_paren_depth = paren_depth - 1;
                    i += 1;
                    continue;
                }

                match token {
                    Token::Let | Token::Const => {
                        state = ScopeState::BindNext;
                    }
                    Token::Fn => {
                        // 检查是否是有名函数：fn [name] [star] (
                        let mut offset = 1;
                        // 跳过可选的函数名
                        if let Some(Token::Ident(_)) = template.get(i + offset) {
                            offset += 1;
                        }
                        // 跳过可选的 Star（已废弃的生成器标记）
                        if template.get(i + offset) == Some(&Token::Star) {
                            offset += 1;
                        }
                        // 如果后面是 (，设置标志等待进入参数列表
                        if template.get(i + offset) == Some(&Token::LParen) {
                            awaiting_fn_params = true;
                            // i += offset - 1 加上循环的 i += 1 = 前进 offset 格
                            // 使下一轮 i 正好指向 (
                            i += offset - 1;
                        }
                    }
                    Token::While => {
                        // while let 模式：while 后的 let 触发 BindNext
                        // 不做任何操作，下一轮 let 会触发
                    }
                    Token::Catch => {
                        // catch 后的 ( 内是模式绑定，下一轮 ( 时通过检查前一个 token 来识别
                    }
                    Token::LParen => {
                        // 检查是否由 catch 触发
                        if i > 0 && template[i - 1] == Token::Catch {
                            state = ScopeState::BindNext;
                            state_paren_depth = paren_depth - 1;
                        }
                    }
                    _ => {}
                }
            }

            ScopeState::BindNext => {
                match token {
                    Token::Ident(name) if !name.starts_with('$') => {
                        bindings.insert(name.clone());
                        // 检查 as 别名：x as y — y 也是绑定
                        if template.get(i + 1) == Some(&Token::As) {
                            if let Some(Token::Ident(alias)) = template.get(i + 2) {
                                if !alias.starts_with('$') {
                                    bindings.insert(alias.clone());
                                }
                            }
                            i += 2; // 跳过 As 和 alias，循环 i+=1 会再进一格
                        }
                        state = ScopeState::Normal;
                    }
                    Token::LBrace => {
                        // 对象解构
                        state = ScopeState::BindDestructure;
                        destructure_stack.clear();
                        destructure_stack.push(brace_depth);
                    }
                    Token::LBracket => {
                        // 数组解构
                        state = ScopeState::BindDestructure;
                        destructure_stack.clear();
                        destructure_stack.push(bracket_depth);
                    }
                    // 通配符、字面量模式、其他关键字 — 不是绑定
                    _ => {
                        state = ScopeState::Normal;
                    }
                }
            }

            ScopeState::BindDestructure => {
                match token {
                    Token::Ident(name) if !name.starts_with('$') => {
                        // 需要看下一个 token 来判断是绑定还是属性键
                        let next = template.get(i + 1);
                        match next {
                            Some(Token::Colon) => {
                                // {key: value} — key 是属性键，不收集
                                // 跳过 key，下一轮处理 :
                                state = ScopeState::BindAfterColon;
                                // 不收集当前 ident，让它通过
                                // 下一轮是 :，再下一轮是绑定名
                            }
                            Some(Token::Comma) | Some(Token::RBrace) | Some(Token::RBracket) => {
                                // 简写形式：{a} 或 {a, b}
                                bindings.insert(name.clone());
                            }
                            Some(Token::Assign) => {
                                // {key = default} — key 是绑定，跳过默认值
                                bindings.insert(name.clone());
                                state = ScopeState::SkipDestructureDefault;
                                i += 1; // 跳过 ident，下一轮是 =，再跳过
                                continue;
                            }
                            Some(Token::As) => {
                                // x as y — 两个都是绑定
                                bindings.insert(name.clone());
                                if let Some(Token::Ident(alias)) = template.get(i + 2) {
                                    if !alias.starts_with('$') {
                                        bindings.insert(alias.clone());
                                    }
                                }
                                i += 2; // 跳过 ident 和 as，循环 i+=1 推进到 alias
                            }
                            _ => {
                                // 默认情况：当作绑定
                                bindings.insert(name.clone());
                            }
                        }
                    }
                    Token::Spread => {
                        // ...rest — rest 是绑定，下一轮处理
                        if let Some(Token::Ident(name)) = template.get(i + 1) {
                            if !name.starts_with('$') {
                                bindings.insert(name.clone());
                            }
                        }
                        // 不手动推进，下一轮 Spread 不匹配任何状态规则
                        // 再下一轮是 Ident，会被收集
                        // 等等，Spread 在 BindDestructure 中被处理了
                        // 但 Ident 下一轮又会被当作简写收集...
                        // 所以需要跳过 Spread 后的 Ident
                        i += 1; // 跳过 Spread，循环 i+=1 推进到 Ident
                                // Ident 在下一轮会被 BindDestructure 处理，但它是 Spread 后的
                                // 需要一种方式标记“已处理”
                                // 简单方案：这里直接跳过 Spread 和 Ident
                    }
                    Token::LBrace | Token::LBracket => {
                        // 嵌套解构：进入更深层
                        let depth = if *token == Token::LBrace {
                            brace_depth
                        } else {
                            bracket_depth
                        };
                        destructure_stack.push(depth);
                    }
                    Token::RBrace | Token::RBracket => {
                        // 退出当前层解构
                        destructure_stack.pop();
                        if destructure_stack.is_empty() {
                            state = ScopeState::Normal;
                        }
                    }
                    Token::Underscore => {
                        // 通配符，不收集
                    }
                    Token::Comma => {
                        // elision（数组解构中的空位）— 继续
                    }
                    // 字面量模式 — 跳过
                    Token::Int(_)
                    | Token::BigInt(_)
                    | Token::Float(_)
                    | Token::String(_)
                    | Token::True
                    | Token::False
                    | Token::Null => {}
                    _ => {}
                }
            }

            ScopeState::BindAfterColon => {
                // : 之后，下一个有效 token 是绑定名或嵌套解构
                match token {
                    Token::Colon => {
                        // : 本身，跳过，等待下一个 token
                    }
                    Token::Ident(name) if !name.starts_with('$') => {
                        bindings.insert(name.clone());
                        state = ScopeState::BindDestructure;
                    }
                    Token::LBrace | Token::LBracket => {
                        // {key: {nested}} 或 {key: [nested]} — 嵌套解构
                        let depth = if *token == Token::LBrace {
                            brace_depth
                        } else {
                            bracket_depth
                        };
                        destructure_stack.push(depth);
                        state = ScopeState::BindDestructure;
                    }
                    _ => {
                        state = ScopeState::BindDestructure;
                    }
                }
            }

            ScopeState::SkipDestructureDefault => {
                // 跳过默认值表达式，直到回到同层的 , 或 } 或 ]
                let target_depth = *destructure_stack.last().unwrap_or(&0);
                if (brace_depth < target_depth) || (bracket_depth < target_depth) {
                    // 退出了当前层
                    if destructure_stack.is_empty() {
                        state = ScopeState::Normal;
                    } else {
                        state = ScopeState::BindDestructure;
                    }
                } else if *token == Token::Comma {
                    state = ScopeState::BindDestructure;
                }
            }

            ScopeState::InFnParams => {
                match token {
                    Token::Ident(name) if !name.starts_with('$') => {
                        bindings.insert(name.clone());
                    }
                    Token::Colon => {
                        // 类型注解开始
                        state = ScopeState::SkipType;
                        state_paren_depth = paren_depth;
                    }
                    Token::Assign => {
                        // 默认参数值开始
                        state = ScopeState::SkipDefault;
                        state_paren_depth = paren_depth;
                    }
                    Token::Comma => {
                        // 下一个参数
                    }
                    Token::RParen => {
                        // 参数列表结束（深度已在本轮递减）
                        if paren_depth <= state_paren_depth {
                            state = ScopeState::Normal;
                        }
                    }
                    _ => {}
                }
            }

            ScopeState::SkipType => {
                // 跳过类型注解直到 = , ) ; 在同层深度
                match token {
                    Token::Assign | Token::Comma | Token::SemiColon => {
                        state = ScopeState::InFnParams;
                    }
                    Token::RParen => {
                        if paren_depth <= state_paren_depth {
                            state = ScopeState::Normal;
                        } else {
                            state = ScopeState::InFnParams;
                        }
                    }
                    _ => {}
                }
            }

            ScopeState::SkipDefault => {
                // 跳过默认值直到 , ) 在同层深度
                match token {
                    Token::Comma => {
                        state = ScopeState::InFnParams;
                    }
                    Token::RParen => {
                        if paren_depth <= state_paren_depth {
                            state = ScopeState::Normal;
                        } else {
                            state = ScopeState::InFnParams;
                        }
                    }
                    _ => {}
                }
            }
        }

        i += 1;
    }

    bindings
}

/**
 * 对 token 流做选择性 mangling。
 *
 * 只替换 `rename_map` 中存在的标识符，其他 token 保持不变。
 * 捕获变量（`$` 前缀或 `Dollar` token）不受影响。
 */
pub fn apply_selective_mangling(
    tokens: Vec<Token>,
    rename_map: &HashMap<String, String>,
) -> Vec<Token> {
    tokens
        .into_iter()
        .map(|token| {
            if let Token::Ident(ref name) = token {
                if !name.starts_with('$') {
                    if let Some(mangled) = rename_map.get(name) {
                        return Token::Ident(mangled.clone());
                    }
                }
            }
            token
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 辅助函数：从 token 切片收集绑定名
    fn collect(tokens: &[Token]) -> HashSet<String> {
        collect_binding_names(tokens)
    }

    /// 辅助函数：构造 token 序列
    fn tokens(toks: Vec<Token>) -> Vec<Token> {
        toks
    }

    #[test]
    fn test_let_simple_binding() {
        // let x = 0;
        let toks = tokens(vec![
            Token::Let,
            Token::Ident("x".into()),
            Token::Assign,
            Token::Int(0),
            Token::SemiColon,
        ]);
        let bindings = collect(&toks);
        assert_eq!(bindings, HashSet::from(["x".to_string()]));
    }

    #[test]
    fn test_let_object_destructure() {
        // let {a, b: c} = obj;
        let toks = tokens(vec![
            Token::Let,
            Token::LBrace,
            Token::Ident("a".into()),
            Token::Comma,
            Token::Ident("b".into()),
            Token::Colon,
            Token::Ident("c".into()),
            Token::RBrace,
            Token::Assign,
            Token::Ident("obj".into()),
            Token::SemiColon,
        ]);
        let bindings = collect(&toks);
        assert_eq!(bindings, HashSet::from(["a".to_string(), "c".to_string()]));
    }

    #[test]
    fn test_let_array_destructure_with_rest() {
        // let [first, ...rest] = arr;
        let toks = tokens(vec![
            Token::Let,
            Token::LBracket,
            Token::Ident("first".into()),
            Token::Comma,
            Token::Spread,
            Token::Ident("rest".into()),
            Token::RBracket,
            Token::Assign,
            Token::Ident("arr".into()),
            Token::SemiColon,
        ]);
        let bindings = collect(&toks);
        assert_eq!(
            bindings,
            HashSet::from(["first".to_string(), "rest".to_string()])
        );
    }

    #[test]
    fn test_let_nested_destructure() {
        // let {nested: [a, b]} = obj;
        let toks = tokens(vec![
            Token::Let,
            Token::LBrace,
            Token::Ident("nested".into()),
            Token::Colon,
            Token::LBracket,
            Token::Ident("a".into()),
            Token::Comma,
            Token::Ident("b".into()),
            Token::RBracket,
            Token::RBrace,
            Token::Assign,
            Token::Ident("obj".into()),
            Token::SemiColon,
        ]);
        let bindings = collect(&toks);
        assert_eq!(bindings, HashSet::from(["a".to_string(), "b".to_string()]));
    }

    #[test]
    fn test_let_with_type_annotation() {
        // let x: int = 5;
        let toks = tokens(vec![
            Token::Let,
            Token::Ident("x".into()),
            Token::Colon,
            Token::Ident("int".into()),
            Token::Assign,
            Token::Int(5),
            Token::SemiColon,
        ]);
        let bindings = collect(&toks);
        // x 是绑定，int 不是（类型注解）
        assert_eq!(bindings, HashSet::from(["x".to_string()]));
    }

    #[test]
    fn test_fn_named_params_with_types() {
        // fn foo(a: int, b: string = "hello") { }
        let toks = tokens(vec![
            Token::Fn,
            Token::Ident("foo".into()),
            Token::LParen,
            Token::Ident("a".into()),
            Token::Colon,
            Token::Ident("int".into()),
            Token::Comma,
            Token::Ident("b".into()),
            Token::Colon,
            Token::Ident("string".into()),
            Token::Assign,
            Token::String("hello".into()),
            Token::RParen,
            Token::LBrace,
            Token::RBrace,
        ]);
        let bindings = collect(&toks);
        // a, b 是绑定，foo 不是（函数名）
        assert_eq!(bindings, HashSet::from(["a".to_string(), "b".to_string()]));
    }

    #[test]
    fn test_fn_anonymous_params() {
        // fn(x, y) { }
        let toks = tokens(vec![
            Token::Fn,
            Token::LParen,
            Token::Ident("x".into()),
            Token::Comma,
            Token::Ident("y".into()),
            Token::RParen,
            Token::LBrace,
            Token::RBrace,
        ]);
        let bindings = collect(&toks);
        assert_eq!(bindings, HashSet::from(["x".to_string(), "y".to_string()]));
    }

    #[test]
    fn test_for_c_style() {
        // for (let i = 0; i < n; i = i + 1) { }
        let toks = tokens(vec![
            Token::For,
            Token::LParen,
            Token::Let,
            Token::Ident("i".into()),
            Token::Assign,
            Token::Int(0),
            Token::SemiColon,
            Token::Ident("i".into()),
            Token::Less,
            Token::Ident("n".into()),
            Token::SemiColon,
            Token::Ident("i".into()),
            Token::Assign,
            Token::Ident("i".into()),
            Token::Plus,
            Token::Int(1),
            Token::RParen,
            Token::LBrace,
            Token::RBrace,
        ]);
        let bindings = collect(&toks);
        assert_eq!(bindings, HashSet::from(["i".to_string()]));
    }

    #[test]
    fn test_for_of() {
        // for (let x of arr) { }
        let toks = tokens(vec![
            Token::For,
            Token::LParen,
            Token::Let,
            Token::Ident("x".into()),
            Token::Of,
            Token::Ident("arr".into()),
            Token::RParen,
            Token::LBrace,
            Token::RBrace,
        ]);
        let bindings = collect(&toks);
        assert_eq!(bindings, HashSet::from(["x".to_string()]));
    }

    #[test]
    fn test_catch_simple() {
        // catch (e) { }
        let toks = tokens(vec![
            Token::Catch,
            Token::LParen,
            Token::Ident("e".into()),
            Token::RParen,
            Token::LBrace,
            Token::RBrace,
        ]);
        let bindings = collect(&toks);
        assert_eq!(bindings, HashSet::from(["e".to_string()]));
    }

    #[test]
    fn test_catch_destructure() {
        // catch ({message, code}) { }
        let toks = tokens(vec![
            Token::Catch,
            Token::LParen,
            Token::LBrace,
            Token::Ident("message".into()),
            Token::Comma,
            Token::Ident("code".into()),
            Token::RBrace,
            Token::RParen,
            Token::LBrace,
            Token::RBrace,
        ]);
        let bindings = collect(&toks);
        assert_eq!(
            bindings,
            HashSet::from(["message".to_string(), "code".to_string()])
        );
    }

    #[test]
    fn test_catch_with_type_annotation() {
        // catch (e: ErrorType) { }
        let toks = tokens(vec![
            Token::Catch,
            Token::LParen,
            Token::Ident("e".into()),
            Token::Colon,
            Token::Ident("ErrorType".into()),
            Token::RParen,
            Token::LBrace,
            Token::RBrace,
        ]);
        let bindings = collect(&toks);
        // e 是绑定，ErrorType 不是（类型注解）
        assert_eq!(bindings, HashSet::from(["e".to_string()]));
    }

    #[test]
    fn test_while_let_simple() {
        // while let x = val { }
        let toks = tokens(vec![
            Token::While,
            Token::Let,
            Token::Ident("x".into()),
            Token::Assign,
            Token::Ident("val".into()),
            Token::LBrace,
            Token::RBrace,
        ]);
        let bindings = collect(&toks);
        assert_eq!(bindings, HashSet::from(["x".to_string()]));
    }

    #[test]
    fn test_as_alias() {
        // let x as y = val;
        let toks = tokens(vec![
            Token::Let,
            Token::Ident("x".into()),
            Token::As,
            Token::Ident("y".into()),
            Token::Assign,
            Token::Ident("val".into()),
            Token::SemiColon,
        ]);
        let bindings = collect(&toks);
        assert_eq!(bindings, HashSet::from(["x".to_string(), "y".to_string()]));
    }

    #[test]
    fn test_destructure_default_value() {
        // let {a = 5} = obj;
        let toks = tokens(vec![
            Token::Let,
            Token::LBrace,
            Token::Ident("a".into()),
            Token::Assign,
            Token::Int(5),
            Token::RBrace,
            Token::Assign,
            Token::Ident("obj".into()),
            Token::SemiColon,
        ]);
        let bindings = collect(&toks);
        assert_eq!(bindings, HashSet::from(["a".to_string()]));
    }

    #[test]
    fn test_array_elision() {
        // let [, a, , b] = arr;
        let toks = tokens(vec![
            Token::Let,
            Token::LBracket,
            Token::Comma,
            Token::Ident("a".into()),
            Token::Comma,
            Token::Comma,
            Token::Ident("b".into()),
            Token::RBracket,
            Token::Assign,
            Token::Ident("arr".into()),
            Token::SemiColon,
        ]);
        let bindings = collect(&toks);
        assert_eq!(bindings, HashSet::from(["a".to_string(), "b".to_string()]));
    }

    #[test]
    fn test_wildcard_not_binding() {
        // let _ = expr;
        let toks = tokens(vec![
            Token::Let,
            Token::Underscore,
            Token::Assign,
            Token::Ident("expr".into()),
            Token::SemiColon,
        ]);
        let bindings = collect(&toks);
        assert!(bindings.is_empty());
    }

    #[test]
    fn test_no_bindings_in_expression() {
        // print(x)
        let toks = tokens(vec![
            Token::Ident("print".into()),
            Token::LParen,
            Token::Ident("x".into()),
            Token::RParen,
        ]);
        let bindings = collect(&toks);
        assert!(bindings.is_empty());
    }

    #[test]
    fn test_capture_variable_not_binding() {
        // let tmp = $a;
        let toks = tokens(vec![
            Token::Let,
            Token::Ident("tmp".into()),
            Token::Assign,
            Token::Ident("$a".into()),
            Token::SemiColon,
        ]);
        let bindings = collect(&toks);
        assert_eq!(bindings, HashSet::from(["tmp".to_string()]));
    }

    #[test]
    fn test_selective_mangling() {
        // 测试 apply_selective_mangling 只替换 rename_map 中的标识符
        let input = vec![
            Token::Let,
            Token::Ident("tmp".into()),
            Token::Assign,
            Token::Ident("print".into()),
            Token::LParen,
            Token::Ident("tmp".into()),
            Token::RParen,
        ];
        let mut rename_map = HashMap::new();
        rename_map.insert("tmp".to_string(), "__hygiene_tmp_1".to_string());

        let result = apply_selective_mangling(input, &rename_map);

        assert_eq!(result[0], Token::Let);
        assert_eq!(result[1], Token::Ident("__hygiene_tmp_1".into()));
        assert_eq!(result[2], Token::Assign);
        assert_eq!(result[3], Token::Ident("print".into())); // 不变
        assert_eq!(result[4], Token::LParen);
        assert_eq!(result[5], Token::Ident("__hygiene_tmp_1".into()));
        assert_eq!(result[6], Token::RParen);
    }

    #[test]
    fn test_mangling_preserves_captures() {
        // 捕获变量 $x 不被 mangling
        let input = vec![
            Token::Ident("tmp".into()),
            Token::Assign,
            Token::Ident("$x".into()),
        ];
        let mut rename_map = HashMap::new();
        rename_map.insert("tmp".to_string(), "__hygiene_tmp_1".to_string());

        let result = apply_selective_mangling(input, &rename_map);

        assert_eq!(result[0], Token::Ident("__hygiene_tmp_1".into()));
        assert_eq!(result[1], Token::Assign);
        assert_eq!(result[2], Token::Ident("$x".into())); // 不变
    }

    #[test]
    fn test_const_binding() {
        // const N = 10;
        let toks = tokens(vec![
            Token::Const,
            Token::Ident("N".into()),
            Token::Assign,
            Token::Int(10),
            Token::SemiColon,
        ]);
        let bindings = collect(&toks);
        assert_eq!(bindings, HashSet::from(["N".to_string()]));
    }

    #[test]
    fn test_fn_params_simple() {
        // fn foo(a, b) { }
        let toks = tokens(vec![
            Token::Fn,
            Token::Ident("foo".into()),
            Token::LParen,
            Token::Ident("a".into()),
            Token::Comma,
            Token::Ident("b".into()),
            Token::RParen,
            Token::LBrace,
            Token::RBrace,
        ]);
        let bindings = collect(&toks);
        assert_eq!(bindings, HashSet::from(["a".to_string(), "b".to_string()]));
    }

    #[test]
    fn test_for_in() {
        // for (let k in obj) { }
        let toks = tokens(vec![
            Token::For,
            Token::LParen,
            Token::Let,
            Token::Ident("k".into()),
            Token::In,
            Token::Ident("obj".into()),
            Token::RParen,
            Token::LBrace,
            Token::RBrace,
        ]);
        let bindings = collect(&toks);
        assert_eq!(bindings, HashSet::from(["k".to_string()]));
    }

    #[test]
    fn test_member_access_not_binding() {
        // let result = obj.field;
        let toks = tokens(vec![
            Token::Let,
            Token::Ident("result".into()),
            Token::Assign,
            Token::Ident("obj".into()),
            Token::Dot,
            Token::Ident("field".into()),
            Token::SemiColon,
        ]);
        let bindings = collect(&toks);
        // result 是绑定，obj 和 field 不是
        assert_eq!(bindings, HashSet::from(["result".to_string()]));
    }

    #[test]
    fn test_multiple_let_statements() {
        // let x = 1; let y = 2;
        let toks = tokens(vec![
            Token::Let,
            Token::Ident("x".into()),
            Token::Assign,
            Token::Int(1),
            Token::SemiColon,
            Token::Let,
            Token::Ident("y".into()),
            Token::Assign,
            Token::Int(2),
            Token::SemiColon,
        ]);
        let bindings = collect(&toks);
        assert_eq!(bindings, HashSet::from(["x".to_string(), "y".to_string()]));
    }

    #[test]
    fn test_complex_macro_template() {
        // let tmp = $a; $a = $b; $b = tmp;
        let toks = tokens(vec![
            Token::Let,
            Token::Ident("tmp".into()),
            Token::Assign,
            Token::Ident("$a".into()),
            Token::SemiColon,
            Token::Ident("$a".into()),
            Token::Assign,
            Token::Ident("$b".into()),
            Token::SemiColon,
            Token::Ident("$b".into()),
            Token::Assign,
            Token::Ident("tmp".into()),
            Token::SemiColon,
        ]);
        let bindings = collect(&toks);
        // 只有 tmp 是绑定，$a 和 $b 是捕获变量
        assert_eq!(bindings, HashSet::from(["tmp".to_string()]));
    }
}
