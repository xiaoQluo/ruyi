//! Free-variable analysis for closure capture.
//!
//! Walks a function/arrow body and reports the names it reads or assigns
//! that it does not bind itself. The caller filters the read set against
//! the variables visible at the closure-creation site; those hits become
//! the closure's capture environment.

use std::collections::HashSet;

use crate::parser::ast::{
    Argument, ArrayPatternElement, ArrowBody, ClassElement, Declaration, Expr, ForInit,
    ObjectPatternField, Pattern, Statement,
};

#[derive(Debug, Default, Clone)]
pub struct CaptureAnalysis {
    /// Free names read by the body, first-reference order, deduplicated.
    pub reads: Vec<String>,
    /// Free names written by an assignment (subset of reads semantics,
    /// tracked separately so the compiler can reject mutable captures).
    pub assigned: Vec<String>,
}

struct Walker {
    bound: HashSet<String>,
    read_set: HashSet<String>,
    reads: Vec<String>,
    assigned_set: HashSet<String>,
    assigned: Vec<String>,
    /// True while walking inside a closure literal (arrow or fn
    /// expression). Assignments recorded under this flag name variables
    /// owned by an enclosing scope, i.e. shared-capture candidates.
    inside_closure: bool,
    /// Bindings made inside each open closure (stack, one entry per
    /// nesting level). Unlike `bound` — which also accumulates enclosing
    /// bindings during the module-wide walk — this set decides whether an
    /// assignment inside a closure targets the closure itself.
    closure_locals: Vec<HashSet<String>>,
    /// Names assigned inside closure literals (subset of reads).
    closure_assigned: HashSet<String>,
    /// Enclosing bindings read inside a closure literal — by-value capture
    /// candidates that any outer mutation would desynchronize.
    closure_captured: HashSet<String>,
    /// Names assigned outside every closure literal (module/function-body
    /// assignments). Intersected with `closure_captured` to decide boxing.
    assigned_outside_closure: HashSet<String>,
}

impl Walker {
    fn new(bound: HashSet<String>) -> Self {
        Self {
            bound,
            read_set: HashSet::new(),
            reads: Vec::new(),
            assigned_set: HashSet::new(),
            assigned: Vec::new(),
            inside_closure: false,
            closure_locals: Vec::new(),
            closure_assigned: HashSet::new(),
            closure_captured: HashSet::new(),
            assigned_outside_closure: HashSet::new(),
        }
    }

    /// A name read (or assigned) inside a closure that resolves to an
    /// enclosing binding is captured; record it for the shared-storage
    /// decision in `collect_closure_shared_names`.
    fn note_capture_if_enclosing(&mut self, name: &str) {
        if self.inside_closure
            && !self
                .closure_locals
                .iter()
                .any(|locals| locals.contains(name))
            && self.bound.contains(name)
        {
            self.closure_captured.insert(name.to_string());
        }
    }

    fn record_read(&mut self, name: &str) {
        self.note_capture_if_enclosing(name);
        if self.bound.contains(name) {
            return;
        }
        if self.read_set.insert(name.to_string()) {
            self.reads.push(name.to_string());
        }
    }

    fn record_assign(&mut self, name: &str) {
        // Bound inside the closure itself: neither a shared-capture
        // candidate nor an enclosing capture.
        if self
            .closure_locals
            .iter()
            .any(|locals| locals.contains(name))
        {
            return;
        }
        if self.inside_closure {
            self.closure_assigned.insert(name.to_string());
            self.note_capture_if_enclosing(name);
        } else {
            self.assigned_outside_closure.insert(name.to_string());
        }
        if self.bound.contains(name) {
            return;
        }
        self.record_read(name);
        if self.assigned_set.insert(name.to_string()) {
            self.assigned.push(name.to_string());
        }
    }

    fn bind_local(&mut self, name: String) {
        if let Some(locals) = self.closure_locals.last_mut() {
            locals.insert(name.clone());
        }
        self.bound.insert(name);
    }

    fn bind_pattern(&mut self, pattern: &Pattern) {
        match pattern {
            Pattern::Identifier(name) => {
                self.bind_local(name.clone());
            }
            Pattern::Literal(expr) => self.walk_expr(expr),
            Pattern::Object(fields) => {
                for field in fields {
                    self.bind_object_field(field);
                }
            }
            Pattern::Array(elements) => {
                for element in elements {
                    self.bind_array_element(element);
                }
            }
            Pattern::Rest(name) => {
                self.bind_local(name.clone());
            }
            Pattern::As(inner, name) => {
                self.bind_pattern(inner);
                self.bind_local(name.clone());
            }
            Pattern::Or(patterns) => {
                for pattern in patterns {
                    self.bind_pattern(pattern);
                }
            }
            Pattern::Wildcard => {}
        }
    }

