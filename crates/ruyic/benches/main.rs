use criterion::{black_box, Criterion, Throughput};
use ruyic::codegen::CodeGenerator;
use ruyic::driver::{CompileOptions, Driver, EmitType, OptLevel};
use ruyic::lexer::Scanner;
use ruyic::macro_expand::{expand_macros, MacroRegistry};
use ruyic::parser::Parser;
use ruyic::typechecker::TypeChecker;
use std::fs;

const LEXER_SOURCE_SMALL: &str = r#"
let x = 42;
let y = "hello";
fn add(a, b) { return a + b; }
"#;

const LEXER_SOURCE_MEDIUM: &str = r#"
let x = 42;
let y = "hello";
let z = [1, 2, 3, 4, 5];
fn add(a, b) { return a + b; }
fn multiply(a, b) { return a * b; }
fn divide(a, b) { if b == 0 { return 0; } return a / b; }
class Point { let x: num; let y: num; }
trait Serializable { fn serialize(): str; }
"#;

const LEXER_SOURCE_LARGE: &str = r#"
let x = 42;
let y = "hello";
let z = [1, 2, 3, 4, 5];
let a = { foo: 1, bar: 2, baz: 3 };
fn add(a, b) { return a + b; }
fn multiply(a, b) { return a * b; }
fn divide(a, b) { if b == 0 { return 0; } return a / b; }
fn subtract(a, b) { return a - b; }
fn modulo(a, b) { return a % b; }
class Point { let x: num; let y: num; }
class Circle { let radius: num; fn area() { return 3.14159 * self.radius * self.radius; } }
trait Serializable { fn serialize(): str; }
trait Deserializable { fn deserialize(data: str); }
impl Serializable for Point { fn serialize() { return "{}"; } }
"#;

const PARSER_SOURCE_SMALL: &str = r#"
let x = 42;
let y = "hello";
fn add(a, b) { return a + b; }
"#;

const PARSER_SOURCE_MEDIUM: &str = r#"
let x = 42;
let y = "hello";
let z = [1, 2, 3, 4, 5];
fn add(a, b) { return a + b; }
fn multiply(a, b) { return a * b; }
fn divide(a, b) { if b == 0 { return 0; } return a / b; }
class Point { let x: num; let y: num; }
trait Serializable { fn serialize(): str; }
"#;

const PARSER_SOURCE_LARGE: &str = r#"
let x = 42;
let y = "hello";
let z = [1, 2, 3, 4, 5];
let a = { foo: 1, bar: 2, baz: 3 };
fn add(a, b) { return a + b; }
fn multiply(a, b) { return a * b; }
fn divide(a, b) { if b == 0 { return 0; } return a / b; }
fn subtract(a, b) { return a - b; }
fn modulo(a, b) { return a % b; }
class Point { let x: num; let y: num; }
class Circle { let radius: num; fn area() { return 3.14159 * self.radius * self.radius; } }
trait Serializable { fn serialize(): str; }
trait Deserializable { fn deserialize(data: str); }
impl Serializable for Point { fn serialize() { return "{}"; } }
"#;

const TYPECHECK_SOURCE_SMALL: &str = r#"
let x = 42;
let y = "hello";
fn add(a: num, b: num): num { return a + b; }
"#;

const TYPECHECK_SOURCE_MEDIUM: &str = r#"
let x = 42;
let y = "hello";
let z: [num] = [1, 2, 3, 4, 5];
fn add(a: num, b: num): num { return a + b; }
fn multiply(a: num, b: num): num { return a * b; }
fn divide(a: num, b: num): num { if b == 0 { return 0; } return a / b; }
class Point { let x: num; let y: num; }
trait Serializable { fn serialize(): str; }
"#;

const TYPECHECK_SOURCE_LARGE: &str = r#"
let x = 42;
let y = "hello";
let z: [num] = [1, 2, 3, 4, 5];
let a: { foo: num, bar: num, baz: num } = { foo: 1, bar: 2, baz: 3 };
fn add(a: num, b: num): num { return a + b; }
fn multiply(a: num, b: num): num { return a * b; }
fn divide(a: num, b: num): num { if b == 0 { return 0; } return a / b; }
fn subtract(a: num, b: num): num { return a - b; }
fn modulo(a: num, b: num): num { return a % b; }
class Point { let x: num; let y: num; }
class Circle { let radius: num; fn area(): num { return 3.14159 * self.radius * self.radius; } }
trait Serializable { fn serialize(): str; }
trait Deserializable { fn deserialize(data: str); }
impl Serializable for Point { fn serialize(): str { return "{}"; } }
"#;

