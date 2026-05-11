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
use crate::parser::ast::Program;
use crate::parser::{ParseError, Parser as RuyiParser};
use crate::typechecker::diagnostics::Diagnostic;
use crate::typechecker::{TypeCheckResult, TypeChecker};

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
    /// RUYI_HOME directory for stdlib resolution
    ruyi_home: Option<PathBuf>,
}

impl ModuleResolver {
    pub fn new(search_paths: Vec<PathBuf>) -> Self {
        Self {
            search_paths,
            loaded_modules: HashMap::new(),
            ruyi_home: std::env::var("RUYI_HOME").ok().map(PathBuf::from),
        }
    }

    /// Add a search path
    pub fn add_search_path(&mut self, path: PathBuf) {
        self.search_paths.push(path);
    }

    /// Resolve a module path to an absolute path.
    /// Looks in RUYI_HOME/stdlib, search paths, and project directories.
    pub fn resolve(
        &self,
        source: &str,
        base_path: Option<&Path>,
    ) -> Result<PathBuf, CompileError> {
        // Remove quotes from the source path
        let module_name = source.trim_matches('"');

        // Handle relative paths starting with ./ or ../
        if module_name.starts_with("./") || module_name.starts_with("../") {
            if let Some(base) = base_path {
                let base_dir = base.parent().unwrap_or(base);
                let candidate = base_dir.join(format!("{}.ry", module_name));
                if candidate.exists() {
                    return Ok(candidate);
                }
                return Err(CompileError::ModuleNotFound(source.to_string()));
            }
        }

        // Check if it's an absolute path
        if module_name.starts_with('/') {
            let path = PathBuf::from(module_name);
            if path.exists() {
                return Ok(path);
            }
            return Err(CompileError::ModuleNotFound(source.to_string()));
        }

        let relative_path = PathBuf::from(format!("{}.ry", module_name));

        // Try RUYI_HOME/stdlib first (for stdlib modules)
        if let Some(ref home) = self.ruyi_home {
            let stdlib_candidate = home.join("stdlib").join(&relative_path);
            if stdlib_candidate.exists() {
                return Ok(stdlib_candidate);
            }
        }

        // Try relative to current directory
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

        // Fallback: try local stdlib/ directory (for development)
        let local_stdlib = PathBuf::from("stdlib").join(&relative_path);
        if local_stdlib.exists() {
            return Ok(local_stdlib);
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
    pub fn compile_file(
        &mut self,
        options: &CompileOptions,
    ) -> Result<CompileResult, CompileError> {
        let source = fs::read_to_string(&options.input)?;

        // Resolve modules and combine programs
        let program = self.resolve_modules(&source, &options.input)?;

        // Run through the full pipeline
        self.compile_program(program, options)
    }

    /// Compile a program string directly.
    pub fn compile_source(
        &mut self,
        source: &str,
        options: &CompileOptions,
    ) -> Result<CompileResult, CompileError> {
        let program = self.parse_source(source)?;
        self.compile_program(program, options)
    }

    fn ensure_runtime_built() -> Result<(), CompileError> {
        let status = std::process::Command::new("cargo")
            .args([
                "build",
                "-p",
                "ruyi_runtime",
                "--lib",
                "--no-default-features",
            ])
            .status()
            .map_err(|e| CompileError::Io(format!("Failed to build runtime: {}", e)))?;

        if !status.success() {
            return Err(CompileError::Linker(
                "Failed to build ruyi_runtime".to_string(),
            ));
        }
        Ok(())
    }

    /// Parse source to AST, handling imports.
    fn resolve_modules(
        &mut self,
        source: &str,
        input_path: &Path,
    ) -> Result<Program, CompileError> {
        // Parse the main file
        let mut parser = RuyiParser::new(source)?;
        let mut program = parser.parse()?;

        // Auto-load stdlib modules (error, collections, etc.)
        self.auto_load_stdlib()?;

        // Process imports recursively
        program = self.resolve_imports(program, input_path)?;

        // Prepend auto-loaded stdlib modules before main program items
        let mut all_items: Vec<crate::parser::ast::ModuleItem> = Vec::new();
        for (_, module) in &self.resolver.loaded_modules {
            for module_item in &module.items {
                all_items.push(module_item.clone());
            }
        }
        all_items.extend(program.items);
        program.items = all_items;

        Ok(program)
    }

    /// Auto-load essential stdlib modules.
    fn auto_load_stdlib(&mut self) -> Result<(), CompileError> {
        let stdlib_modules = ["error", "core", "collections"];

        for module_name in &stdlib_modules {
            let module_path = PathBuf::from(format!("stdlib/{}.ry", module_name));
            if !module_path.exists() {
                continue;
            }

            let canonical = self.resolver.canonical_path(&module_path);
            if self.resolver.loaded_modules.contains_key(&canonical) {
                continue;
            }

            let module_source = fs::read_to_string(&module_path)?;
            let mut module_parser = RuyiParser::new(&module_source)?;
            let mut module_ast = module_parser.parse()?;
            module_ast = self.resolve_imports(module_ast, &module_path)?;
            self.resolver.loaded_modules.insert(canonical, module_ast);
        }

        Ok(())
    }

    /// Recursively resolve imports in a program.
    fn resolve_imports(
        &mut self,
        mut program: Program,
        input_path: &Path,
    ) -> Result<Program, CompileError> {
        let items_to_process: Vec<_> = program.items.clone();
        program.items.clear();

        for item in items_to_process {
            match item {
                crate::parser::ast::ModuleItem::Import(import_decl) => {
                    let resolved_path = self
                        .resolver
                        .resolve(&import_decl.source, Some(input_path))?;
                    let canonical = self.resolver.canonical_path(&resolved_path);

                    // Check if already loaded
                    if !self.resolver.loaded_modules.contains_key(&canonical) {
                        let module_source = fs::read_to_string(&resolved_path)?;
                        let mut module_parser = RuyiParser::new(&module_source)?;
                        let mut module_ast = module_parser.parse()?;
                        module_ast = self.resolve_imports(module_ast, &resolved_path)?;
                        self.resolver
                            .loaded_modules
                            .insert(canonical.clone(), module_ast);
                    }

                    // Merge imported module items into the main program.
                    // Unwrap exports so the typechecker can see the underlying declarations.
                    //
                    // Collect re-export sources first, before borrowing loaded_modules,
                    // to avoid borrow conflicts with self.resolve_imports.
                    let reexport_sources: Vec<(String, PathBuf)> = {
                        if let Some(module) = self.resolver.loaded_modules.get(&canonical) {
                            module
                                .items
                                .iter()
                                .filter_map(|item| {
                                    if let crate::parser::ast::ModuleItem::Export(export) = item {
                                        match export {
                                            crate::parser::ast::ExportDecl::ReExportAll {
                                                source,
                                            }
                                            | crate::parser::ast::ExportDecl::ReExportNamed {
                                                source,
                                                ..
                                            } => Some((source.clone(), canonical.clone())),
                                            _ => None,
                                        }
                                    } else {
                                        None
                                    }
                                })
                                .collect()
                        } else {
                            Vec::new()
                        }
                    };

                    // Process re-exports (outside the immutable borrow).
                    for (source, dir) in &reexport_sources {
                        let reexport_path = self.resolver.resolve(source, Some(dir))?;
                        let reexport_canonical =
                            self.resolver.canonical_path(&reexport_path);
                        if !self
                            .resolver
                            .loaded_modules
                            .contains_key(&reexport_canonical)
                        {
                            let module_source = fs::read_to_string(&reexport_path)?;
                            let mut module_parser = RuyiParser::new(&module_source)?;
                            let mut module_ast = module_parser.parse()?;
                            module_ast =
                                self.resolve_imports(module_ast, &reexport_path)?;
                            self.resolver
                                .loaded_modules
                                .insert(reexport_canonical.clone(), module_ast);
                        }
                        if let Some(reexport_module) = self
                            .resolver
                            .loaded_modules
                            .get(&reexport_canonical)
                        {
                            for reexport_item in &reexport_module.items {
                                Self::push_unwrapped(&mut program, reexport_item);
                            }
                        }
                    }

                    // Main merge and local bindings.
                    if let Some(module) = self.resolver.loaded_modules.get(&canonical) {
                        for module_item in &module.items {
                            Self::push_unwrapped(&mut program, module_item);
                        }

                        // Aliases: import { x as y } → const y = x;
                        for named_import in &import_decl.named {
                            if let Some(alias) = &named_import.alias {
                                program.items.push(
                                    crate::parser::ast::ModuleItem::Declaration(
                                        crate::parser::ast::Declaration::Const(vec![
                                            crate::parser::ast::Binding {
                                                pattern:
                                                    crate::parser::ast::Pattern::Identifier(
                                                        alias.clone(),
                                                    ),
                                                init: Some(Box::new(
                                                    crate::parser::ast::Expr::Identifier(
                                                        named_import.name.clone(),
                                                    ),
                                                )),
                                                ty: None,
                                            },
                                        ]),
                                    ),
                                );
                            }
                        }

                        // Namespace: import * as ns → const ns = { name1, name2, ... };
                        if let Some(ns) = &import_decl.namespace {
                            let mut props: Vec<crate::parser::ast::ObjectProperty> = Vec::new();
                            for module_item in &module.items {
                                Self::collect_export_names(module_item, &mut props);
                            }
                            if !props.is_empty() {
                                program.items.push(
                                    crate::parser::ast::ModuleItem::Declaration(
                                        crate::parser::ast::Declaration::Const(vec![
                                            crate::parser::ast::Binding {
                                                pattern:
                                                    crate::parser::ast::Pattern::Identifier(
                                                        ns.clone(),
                                                    ),
                                                init: Some(Box::new(
                                                    crate::parser::ast::Expr::ObjectLiteral(
                                                        props,
                                                    ),
                                                )),
                                                ty: None,
                                            },
                                        ]),
                                    ),
                                );
                            }
                        }
                    }
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
    pub fn compile_program(
        &mut self,
        program: Program,
        options: &CompileOptions,
    ) -> Result<CompileResult, CompileError> {
        // Phase 1: Lexing (already done in parser)
        // Phase 2: Parsing (already done)

        if matches!(options.emit, EmitType::Ast) {
            return Ok(CompileResult {
                llvm_ir: Some(format!("{:#?}", program)),
                output_path: options
                    .output
                    .clone()
                    .unwrap_or_else(|| PathBuf::from("ast.txt")),
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
                output_path: options
                    .output
                    .clone()
                    .unwrap_or_else(|| PathBuf::from("typed_ast.txt")),
            });
        }

        if matches!(options.emit, EmitType::Check) {
            return Ok(CompileResult {
                llvm_ir: None,
                output_path: PathBuf::from(""), // No output for check mode
            });
        }

        Self::ensure_runtime_built()?;

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
        let output_path = options
            .output
            .clone()
            .unwrap_or_else(|| match options.emit {
                EmitType::LlvmIr => {
                    let parent = options.input.parent().unwrap_or(std::path::Path::new("."));
                    let target_dir = parent.join("target");
                    std::fs::create_dir_all(&target_dir).ok();
                    let stem = options
                        .input
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("main");
                    target_dir.join(format!("{}.ll", stem))
                }
                _ => {
                    let parent = options.input.parent().unwrap_or(std::path::Path::new("."));
                    let target_dir = parent.join("target");
                    std::fs::create_dir_all(&target_dir).ok();
                    let stem = options
                        .input
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("main");
                    target_dir.join(stem)
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

    /// Push a module item to the program, unwrapping `Export` into the contained `Declaration`
    /// so the typechecker can see it (it skips `ModuleItem::Export` items).
    fn push_unwrapped(
        program: &mut Program,
        item: &crate::parser::ast::ModuleItem,
    ) {
        match item {
            crate::parser::ast::ModuleItem::Export(export) => match export {
                crate::parser::ast::ExportDecl::Declaration(decl) => {
                    program
                        .items
                        .push(crate::parser::ast::ModuleItem::Declaration(decl.clone()));
                }
                crate::parser::ast::ExportDecl::DefaultFunction {
                    name,
                    type_params,
                    params,
                    return_type,
                    body,
                    is_async,
                } => {
                    program.items.push(
                        crate::parser::ast::ModuleItem::Declaration(
                            crate::parser::ast::Declaration::Function {
                                name: name.clone(),
                                type_params: type_params.clone(),
                                params: params.clone(),
                                return_type: return_type.clone(),
                                body: body.clone(),
                                is_async: *is_async,
                            },
                        ),
                    );
                }
                crate::parser::ast::ExportDecl::DefaultClass {
                    name,
                    type_params,
                    extends,
                    body,
                    annotations,
                } => {
                    program.items.push(
                        crate::parser::ast::ModuleItem::Declaration(
                            crate::parser::ast::Declaration::Class {
                                name: name.clone(),
                                type_params: type_params.clone(),
                                extends: extends.clone(),
                                body: body.clone(),
                                annotations: annotations.clone(),
                            },
                        ),
                    );
                }
                // Named, ReExportAll, ReExportNamed, DefaultExpr are not declarations;
                // they are processed separately or left as-is.
                _ => {}
            },
            _ => {
                program.items.push(item.clone());
            }
        }
    }

    /// Collect export names from a module item into object properties (for namespace imports).
    fn collect_export_names(
        item: &crate::parser::ast::ModuleItem,
        props: &mut Vec<crate::parser::ast::ObjectProperty>,
    ) {
        if let crate::parser::ast::ModuleItem::Export(export) = item {
            let name = match export {
                crate::parser::ast::ExportDecl::Declaration(decl) => match decl {
                    crate::parser::ast::Declaration::Function { name, .. } => Some(name.clone()),
                    crate::parser::ast::Declaration::Class { name, .. } => Some(name.clone()),
                    crate::parser::ast::Declaration::Const(bindings)
                    | crate::parser::ast::Declaration::Let(bindings) => bindings.first().and_then(
                        |b| {
                            if let crate::parser::ast::Pattern::Identifier(n) = &b.pattern {
                                Some(n.clone())
                            } else {
                                None
                            }
                        },
                    ),
                    _ => None,
                },
                crate::parser::ast::ExportDecl::DefaultFunction { name, .. }
                | crate::parser::ast::ExportDecl::DefaultClass { name, .. } => Some(name.clone()),
                _ => None,
            };
            if let Some(n) = name {
                props.push(crate::parser::ast::ObjectProperty::Shorthand(n));
            }
        }
    }
}

/// Format a diagnostic for display with source location.
pub fn format_diagnostic(diag: &Diagnostic, file: &Path) -> String {
    let result = format!("{}: {}", file.display(), diag);
    result
}