    fn bind_object_field(&mut self, field: &ObjectPatternField) {
        match field {
            ObjectPatternField::Shorthand(name) | ObjectPatternField::Rest(name) => {
                self.bind_local(name.clone());
            }
            ObjectPatternField::ShorthandDefault(name, default) => {
                self.walk_expr(default);
                self.bind_local(name.clone());
            }
            ObjectPatternField::Property { pattern, .. } => self.bind_pattern(pattern),
        }
    }

    fn bind_array_element(&mut self, element: &ArrayPatternElement) {
        match element {
            ArrayPatternElement::Pattern(pattern) => self.bind_pattern(pattern),
            ArrayPatternElement::Default(pattern, default) => {
                self.walk_expr(default);
                self.bind_pattern(pattern);
            }
            ArrayPatternElement::Rest(pattern) => self.bind_pattern(pattern),
            ArrayPatternElement::Elision => {}
        }
    }

    fn pattern_bindings(pattern: &Pattern, out: &mut Vec<String>) {
        match pattern {
            Pattern::Identifier(name) => out.push(name.clone()),
            Pattern::Object(fields) => {
                for field in fields {
                    match field {
                        ObjectPatternField::Shorthand(name)
                        | ObjectPatternField::Rest(name)
                        | ObjectPatternField::ShorthandDefault(name, _) => out.push(name.clone()),
                        ObjectPatternField::Property { pattern, .. } => {
                            Self::pattern_bindings(pattern, out)
                        }
                    }
                }
            }
            Pattern::Array(elements) => {
                for element in elements {
                    match element {
                        ArrayPatternElement::Pattern(pattern)
                        | ArrayPatternElement::Rest(pattern) => {
                            Self::pattern_bindings(pattern, out)
                        }
                        ArrayPatternElement::Default(pattern, _) => {
                            Self::pattern_bindings(pattern, out)
                        }
                        ArrayPatternElement::Elision => {}
                    }
                }
            }
            Pattern::Rest(name) => out.push(name.clone()),
            Pattern::As(_, name) => out.push(name.clone()),
            Pattern::Or(patterns) => {
                for pattern in patterns {
                    Self::pattern_bindings(pattern, out);
                }
            }
            Pattern::Literal(_) | Pattern::Wildcard => {}
        }
    }

    fn walk_statements(&mut self, statements: &[Statement]) {
        for statement in statements {
            self.walk_statement(statement);
        }
    }

    fn walk_scoped(&mut self, statements: &[Statement]) {
        let saved = self.bound.clone();
        self.walk_statements(statements);
        self.bound = saved;
    }

    fn walk_declaration(&mut self, declaration: &Declaration) {
        match declaration {
            Declaration::Let(bindings) | Declaration::Const(bindings) => {
                for binding in bindings {
                    if let Some(init) = &binding.init {
                        self.walk_expr(init);
                    }
                    self.bind_pattern(&binding.pattern);
                }
            }
            Declaration::Function {
                name, params, body, ..
            } => {
                self.bind_local(name.clone());
                self.walk_function(params, body);
            }
            Declaration::Class { name, body, .. } => {
                self.bind_local(name.clone());
                self.walk_class_body(body);
            }
            Declaration::Trait { .. }
            | Declaration::TypeAlias { .. }
            | Declaration::Macro { .. } => {}
            Declaration::Impl { body, .. } => self.walk_class_body(body),
            // Externs declare module-level names, not locals.
            Declaration::ExternFn { .. } => {}
        }
    }

    fn walk_function(&mut self, params: &[crate::parser::ast::Param], body: &[Statement]) {
        let saved = self.bound.clone();
        let mut names = Vec::new();
        for param in params {
            Self::pattern_bindings(&param.pattern, &mut names);
        }
        for name in names {
            self.bind_local(name);
        }
        self.walk_statements(body);
        self.bound = saved;
    }

    fn walk_class_body(&mut self, body: &[ClassElement]) {
        for element in body {
            match element {
                ClassElement::Method { params, body, .. } => self.walk_function(params, body),
                ClassElement::Field { init, .. } => {
                    if let Some(init) = init {
                        self.walk_expr(init);
                    }
                }
                ClassElement::Empty => {}
            }
        }
    }