const CODEGEN_SOURCE_SMALL: &str = r#"
let x = 42;
let y = 10;
fn add(a, b) { return a + b; }
fn main() { return add(x, y); }
"#;

const CODEGEN_SOURCE_MEDIUM: &str = r#"
let x = 42;
let y = "hello";
let z = [1, 2, 3, 4, 5];
fn add(a, b) { return a + b; }
fn multiply(a, b) { return a * b; }
fn divide(a, b) { if b == 0 { return 0; } return a / b; }
fn main() { return add(multiply(x, 2), divide(y.len(), 2)); }
"#;

const CODEGEN_SOURCE_LARGE: &str = r#"
let x = 42;
let y = "hello";
let z = [1, 2, 3, 4, 5];
let a = { foo: 1, bar: 2, baz: 3 };
fn add(a, b) { return a + b; }
fn multiply(a, b) { return a * b; }
fn divide(a, b) { if b == 0 { return 0; } return a / b; }
fn subtract(a, b) { return a - b; }
fn modulo(a, b) { return a % b; }
fn main() { return add(multiply(x, 2), divide(z[0], 2)); }
"#;

fn compile_time_impl(source: &str, opt_level: OptLevel) -> std::time::Duration {
    let temp_dir = std::env::temp_dir();
    let input_path = temp_dir.join("bench_input.ry");
    let output_path = temp_dir.join("bench_output");

    fs::write(&input_path, source).unwrap();

    let options = CompileOptions {
        emit: EmitType::Binary,
        opt_level,
        target: None,
        output: Some(output_path.clone()),
        input: input_path.clone(),
        search_paths: vec![],
    };

    let start = std::time::Instant::now();
    let mut driver = Driver::new(vec![]);
    let result = driver.compile_file(&options);
    let duration = start.elapsed();

    if result.is_ok() {
        let _ = fs::remove_file(output_path);
    }
    let _ = fs::remove_file(input_path);

    duration
}

