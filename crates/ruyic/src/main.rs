/**
 * Ruyi compiler driver.
 *
 * Command-line interface for compiling Ruyi source files to native binaries.
 * Pipeline: Source → Tokens → AST → Typed AST → LLVM IR → Native Binary
 *
 * @author Ruyi Team
 * @date 2026-05-01
 */
use std::path::PathBuf;
use std::process;

use clap::Parser;
use ruyic::cli::gc_mode::GcMode;
use ruyic::driver::{CompileOptions, Driver, EmitType, OptLevel};

#[derive(Parser, Debug)]
#[command(name = "ruyic")]
#[command(version = "0.5.11")]
#[command(about = "Ruyi compiler - compiles .ry source files to native binaries")]
struct Args {
    #[arg(help = "Input file to compile")]
    input: Option<PathBuf>,

    #[arg(short, long, help = "Output file path")]
    output: Option<PathBuf>,

    #[arg(short = 'O', help = "Optimization level (0, 1, 2)")]
    opt_level: Option<u8>,

    #[arg(long, help = "Target triple (e.g., x86_64-unknown-linux-gnu)")]
    target: Option<String>,

    #[arg(long, help = "Emit LLVM IR instead of compiling")]
    emit_llvm: bool,

    #[arg(long, help = "Emit AST (for debugging)")]
    emit_ast: bool,

    #[arg(long, help = "Emit typed AST (for debugging)")]
    emit_typed_ast: bool,

    #[arg(long, help = "Parse and type check only (no codegen)")]
    check: bool,

    #[arg(long, help = "Discover @test fn declarations and list them")]
    test: bool,

    #[arg(
        long,
        default_value = "stub",
        value_name = "MODE",
        help = "GC mode: 'stub' (default) or 'real'"
    )]
    gc: String,
}

fn main() {
    let args = Args::parse();

    if args.input.is_none() {
        println!("Ruyi compiler v0.1.0");
        println!("Usage: ruyic <input> [options]");
        println!("Run 'ruyic --help' for more information.");
        return;
    }

    let input = args.input.unwrap();

    // Build emit type from flags
    let emit = if args.emit_ast {
        EmitType::Ast
    } else if args.emit_typed_ast {
        EmitType::TypedAst
    } else if args.test {
        EmitType::Test
    } else if args.check {
        EmitType::Check
    } else if args.emit_llvm {
        EmitType::LlvmIr
    } else {
        EmitType::Binary
    };

    // Build optimization level
    let opt_level = match args.opt_level.unwrap_or(0) {
        0 => OptLevel::O0,
        1 => OptLevel::O1,
        2 => OptLevel::O2,
        _ => OptLevel::O0,
    };

    let gc_mode = match GcMode::parse(&args.gc) {
        Ok(mode) => mode,
        Err(err) => {
            eprintln!("error: {}", err);
            process::exit(2);
        }
    };

    let options = CompileOptions {
        emit,
        opt_level,
        target: args.target,
        output: args.output,
        input: input.clone(),
        search_paths: vec![],
        gc_mode,
    };

    let mut driver = Driver::new(vec![]);

    match driver.compile_file(&options) {
        Ok(result) => match emit {
            EmitType::Check => {
                println!("Type checking passed.");
            }
            EmitType::Ast | EmitType::TypedAst => {
                if let Some(ir) = result.llvm_ir {
                    println!("{}", ir);
                }
                println!("Output written to: {}", result.output_path.display());
            }
            EmitType::LlvmIr => {
                println!("LLVM IR written to: {}", result.output_path.display());
            }
            EmitType::Binary => {
                println!("Binary written to: {}", result.output_path.display());
            }
            EmitType::Test => {
                println!("Test discovery complete.");
            }
        },
        Err(e) => {
            eprintln!("error: {}", e);
            process::exit(1);
        }
    }
}