    fn walk_statement(&mut self, statement: &Statement) {
        match statement {
            Statement::Block(statements) => self.walk_scoped(statements),
            Statement::Expression(expr) => self.walk_expr(expr),
            Statement::If {
                condition,
                then_branch,
                else_branch,
            } => {
                self.walk_expr(condition);
                self.walk_statement(then_branch);
                if let Some(else_branch) = else_branch {
                    self.walk_statement(else_branch);
                }
            }
            Statement::IfLet {
                pattern,
                value,
                then_branch,
                else_branch,
            } => {
                self.walk_expr(value);
                let saved = self.bound.clone();
                self.bind_pattern(pattern);
                self.walk_statement(then_branch);
                self.bound = saved;
                if let Some(else_branch) = else_branch {
                    self.walk_statement(else_branch);
                }
            }
            Statement::While { condition, body } => {
                self.walk_expr(condition);
                self.walk_statement(body);
            }
            Statement::WhileLet {
                pattern,
                value,
                body,
            } => {
                self.walk_expr(value);
                let saved = self.bound.clone();
                self.bind_pattern(pattern);
                self.walk_statement(body);
                self.bound = saved;
            }
            Statement::For {
                init,
                condition,
                update,
                body,
            } => {
                let saved = self.bound.clone();
                if let Some(init) = init {
                    match init {
                        ForInit::VarDecl(declaration) => self.walk_declaration(declaration),
                        ForInit::Expr(expr) => self.walk_expr(expr),
                    }
                }
                if let Some(condition) = condition {
                    self.walk_expr(condition);
                }
                if let Some(update) = update {
                    self.walk_expr(update);
                }
                self.walk_statement(body);
                self.bound = saved;
            }
            Statement::ForIn {
                variable,
                iterable,
                body,
            } => {
                self.walk_expr(iterable);
                let saved = self.bound.clone();
                self.bind_local(variable.clone());
                self.walk_statement(body);
                self.bound = saved;
            }
            Statement::ForOf {
                variable,
                iterable,
                body,
                ..
            } => {
                self.walk_expr(iterable);
                let saved = self.bound.clone();
                self.bind_local(variable.clone());
                self.walk_statement(body);
                self.bound = saved;
            }
            Statement::Return(expr) => {
                if let Some(expr) = expr {
                    self.walk_expr(expr);
                }
            }
            Statement::Throw(expr) => self.walk_expr(expr),
            Statement::Try {
                body,
                catch,
                finally,
            } => {
                self.walk_scoped(body);
                for clause in catch {
                    let saved = self.bound.clone();
                    if let Some(pattern) = &clause.pattern {
                        self.bind_pattern(pattern);
                    }
                    self.walk_statements(&clause.body);
                    self.bound = saved;
                }
                if let Some(finally) = finally {
                    self.walk_scoped(finally);
                }
            }
            Statement::Match { value, arms } => {
                self.walk_expr(value);
                for arm in arms {
                    let saved = self.bound.clone();
                    self.bind_pattern(&arm.pattern);
                    if let Some(guard) = &arm.guard {
                        self.walk_expr(guard);
                    }
                    self.walk_statements(&arm.body);
                    self.bound = saved;
                }
            }
            Statement::Break(_) | Statement::Continue(_) | Statement::Empty => {}
            Statement::Yield(expr) => {
                if let Some(expr) = expr {
                    self.walk_expr(expr);
                }
            }
            Statement::Labeled { body, .. } => self.walk_statement(body),
            Statement::Declaration(declaration) => self.walk_declaration(declaration),
        }
    }