fn main() {
    let mut c = Criterion::default();

    let mut group = c.benchmark_group("lexer_small");
    group.throughput(Throughput::Bytes(LEXER_SOURCE_SMALL.len() as u64));
    group.bench_function("scanner", |b| {
        b.iter(|| {
            let mut scanner = Scanner::new(black_box(LEXER_SOURCE_SMALL));
            while scanner.next_token().unwrap().token != ruyic::lexer::Token::Eof {}
        });
    });
    group.finish();

    let mut group = c.benchmark_group("lexer_medium");
    group.throughput(Throughput::Bytes(LEXER_SOURCE_MEDIUM.len() as u64));
    group.bench_function("scanner", |b| {
        b.iter(|| {
            let mut scanner = Scanner::new(black_box(LEXER_SOURCE_MEDIUM));
            while scanner.next_token().unwrap().token != ruyic::lexer::Token::Eof {}
        });
    });
    group.finish();

    let mut group = c.benchmark_group("lexer_large");
    group.throughput(Throughput::Bytes(LEXER_SOURCE_LARGE.len() as u64));
    group.bench_function("scanner", |b| {
        b.iter(|| {
            let mut scanner = Scanner::new(black_box(LEXER_SOURCE_LARGE));
            while scanner.next_token().unwrap().token != ruyic::lexer::Token::Eof {}
        });
    });
    group.finish();

    let mut group = c.benchmark_group("parser_small");
    group.throughput(Throughput::Bytes(PARSER_SOURCE_SMALL.len() as u64));
    group.bench_function("parser", |b| {
        b.iter(|| {
            let mut parser = Parser::new(black_box(PARSER_SOURCE_SMALL)).unwrap();
            parser.parse().unwrap();
        });
    });
    group.finish();

    let mut group = c.benchmark_group("parser_medium");
    group.throughput(Throughput::Bytes(PARSER_SOURCE_MEDIUM.len() as u64));
    group.bench_function("parser", |b| {
        b.iter(|| {
            let mut parser = Parser::new(black_box(PARSER_SOURCE_MEDIUM)).unwrap();
            parser.parse().unwrap();
        });
    });
    group.finish();

    let mut group = c.benchmark_group("parser_large");
    group.throughput(Throughput::Bytes(PARSER_SOURCE_LARGE.len() as u64));
    group.bench_function("parser", |b| {
        b.iter(|| {
            let mut parser = Parser::new(black_box(PARSER_SOURCE_LARGE)).unwrap();
            parser.parse().unwrap();
        });
    });
    group.finish();

    let mut group = c.benchmark_group("typecheck_small");
    group.throughput(Throughput::Bytes(TYPECHECK_SOURCE_SMALL.len() as u64));
    group.bench_function("typecheck", |b| {
        b.iter(|| {
            let mut parser = Parser::new(black_box(TYPECHECK_SOURCE_SMALL)).unwrap();
            let ast = parser.parse().unwrap();
            let registry = MacroRegistry::with_builtins();
            let expanded = expand_macros(&ast, &registry).unwrap();
            let mut checker = TypeChecker::new();
            checker.check(&expanded);
        });
    });
    group.finish();

    let mut group = c.benchmark_group("typecheck_medium");
    group.throughput(Throughput::Bytes(TYPECHECK_SOURCE_MEDIUM.len() as u64));
    group.bench_function("typecheck", |b| {
        b.iter(|| {
            let mut parser = Parser::new(black_box(TYPECHECK_SOURCE_MEDIUM)).unwrap();
            let ast = parser.parse().unwrap();
            let registry = MacroRegistry::with_builtins();
            let expanded = expand_macros(&ast, &registry).unwrap();
            let mut checker = TypeChecker::new();
            checker.check(&expanded);
        });
    });
    group.finish();

    let mut group = c.benchmark_group("typecheck_large");
    group.throughput(Throughput::Bytes(TYPECHECK_SOURCE_LARGE.len() as u64));
    group.bench_function("typecheck", |b| {
        b.iter(|| {
            let mut parser = Parser::new(black_box(TYPECHECK_SOURCE_LARGE)).unwrap();
            let ast = parser.parse().unwrap();
            let registry = MacroRegistry::with_builtins();
            let expanded = expand_macros(&ast, &registry).unwrap();
            let mut checker = TypeChecker::new();
            checker.check(&expanded);
        });
    });
    group.finish();

    let mut group = c.benchmark_group("codegen_small");
    group.throughput(Throughput::Bytes(CODEGEN_SOURCE_SMALL.len() as u64));
    group.bench_function("codegen", |b| {
        b.iter(|| {
            let mut parser = Parser::new(black_box(CODEGEN_SOURCE_SMALL)).unwrap();
            let ast = parser.parse().unwrap();
            let registry = MacroRegistry::with_builtins();
            let expanded = expand_macros(&ast, &registry).unwrap();
            let context = inkwell::context::Context::create();
            let generator = CodeGenerator::new(&context, "bench");
            generator.generate(&expanded).unwrap();
        });
    });
    group.finish();

    let mut group = c.benchmark_group("codegen_medium");
    group.throughput(Throughput::Bytes(CODEGEN_SOURCE_MEDIUM.len() as u64));
    group.bench_function("codegen", |b| {
        b.iter(|| {
            let mut parser = Parser::new(black_box(CODEGEN_SOURCE_MEDIUM)).unwrap();
            let ast = parser.parse().unwrap();
            let registry = MacroRegistry::with_builtins();
            let expanded = expand_macros(&ast, &registry).unwrap();
            let context = inkwell::context::Context::create();
            let generator = CodeGenerator::new(&context, "bench");
            generator.generate(&expanded).unwrap();
        });
    });
    group.finish();

    let mut group = c.benchmark_group("codegen_large");
    group.throughput(Throughput::Bytes(CODEGEN_SOURCE_LARGE.len() as u64));
    group.bench_function("codegen", |b| {
        b.iter(|| {
            let mut parser = Parser::new(black_box(CODEGEN_SOURCE_LARGE)).unwrap();
            let ast = parser.parse().unwrap();
            let registry = MacroRegistry::with_builtins();
            let expanded = expand_macros(&ast, &registry).unwrap();
            let context = inkwell::context::Context::create();
            let generator = CodeGenerator::new(&context, "bench");
            generator.generate(&expanded).unwrap();
        });
    });
    group.finish();

    let compile_source = r#"
let x = 42;
let y = "hello";
let z = [1, 2, 3, 4, 5];
fn add(a, b) { return a + b; }
fn multiply(a, b) { return a * b; }
fn divide(a, b) { if b == 0 { return 0; } return a / b; }
fn main() { return add(multiply(x, 2), divide(z[0], 2)); }
"#;

    let mut group = c.benchmark_group("compile_time");
    group.throughput(Throughput::Bytes(compile_source.len() as u64));
    group.bench_function("O0", |b| {
        b.iter(|| compile_time_impl(black_box(compile_source), OptLevel::O0));
    });
    group.bench_function("O1", |b| {
        b.iter(|| compile_time_impl(black_box(compile_source), OptLevel::O1));
    });
    group.bench_function("O2", |b| {
        b.iter(|| compile_time_impl(black_box(compile_source), OptLevel::O2));
    });
    group.finish();
}
