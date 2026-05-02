/**
 * Compiler driver - orchestrates the full compilation pipeline.
 *
 * Pipeline: Source → Lexer → Parser → Macro Expansion → Type Checker → Code Gen → Linker
 *
 * @author Ruyi Team
 * @date 2026-05-01
 */

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::codegen::CodeGenerator;
use crate::lexer::LexerError;
use crate::macro_expand::{expand_macros, MacroError, MacroRegistry};
use crate::parser::{ParseError, Parser as RuyiParser};
use crate::typechecker::{TypeCheckResult, TypeChecker};
use crate::parser::ast::Program;
use crate::typechecker::diagnostics::Diagnostic;

/// Errors that can occur during compilation.
#[derive(Debug)]
pub enum CompileError {
    Io(String),
    Lexer(String),
    Parser(ParseError),
    Macro(MacroError),
    TypeCheck(Vec<Diagnostic>),
    Codegen(String),
    Linker(String),
    ModuleNotFound(String),
}

impl std::fmt::Display for CompileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CompileError::Io(msg) => write!(f, "IO error: {}", msg),
            CompileError::Lexer(msg) => write!(f, "lexer error: {}", msg),
            CompileError::Parser(err) => write!(f, "parse error: {}", err),
            CompileError::Macro(err) => write!(f, "macro error: {}", err),
            CompileError::TypeCheck(diags) => {
                for diag in diags {
                    writeln!(f, "{}", diag)?;
                }
                Ok(())
            }
            CompileError::Codegen(msg) => write!(f, "codegen error: {}", msg),
            CompileError::Linker(msg) => write!(f, "linker error: {}", msg),
            CompileError::ModuleNotFound(path) => write!(f, "module not found: {}", path),
        }
    }
}

impl std::error::Error for CompileError {}

impl From<std::io::Error> for CompileError {
    fn from(e: std::io::Error) -> Self {
        CompileError::Io(e.to_string())
    }
}

impl From<ParseError> for CompileError {
    fn from(e: ParseError) -> Self {
        CompileError::Parser(e)
    }
}

impl From<LexerError> for CompileError {
    fn from(e: LexerError) -> Self {
        CompileError::Lexer(e.to_string())
    }
}

impl From<MacroError> for CompileError {
    fn from(e: MacroError) -> Self {
        CompileError::Macro(e)
    }
}

impl From<String> for CompileError {
    fn from(e: String) -> Self {
        CompileError::Codegen(e)
    }
}

/// Emit options for intermediate compilation results.
#[derive(Debug, Clone, Copy)]
pub enum EmitType {
    /// Emit native binary (default)
    Binary,
    /// Emit LLVM IR (.ll file)
    LlvmIr,
    /// Emit AST (for debugging)
    Ast,
    /// Emit typed AST (for debugging)
    TypedAst,
    /// Parse and type check only (no codegen)
    Check,
}

/// Optimization level.
#[derive(Debug, Clone, Copy, Default)]
pub enum OptLevel {
    #[default]
    O0,
    O1,
    O2,
}

/// Compilation options.
#[derive(Debug, Clone)]
pub struct CompileOptions {
    /// What to emit
    pub emit: EmitType,
    /// Optimization level
    pub opt_level: OptLevel,
    /// Target triple (e.g., "x86_64-unknown-linux-gnu")
    pub target: Option<String>,
    /// Output file path
    pub output: Option<PathBuf>,
    /// Input file path (for error reporting)
    pub input: PathBuf,
    /// Search paths for modules
    pub search_paths: Vec<PathBuf>,
}

impl Default for CompileOptions {
    fn default() -> Self {
        Self {
            emit: EmitType::Binary,
            opt_level: OptLevel::O0,
            target: None,
            output: None,
            input: PathBuf::from(""),
            search_paths: Vec::new(),
        }
    }
}

/// Result of successful compilation.
#[derive(Debug)]
pub struct CompileResult {
    /// LLVM IR string (if emitted)
    pub llvm_ir: Option<String>,
    /// Path to output file
    pub output_path: PathBuf,
}

/// Module resolver for handling import statements.
pub struct ModuleResolver {
    /// Search paths to look for modules
    search_paths: Vec<PathBuf>,
    /// Already loaded modules (path -> AST)
    loaded_modules: HashMap<PathBuf, Program>,
}

impl ModuleResolver {
    pub fn new(search_paths: Vec<PathBuf>) -> Self {
        Self {
            search_paths,
            loaded_modules: HashMap::new(),
        }
    }

    /// Add a search path
    pub fn add_search_path(&mut self, path: PathBuf) {
        self.search_paths.push(path);
    }

    /// Resolve a module path to an absolute path.
    /// Looks in stdlib/ directory and project directories.
    pub fn resolve(&self, source: &str) -> Result<PathBuf, CompileError> {
        // Remove quotes from the source path
        let module_name = source.trim_matches('"');

        // Check if it's an absolute path
        if module_name.starts_with('/') {
            let path = PathBuf::from(module_name);
            if path.exists() {
                return Ok(path);
            }
            return Err(CompileError::ModuleNotFound(source.to_string()));
        }

        // Try relative to current directory first
        let relative_path = PathBuf::from(format!("{}.ry", module_name));
        if relative_path.exists() {
            return Ok(relative_path);
        }

        // Try search paths
        for search_path in &self.search_paths {
            let candidate = search_path.join(&relative_path);
            if candidate.exists() {
                return Ok(candidate);
            }
        }

        // Try stdlib directory
        let stdlib_path = PathBuf::from("stdlib").join(&relative_path);
        if stdlib_path.exists() {
            return Ok(stdlib_path);
        }

        Err(CompileError::ModuleNotFound(source.to_string()))
    }