    fn walk_expr(&mut self, expr: &Expr) {
        match expr {
            Expr::Identifier(name) => self.record_read(name),
            Expr::IntLiteral(_)
            | Expr::FloatLiteral(_)
            | Expr::StringLiteral(_)
            | Expr::BigIntLiteral(_)
            | Expr::BooleanLiteral(_)
            | Expr::NullLiteral
            | Expr::This
            | Expr::Super => {}
            Expr::SelfExpr => self.record_read("self"),
            Expr::TemplateLiteral(parts) => {
                for part in parts {
                    if let crate::parser::ast::TemplatePart::Expr(inner) = part {
                        self.walk_expr(inner);
                    }
                }
            }
            Expr::ArrayLiteral(elements) => {
                for element in elements {
                    match element {
                        crate::parser::ast::ArrayElement::Expr(inner) => self.walk_expr(inner),
                        crate::parser::ast::ArrayElement::Spread(inner) => self.walk_expr(inner),
                        crate::parser::ast::ArrayElement::Elision => {}
                    }
                }
            }
            Expr::ObjectLiteral(properties) => {
                for property in properties {
                    match property {
                        crate::parser::ast::ObjectProperty::Property { value, .. } => {
                            self.walk_expr(value)
                        }
                        crate::parser::ast::ObjectProperty::Shorthand(name) => {
                            self.record_read(name)
                        }
                        crate::parser::ast::ObjectProperty::Spread(inner) => self.walk_expr(inner),
                        crate::parser::ast::ObjectProperty::ComputedProperty { key, value } => {
                            self.walk_expr(key);
                            self.walk_expr(value);
                        }
                    }
                }
            }
            Expr::Binary { left, right, .. } => {
                self.walk_expr(left);
                self.walk_expr(right);
            }
            Expr::Unary { operand, .. } => self.walk_expr(operand),
            Expr::Call { callee, args } => {
                self.walk_expr(callee);
                self.walk_args(args);
            }
            Expr::OptionalCall { callee, args } => {
                self.walk_expr(callee);
                self.walk_args(args);
            }
            Expr::Member {
                object, property, ..
            } => {
                self.walk_expr(object);
                if let crate::parser::ast::MemberProperty::Expr(key) = property {
                    self.walk_expr(key);
                }
            }
            Expr::Conditional {
                condition,
                then_branch,
                else_branch,
            } => {
                self.walk_expr(condition);
                self.walk_expr(then_branch);
                self.walk_expr(else_branch);
            }
            Expr::Assignment { left, right, .. } => {
                self.walk_expr(right);
                // The write target is analyzed as a read (compound
                // assignment reads it too) and, for plain identifiers,
                // as an assignment so mutable captures can be rejected.
                if let Expr::Identifier(name) = left.as_ref() {
                    self.record_read(name);
                    self.record_assign(name);
                } else {
                    self.walk_expr(left);
                }
            }
            Expr::ArrowFunction { params, body, .. } => {
                let saved = self.inside_closure;
                self.inside_closure = true;
                self.closure_locals.push(HashSet::new());
                self.walk_arrow(params, body);
                self.closure_locals.pop();
                self.inside_closure = saved;
            }
            Expr::Await(inner) => self.walk_expr(inner),
            Expr::Sequence(exprs) => {
                for inner in exprs {
                    self.walk_expr(inner);
                }
            }
            Expr::Function {
                name, params, body, ..
            } => {
                if let Some(name) = name {
                    self.bind_local(name.clone());
                }
                let saved = self.inside_closure;
                self.inside_closure = true;
                self.closure_locals.push(HashSet::new());
                self.walk_function(params, body);
                self.closure_locals.pop();
                self.inside_closure = saved;
            }
            Expr::Class { body, .. } => self.walk_class_body(body),
            Expr::New { callee, args } => {
                self.walk_expr(callee);
                self.walk_args(args);
            }
            Expr::Match { value, arms } => {
                self.walk_expr(value);
                for arm in arms {
                    let saved = self.bound.clone();
                    self.bind_pattern(&arm.pattern);
                    if let Some(guard) = &arm.guard {
                        self.walk_expr(guard);
                    }
                    self.walk_statements(&arm.body);
                    self.bound = saved;
                }
            }
            Expr::If {
                condition,
                then_branch,
                else_branch,
            } => {
                self.walk_expr(condition);
                self.walk_expr(then_branch);
                if let Some(else_branch) = else_branch {
                    self.walk_expr(else_branch);
                }
            }
            Expr::Grouping(inner) | Expr::NullAssert(inner) => self.walk_expr(inner),
            Expr::Block(statements) => self.walk_scoped(statements),
            // Parse-stage intermediate node; never reaches codegen.
            Expr::ArrowParams(_) => {}
        }
    }

    fn walk_arrow(&mut self, params: &[crate::parser::ast::Param], body: &ArrowBody) {
        let saved = self.bound.clone();
        let mut names = Vec::new();
        for param in params {
            Self::pattern_bindings(&param.pattern, &mut names);
        }
        for name in names {
            self.bind_local(name);
        }
        match body {
            ArrowBody::Expr(inner) => self.walk_expr(inner),
            ArrowBody::Block(statements) => self.walk_statements(statements),
        }
        self.bound = saved;
    }

    fn walk_args(&mut self, args: &[Argument]) {
        for arg in args {
            match arg {
                Argument::Expr(inner) | Argument::Spread(inner) => self.walk_expr(inner),
            }
        }
    }
}

/// Analyze a function or arrow body: returns every free name it reads and
/// the subset it assigns to. Nested functions are walked in the enclosing
/// scope so a name free in a nested closure also shows up here — that is
/// required for transitive captures, where an inner arrow's environment is
/// materialized by the outer arrow's body.
pub fn analyze(
    params: &[crate::parser::ast::Param],
    body_statements: &[Statement],
) -> CaptureAnalysis {
    let mut bound = HashSet::new();
    let mut names = Vec::new();
    for param in params {
        Walker::pattern_bindings(&param.pattern, &mut names);
    }
    for name in names {
        bound.insert(name);
    }
    let mut walker = Walker::new(bound);
    walker.walk_statements(body_statements);
    CaptureAnalysis {
        reads: walker.reads,
        assigned: walker.assigned,
    }
}

/// Module-wide pre-pass: decide which `let` bindings must live in shared
/// GC-heap cells so closures and the enclosing scope observe one storage
/// location. Two situations qualify:
///
/// 1. the name is assigned inside a closure literal (the closure mutates
///    an enclosing local — counter patterns), or
/// 2. the name is read inside a closure AND assigned outside every
///    closure (outer mutation must be visible to the closure — condvar
///    flag patterns; a by-value copy would deadlock the reader).
///
/// The set is intentionally scope-insensitive: an unrelated local sharing
/// the name is boxed too, which is transparent to its load/store codegen.
pub fn collect_closure_shared_names(items: &[crate::parser::ast::ModuleItem]) -> HashSet<String> {
    let mut walker = Walker::new(HashSet::new());
    for item in items {
        match item {
            crate::parser::ast::ModuleItem::Declaration(decl) => walker.walk_declaration(decl),
            crate::parser::ast::ModuleItem::Export(export) => match export {
                crate::parser::ast::ExportDecl::Declaration(decl) => walker.walk_declaration(decl),
                crate::parser::ast::ExportDecl::DefaultExpr(expr) => walker.walk_expr(expr),
                _ => {}
            },
            crate::parser::ast::ModuleItem::Statement(stmt) => walker.walk_statement(stmt),
            crate::parser::ast::ModuleItem::Import(_) => {}
        }
    }
    let mut shared = walker.closure_assigned;
    for name in walker.closure_captured {
        if walker.assigned_outside_closure.contains(&name) {
            shared.insert(name);
        }
    }
    shared
}

use crate::typechecker::types::Type;

/// Byte size of one capture-environment slot. Slots are 8-byte aligned;
/// only multi-word values exceed one word. Must stay in sync with
/// `ruyi_type_to_llvm`: every type that lowers to an LLVM struct
/// (Dynamic, trait objects, tuples, function values) occupies more than
/// one slot word.
pub fn capture_slot_size(ty: &Type) -> u64 {
    match ty {
        Type::Nullable(inner) => capture_slot_size(inner),
        Type::Dynamic | Type::Trait(_) | Type::Function { .. } => 16,
        Type::Tuple(types) => 8 * types.len() as u64,
        _ => 8,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::Parser;

    #[test]
    fn collect_finds_closure_assigned_names() {
        let source = r#"
fn main() {
  let counter: int = 0;
  let bump = () => { counter = counter + 1; };
  let t = Thread.spawn(fn(arg) { counter = counter + 2; });
  let other = 5;
  other = 6;
}
"#;
        let mut parser = Parser::new(source).expect("lexer should not fail");
        let program = parser.parse().unwrap();
        let names = collect_closure_shared_names(&program.items);
        assert!(names.contains("counter"), "got {:?}", names);
        assert!(!names.contains("other"), "got {:?}", names);
    }

    #[test]
    fn collect_boxes_outer_assigned_closure_read_names() {
        let source = r#"
fn main() {
  let flag = false;
  let mtx = Mutex.new();
  let waiter = () => {
    while (!flag) {
      mtx.wait();
    }
  };
  flag = true;
}
"#;
        let mut parser = Parser::new(source).expect("lexer should not fail");
        let program = parser.parse().unwrap();
        let names = collect_closure_shared_names(&program.items);
        // Assigned outside + read inside the closure: must be boxed, or
        // the closure observes a stale by-value copy forever.
        assert!(names.contains("flag"), "got {:?}", names);
        // Read-only capture: no outer assignment, stays by-value.
        assert!(!names.contains("mtx"), "got {:?}", names);
    }
}