    /// Get the canonical path for a module (resolves symlinks etc)
    fn canonical_path(&self, path: &Path) -> PathBuf {
        path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
    }
}

/// Compiler driver - orchestrates the full compilation pipeline.
pub struct Driver {
    /// Module resolver
    resolver: ModuleResolver,
    /// Macro registry for macro expansion
    macro_registry: MacroRegistry,
}

impl Driver {
    pub fn new(search_paths: Vec<PathBuf>) -> Self {
        Self {
            resolver: ModuleResolver::new(search_paths),
            macro_registry: MacroRegistry::with_builtins(),
        }
    }

    /// Compile a source file with the given options.
    pub fn compile_file(&mut self, options: &CompileOptions) -> Result<CompileResult, CompileError> {
        let source = fs::read_to_string(&options.input)?;

        // Resolve modules and combine programs
        let program = self.resolve_modules(&source, &options.input)?;

        // Run through the full pipeline
        self.compile_program(program, options)
    }

    /// Compile a program string directly.
    pub fn compile_source(&mut self, source: &str, options: &CompileOptions) -> Result<CompileResult, CompileError> {
        let program = self.parse_source(source)?;
        self.compile_program(program, options)
    }

    /// Parse source to AST, handling imports.
    fn resolve_modules(&mut self, source: &str, input_path: &Path) -> Result<Program, CompileError> {
        // Parse the main file
        let mut parser = RuyiParser::new(source)?;
        let mut program = parser.parse()?;

        // Process imports recursively
        program = self.resolve_imports(program, input_path)?;

        Ok(program)
    }

    /// Recursively resolve imports in a program.
    fn resolve_imports(&mut self, mut program: Program, _input_path: &Path) -> Result<Program, CompileError> {
        let items_to_process: Vec<_> = program.items.clone();
        program.items.clear();

        for item in items_to_process {
            match item {
                crate::parser::ast::ModuleItem::Import(import_decl) => {
                    let resolved_path = self.resolver.resolve(&import_decl.source)?;
                    let canonical = self.resolver.canonical_path(&resolved_path);

                    // Check if already loaded
                    if !self.resolver.loaded_modules.contains_key(&canonical) {
                        let module_source = fs::read_to_string(&resolved_path)?;
                        let mut module_parser = RuyiParser::new(&module_source)?;
                        let mut module_ast = module_parser.parse()?;
                        module_ast = self.resolve_imports(module_ast, &resolved_path)?;
                        self.resolver.loaded_modules.insert(canonical.clone(), module_ast);
                    }

                    // For now, we just skip the import item since we're
                    // inlining everything. In a full implementation,
                    // we'd track what's imported and build a proper module table.
                }
                _ => {
                    program.items.push(item);
                }
            }
        }

        Ok(program)
    }

    /// Parse source to AST without processing imports.
    fn parse_source(&mut self, source: &str) -> Result<Program, CompileError> {
        let mut parser = RuyiParser::new(source)?;
        let program = parser.parse()?;
        Ok(program)
    }

    /// Run the full compilation pipeline on a program.
    pub fn compile_program(&mut self, program: Program, options: &CompileOptions) -> Result<CompileResult, CompileError> {
        // Phase 1: Lexing (already done in parser)
        // Phase 2: Parsing (already done)

        if matches!(options.emit, EmitType::Ast) {
            return Ok(CompileResult {
                llvm_ir: Some(format!("{:#?}", program)),
                output_path: options.output.clone().unwrap_or_else(|| PathBuf::from("ast.txt")),
            });
        }

        // Phase 3: Macro expansion
        let expanded = expand_macros(&program, &self.macro_registry)?;

        // Phase 4: Type checking
        let mut checker = TypeChecker::new();
        let type_result = checker.check(&expanded);

        if type_result.has_errors {
            return Err(CompileError::TypeCheck(type_result.diagnostics.clone()));
        }

        if matches!(options.emit, EmitType::TypedAst) {
            return Ok(CompileResult {
                llvm_ir: Some(format!("{:#?}", type_result.env)),
                output_path: options.output.clone().unwrap_or_else(|| PathBuf::from("typed_ast.txt")),
            });
        }

        if matches!(options.emit, EmitType::Check) {
            return Ok(CompileResult {
                llvm_ir: None,
                output_path: PathBuf::from(""), // No output for check mode
            });
        }

        // Phase 5: Code generation
        let context = inkwell::context::Context::create();
        let module_name = options
            .input
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("main");
        let generator = CodeGenerator::new(&context, module_name);

        generator.generate(&expanded)?;

        // Phase 6: Output
        let output_path = options.output.clone().unwrap_or_else(|| {
            match options.emit {
                EmitType::LlvmIr => options.input.with_extension("ll"),
                _ => options.input.with_extension(""),
            }
        });

        match options.emit {
            EmitType::LlvmIr => {
                generator.write_llvm_ir(&output_path)?;
                Ok(CompileResult {
                    llvm_ir: Some(generator.print_to_string()),
                    output_path,
                })
            }
            _ => {
                generator.compile_to_binary_with_opt(&output_path, options.opt_level)?;
                Ok(CompileResult {
                    llvm_ir: None,
                    output_path,
                })
            }
        }
    }

    /// Get type check result for a source file (for --check mode).
    pub fn type_check(&mut self, source: &str) -> Result<TypeCheckResult, CompileError> {
        let mut parser = RuyiParser::new(source)?;
        let program = parser.parse()?;
        let expanded = expand_macros(&program, &self.macro_registry)?;
        let mut checker = TypeChecker::new();
        Ok(checker.check(&expanded))
    }
}

/// Format a diagnostic for display with source location.
pub fn format_diagnostic(diag: &Diagnostic, file: &Path) -> String {
    let result = format!("{}: {}", file.display(), diag);
    result
}