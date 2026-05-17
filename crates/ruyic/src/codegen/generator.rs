/**
 * Main LLVM IR generator for Ruyi.
 *
 * Orchestrates the code generation pipeline: types, expressions,
 * statements, declarations, and built-in functions.
 *
 * @author Ruyi Team
 * @date 2026-05-01
 */
use std::collections::HashMap;

use crate::driver::OptLevel;
use inkwell::builder::Builder;
use inkwell::context::Context;
use inkwell::module::Module;
use inkwell::targets::{
    CodeModel, FileType, InitializationConfig, RelocMode, Target, TargetMachine,
};
use inkwell::values::{BasicValueEnum, FunctionValue};
use inkwell::OptimizationLevel;

fn opt_level_to_inkwell(level: OptLevel) -> OptimizationLevel {
    match level {
        OptLevel::O0 => OptimizationLevel::None,
        OptLevel::O1 => OptimizationLevel::Less,
        OptLevel::O2 => OptimizationLevel::Aggressive,
    }
}

use super::builtins::declare_builtins;
use super::decl::compile_declaration;
use super::monomorph::{MonomorphizationContext, MonomorphizedFunction};
use super::stmt::compile_block;
use super::types::{function_type_from_ruyi, ruyi_type_to_llvm};
use crate::parser::ast::Program;
use crate::typechecker::generics::MonomorphizationTracker;
use crate::typechecker::types::Type;

/// Extract a `Declaration` reference from a `ModuleItem`, handling both
/// direct declarations and exported declarations.
fn extract_declaration(item: &crate::parser::ast::ModuleItem) -> Option<&crate::parser::ast::Declaration> {
    match item {
        crate::parser::ast::ModuleItem::Declaration(decl) => Some(decl),
        crate::parser::ast::ModuleItem::Export(
            crate::parser::ast::ExportDecl::Declaration(decl),
        ) => Some(decl),
        _ => None,
    }
}

pub struct TryContext<'ctx> {
    pub exception_ptr: inkwell::values::PointerValue<'ctx>,
    pub catch_bb: Option<inkwell::basic_block::BasicBlock<'ctx>>,
    pub finally_bb: Option<inkwell::basic_block::BasicBlock<'ctx>>,
    pub merge_bb: inkwell::basic_block::BasicBlock<'ctx>,
    pub landing_pad_bb: Option<inkwell::basic_block::BasicBlock<'ctx>>,
}

/// Context for code generation, holding LLVM constructs and variable mappings.
pub struct CodegenContext<'ctx, 'm> {
    pub context: &'ctx Context,
    pub module: &'m Module<'ctx>,
    pub builder: Builder<'ctx>,
    pub variables: HashMap<String, (inkwell::values::PointerValue<'ctx>, Type)>,
    pub current_function: Option<FunctionValue<'ctx>>,
    pub loop_stack: Vec<(
        inkwell::basic_block::BasicBlock<'ctx>,
        inkwell::basic_block::BasicBlock<'ctx>,
    )>,
    pub gc_roots: Vec<Vec<(inkwell::values::PointerValue<'ctx>, Type)>>,
    /// Async state machine support: pointer to the state struct's state field (i32*)
    pub async_state_field_ptr: Option<inkwell::values::PointerValue<'ctx>>,
    /// Async state machine support: pointer to the state struct's result field
    pub async_result_ptr: Option<inkwell::values::PointerValue<'ctx>>,
    /// Async state machine support: basic block to jump to instead of returning
    pub async_return_bb: Option<inkwell::basic_block::BasicBlock<'ctx>>,
    /// Async state machine support: waker pointer for await expressions
    pub waker_ptr: Option<inkwell::values::PointerValue<'ctx>>,
    pub try_stack: Vec<TryContext<'ctx>>,
    pub class_fields: HashMap<String, Vec<(String, Type)>>,
    pub class_struct_types: HashMap<String, inkwell::types::StructType<'ctx>>,
    pub enum_struct_types: HashMap<String, inkwell::types::StructType<'ctx>>,
    /// Maps child class name to parent class name (for super.new() support)
    pub class_extends: HashMap<String, String>,
    /// Counter for generating unique anonymous arrow function names.
    pub arrow_counter: u64,
    pub function_types: HashMap<String, Type>,
    /// Maps function name to (rest_param_index, element_type) for rest parameter handling.
    pub rest_params: HashMap<String, (usize, Type)>,
    /// Tracks the return type of the current function for null sentinel handling.
    pub current_return_type: Option<Type>,
    /// Tracks the expected type for the current expression being compiled (for null literal handling).
    pub expected_expr_type: Option<Type>,
    /// When true, codegen errors for class/trait/impl declarations are logged as warnings
    /// instead of failing. Used when compiling stdlib modules that may use unsupported patterns.
    pub allow_partial_codegen: bool,
}

impl<'ctx, 'm> CodegenContext<'ctx, 'm> {
    pub fn new(context: &'ctx Context, module: &'m Module<'ctx>, builder: Builder<'ctx>) -> Self {
        Self {
            context,
            module,
            builder,
            variables: HashMap::new(),
            current_function: None,
            loop_stack: Vec::new(),
            gc_roots: Vec::new(),
            async_state_field_ptr: None,
            async_result_ptr: None,
            async_return_bb: None,
            waker_ptr: None,
            try_stack: Vec::new(),
            class_fields: HashMap::new(),
            class_struct_types: HashMap::new(),
            enum_struct_types: HashMap::new(),
            class_extends: HashMap::new(),
            arrow_counter: 0,
            function_types: HashMap::new(),
            rest_params: HashMap::new(),
            current_return_type: None,
            expected_expr_type: None,
            allow_partial_codegen: false,
        }
    }

    /// Push a new GC root scope for a function.
    pub fn push_gc_root_scope(&mut self) {
        self.gc_roots.push(Vec::new());
    }

    /// Emit `ruyi_gc_remove_root` calls for every root in the current scope.
    pub fn emit_gc_root_removals(&self) {
        if let Some(roots) = self.gc_roots.last() {
            for (ptr, ty) in roots {
                let llvm_ty = super::types::ruyi_type_to_llvm(self.context, ty);
                if llvm_ty.is_pointer_type() {
                    let loaded = self
                        .builder
                        .build_load(*ptr, "root_val")
                        .into_pointer_value();
                    super::builtins::build_gc_remove_root(&self.builder, &self.module, loaded);
                }
            }
        }
    }

    /// Pop the current GC root scope.
    pub fn pop_gc_root_scope(&mut self) {
        self.gc_roots.pop();
    }

    /// Register a local variable as a GC root and emit `ruyi_gc_add_root`.
    pub fn add_gc_root(&mut self, ptr: inkwell::values::PointerValue<'ctx>, ty: Type) {
        if let Some(scope) = self.gc_roots.last_mut() {
            scope.push((ptr, ty.clone()));
            let llvm_ty = super::types::ruyi_type_to_llvm(self.context, &ty);
            if llvm_ty.is_pointer_type() {
                let loaded = self
                    .builder
                    .build_load(ptr, "root_val")
                    .into_pointer_value();
                super::builtins::build_gc_add_root(&self.builder, &self.module, loaded);
            }
        }
    }
}

/// Main code generator for Ruyi programs.
#[allow(dead_code)]
pub struct CodeGenerator<'ctx> {
    context: &'ctx Context,
    module: Module<'ctx>,
    builder: Builder<'ctx>,
    pub allow_partial_codegen: bool,
}

impl<'ctx> CodeGenerator<'ctx> {
    pub fn new(context: &'ctx Context, name: &str) -> Self {
        let module = context.create_module(name);
        let builder = context.create_builder();
        Self {
            context,
            module,
            builder,
            allow_partial_codegen: false,
        }
    }

    /// Generate LLVM IR from a typed AST program.
    pub fn generate(&self, program: &Program) -> Result<(), String> {
        self.generate_with_tracker(program, &MonomorphizationTracker::new())
    }

    /// Generate LLVM IR from a typed AST program with monomorphization tracker.
    pub fn generate_with_tracker(
        &self,
        program: &Program,
        tracker: &MonomorphizationTracker,
    ) -> Result<(), String> {
        let mut ctx =
            CodegenContext::new(self.context, &self.module, self.context.create_builder());
        ctx.allow_partial_codegen = self.allow_partial_codegen;

        declare_builtins(self.context, &self.module);

        // Generate monomorphized generic functions
        let mut mono_ctx = MonomorphizationContext::new();
        mono_ctx.populate_from_tracker(tracker);
        let mono_func_names: Vec<String> = mono_ctx.functions().keys().cloned().collect();
        for mangled_name in mono_func_names {
            if !mono_ctx.is_generated(&mangled_name) {
                if let Some(mono_func) = mono_ctx.get_function(&mangled_name) {
                    let mono_func = mono_func.clone();
                    compile_monomorphized_function(&mut ctx, &mono_func)?;
                    mono_ctx.mark_generated(&mangled_name);
                }
            }
        }

        let has_async_main = program.items.iter().any(|item| {
            if let crate::parser::ast::ModuleItem::Declaration(
                crate::parser::ast::Declaration::Function { name, is_async, .. },
            ) = item
            {
                name == "main" && *is_async
            } else {
                false
            }
        });

        let llvm_main_name = "main";
        let i32_ty = self.context.i32_type();
        let main_fn = ctx
            .module
            .add_function(llvm_main_name, i32_ty.fn_type(&[], false), None);
        let entry_bb = ctx.context.append_basic_block(main_fn, "entry");
        ctx.builder.position_at_end(entry_bb);
        ctx.current_function = Some(main_fn);

        // Save entry block position before compiling declarations
        let main_entry_bb = entry_bb;

        // Collect the main function body to compile into the entry point
        let mut main_body: Option<&[crate::parser::ast::Statement]> = None;
        let mut main_params: Option<&[crate::parser::ast::Param]> = None;

        for item in &program.items {
            if let Some(decl) = extract_declaration(item) {
                if let crate::parser::ast::Declaration::Function {
                    name,
                    params,
                    return_type,
                    is_async,
                    ..
                } = decl
                {
                    if name == "main" && !*is_async {
                        if let crate::parser::ast::Declaration::Function { params, body, .. } = decl
                        {
                            main_body = Some(body);
                            main_params = Some(params);
                        }
                    } else if !*is_async {
                        super::decl::predeclare_function(
                            &mut ctx,
                            name,
                            params,
                            return_type.as_ref(),
                        );
                    }
                }
            }
        }

        // First pass: predeclare class struct types and method signatures for forward references
        for item in &program.items {
            if let crate::parser::ast::ModuleItem::Declaration(
                crate::parser::ast::Declaration::Class { name, body, .. },
            ) = item
            {
                // Predeclare struct type
                let mut fields: Vec<(String, crate::typechecker::types::Type)> = Vec::new();
                for element in body {
                    if let crate::parser::ast::ClassElement::Field {
                        name: prop_name,
                        ty,
                        is_static: false,
                        ..
                    } = element
                    {
                        if let crate::parser::ast::PropertyName::Ident(n) = prop_name {
                            let field_ty = ty
                                .as_ref()
                                .map(crate::typechecker::types::Type::from_annotation)
                                .unwrap_or(crate::typechecker::types::Type::Dynamic);
                            fields.push((n.clone(), field_ty));
                        }
                    }
                }
                let field_types: Vec<_> = fields
                    .iter()
                    .map(|(_, ty)| super::types::ruyi_type_to_llvm(ctx.context, ty))
                    .collect();
                let struct_type = ctx.context.struct_type(&field_types, false);
                ctx.class_struct_types.insert(name.clone(), struct_type);
                ctx.class_fields.insert(name.clone(), fields);

                // Predeclare methods
                for element in body {
                    if let crate::parser::ast::ClassElement::Method {
                        name: prop_name,
                        params,
                        return_type,
                        is_static: false,
                        is_async: false,
                        ..
                    } = element
                    {
                        if let crate::parser::ast::PropertyName::Ident(method) = prop_name {
                            let method_name = format!("{}_{}", name, method);
                            let mut method_params = vec![crate::parser::ast::Param {
                                pattern: crate::parser::ast::Pattern::Identifier(
                                    "self".to_string(),
                                ),
                                ty: Some(crate::parser::ast::TypeAnnotation::Identifier(
                                    name.clone(),
                                )),
                                init: None,
                                is_rest: false,
                            }];
                            method_params.extend(
                                params
                                    .iter()
                                    .filter(|p| !matches!(&p.pattern, crate::parser::ast::Pattern::Identifier(n) if n == "self"))
                                    .cloned(),
                            );
                            super::decl::predeclare_function(
                                &mut ctx,
                                &method_name,
                                &method_params,
                                return_type.as_ref(),
                            );
                        }
                    }
                }
            }
        }

        for item in &program.items {
            match item {
                crate::parser::ast::ModuleItem::Declaration(decl) => {
                    // Skip main function - it will be compiled into the entry point
                    if let crate::parser::ast::Declaration::Function { name, is_async, .. } = decl {
                        if name == "main" && !*is_async {
                            continue;
                        }
                    }
                    if let Err(_e) = compile_declaration(&mut ctx, decl) {
                        match decl {
                            crate::parser::ast::Declaration::Class { .. }
                            | crate::parser::ast::Declaration::Impl { .. }
                            | crate::parser::ast::Declaration::Trait { .. } => {
                                if !ctx.allow_partial_codegen {
                                    return Err(format!("codegen error: {}", _e));
                                }
                            }
                            _ => return Err(format!("codegen error: {}", _e)),
                        }
                    }
                }
                crate::parser::ast::ModuleItem::Export(
                    crate::parser::ast::ExportDecl::Declaration(decl),
                ) => {
                    if let crate::parser::ast::Declaration::Function { name, is_async, .. } = decl {
                        if name == "main" && !*is_async {
                            continue;
                        }
                    }
                    if let Err(_e) = compile_declaration(&mut ctx, decl) {
                        if !ctx.allow_partial_codegen {
                            return Err(format!("codegen error: {}", _e));
                        }
                    }
                }
                crate::parser::ast::ModuleItem::Statement(stmt) => {
                    compile_block(&mut ctx, std::slice::from_ref(stmt))?;
                }
                _ => {}
            }
        }

        // Restore builder position to entry main block
        ctx.builder.position_at_end(main_entry_bb);
        ctx.current_function = Some(main_fn);

        // Declare main function parameters as local variables
        if let Some(params) = main_params {
            for (i, param) in params.iter().enumerate() {
                let param_name = match &param.pattern {
                    crate::parser::ast::Pattern::Identifier(n) => n.clone(),
                    _ => format!("param_{}", i),
                };
                let param_ty = param
                    .ty
                    .as_ref()
                    .map(Type::from_annotation)
                    .unwrap_or(Type::Dynamic);
                let llvm_ty = ruyi_type_to_llvm(ctx.context, &param_ty);
                let ptr = ctx.builder.build_alloca(llvm_ty, &param_name);
                // Main has no external params, so just initialize with zero
                let zero = match param_ty {
                    Type::Int => {
                        BasicValueEnum::IntValue(ctx.context.i64_type().const_int(0, true))
                    }
                    Type::Float => {
                        BasicValueEnum::FloatValue(ctx.context.f64_type().const_float(0.0))
                    }
                    Type::Bool => {
                        BasicValueEnum::IntValue(ctx.context.bool_type().const_int(0, false))
                    }
                    _ => BasicValueEnum::PointerValue(
                        ctx.context
                            .i8_type()
                            .ptr_type(Default::default())
                            .const_null(),
                    ),
                };
                ctx.builder.build_store(ptr, zero);
                if crate::codegen::builtins::is_gc_managed(&param_ty) {
                    ctx.add_gc_root(ptr, param_ty.clone());
                }
                ctx.variables.insert(param_name, (ptr, param_ty));
            }
        }

        // Compile main function body
        if let Some(body) = main_body {
            compile_block(&mut ctx, body)?;
        }

        let current_bb = ctx.builder.get_insert_block().unwrap();
        if current_bb.get_terminator().is_none() {
            if has_async_main {
                if let Some(async_main) = ctx.module.get_function("_ruyi_async_main") {
                    let future_ptr = ctx
                        .builder
                        .build_call(async_main, &[], "async_main_call")
                        .try_as_basic_value()
                        .left()
                        .unwrap();
                    let spawn_fn = ctx
                        .module
                        .get_function("ruyi_spawn")
                        .expect("ruyi_spawn not declared");
                    ctx.builder
                        .build_call(spawn_fn, &[future_ptr.into()], "spawn_main");
                    let scheduler_fn = ctx
                        .module
                        .get_function("ruyi_run_scheduler")
                        .expect("ruyi_run_scheduler not declared");
                    ctx.builder.build_call(scheduler_fn, &[], "run_scheduler");
                }
            }
            let zero = BasicValueEnum::IntValue(i32_ty.const_int(0, false));
            ctx.builder.build_return(Some(&zero));
        }

        Ok(())
    }

    /// Get the generated LLVM module.
    pub fn module(&self) -> &Module<'ctx> {
        &self.module
    }

    /// Print the generated LLVM IR to a string.
    pub fn print_to_string(&self) -> String {
        self.module.print_to_string().to_string()
    }

    /// Write the generated LLVM IR to a file.
    pub fn write_llvm_ir(&self, path: &std::path::Path) -> Result<(), String> {
        self.module.print_to_file(path).map_err(|e| e.to_string())
    }

    /// Compile the generated LLVM IR to a native object file with optimization level.
    pub fn compile_to_object_with_opt(
        &self,
        path: &std::path::Path,
        opt_level: OptLevel,
    ) -> Result<(), String> {
        Target::initialize_native(&InitializationConfig::default())
            .map_err(|e| format!("Failed to initialize native target: {}", e))?;

        let triple = TargetMachine::get_default_triple();
        let target = Target::from_triple(&triple)
            .map_err(|e| format!("Failed to get target from triple: {}", e))?;

        let machine = target
            .create_target_machine(
                &triple,
                "generic",
                "",
                opt_level_to_inkwell(opt_level),
                RelocMode::Default,
                CodeModel::Default,
            )
            .ok_or("Failed to create target machine")?;

        machine
            .write_to_file(&self.module, FileType::Object, path)
            .map_err(|e| format!("Failed to write object file: {}", e))
    }

    /// Compile the generated LLVM IR to a native object file.
    pub fn compile_to_object(&self, path: &std::path::Path) -> Result<(), String> {
        self.compile_to_object_with_opt(path, OptLevel::O2)
    }

    /// Compile the generated LLVM IR to a native binary with optimization level.
    pub fn compile_to_binary_with_opt(
        &self,
        path: &std::path::Path,
        opt_level: OptLevel,
    ) -> Result<(), String> {
        let temp_obj = std::env::temp_dir().join("ruyi_temp.o");
        self.compile_to_object_with_opt(&temp_obj, opt_level)?;

        let runtime_lib = option_env!("RUYI_RUNTIME_LIB")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| {
                let debug_path = std::path::PathBuf::from("target/debug/libruyi_runtime.a");
                if debug_path.exists() {
                    debug_path
                } else {
                    std::path::PathBuf::from("target/release/libruyi_runtime.a")
                }
            });

        std::process::Command::new("cc")
            .arg(&temp_obj)
            .arg("-o")
            .arg(path)
            .arg(&runtime_lib)
            .arg("-lm")
            .arg(if std::env::consts::OS == "macos" {
                "-lc++"
            } else {
                "-lstdc++"
            })
            .status()
            .map_err(|e| format!("Failed to link binary: {}", e))?;

        let _ = std::fs::remove_file(&temp_obj);

        if std::process::Command::new("test")
            .arg("-f")
            .arg(path)
            .status()
            .map_err(|e| format!("Failed to check binary: {}", e))?
            .success()
        {
            Ok(())
        } else {
            Err("Linking failed".to_string())
        }
    }

    /// Compile the generated LLVM IR to a native binary.
    pub fn compile_to_binary(&self, path: &std::path::Path) -> Result<(), String> {
        self.compile_to_binary_with_opt(path, OptLevel::O2)
    }
}

/// Compiles a monomorphized generic function to LLVM IR.
///
/// Per spec Section 10.3, each specialization of a generic function
/// gets its own LLVM function with the mangled name.
fn compile_monomorphized_function<'ctx>(
    ctx: &mut CodegenContext<'ctx, '_>,
    mono_func: &MonomorphizedFunction,
) -> Result<(), String> {
    let fn_type =
        function_type_from_ruyi(ctx.context, &mono_func.param_types, &mono_func.return_type);
    let function = ctx
        .module
        .add_function(&mono_func.mangled_name, fn_type, None);

    let entry_bb = ctx.context.append_basic_block(function, "entry");
    let prev_function = ctx.current_function;
    let prev_return_type = ctx.current_return_type.clone();
    ctx.current_function = Some(function);
    ctx.current_return_type = Some(mono_func.return_type.clone());
    let prev_block = ctx.builder.get_insert_block();
    ctx.builder.position_at_end(entry_bb);

    // Allocate parameters
    let mut prev_vars = std::collections::HashMap::new();
    ctx.push_gc_root_scope();

    for (i, param_ty) in mono_func.param_types.iter().enumerate() {
        let param_name = format!("arg_{}", i);
        let llvm_ty = ruyi_type_to_llvm(ctx.context, param_ty);
        let ptr = ctx.builder.build_alloca(llvm_ty, &param_name);

        if let Some(param_value) = function.get_nth_param(i as u32) {
            ctx.builder.build_store(ptr, param_value);
        }

        if super::builtins::is_gc_managed(param_ty) {
            ctx.add_gc_root(ptr, param_ty.clone());
        }

        if let Some(old) = ctx
            .variables
            .insert(param_name.clone(), (ptr, param_ty.clone()))
        {
            prev_vars.insert(param_name, old);
        }
    }

    // Ensure the function has a terminator
    let current_bb = ctx.builder.get_insert_block().unwrap();
    if current_bb.get_terminator().is_none() {
        use inkwell::values::BasicValueEnum;
        ctx.emit_gc_root_removals();
        match &mono_func.return_type {
            Type::Void | Type::Never => {
                ctx.builder.build_return(None);
            }
            Type::Int => {
                let zero = ctx.context.i64_type().const_int(0, true);
                ctx.builder
                    .build_return(Some(&BasicValueEnum::IntValue(zero)));
            }
            Type::Float => {
                let zero = ctx.context.f64_type().const_float(0.0);
                ctx.builder
                    .build_return(Some(&BasicValueEnum::FloatValue(zero)));
            }
            Type::Bool => {
                let zero = ctx.context.bool_type().const_int(0, false);
                ctx.builder
                    .build_return(Some(&BasicValueEnum::IntValue(zero)));
            }
            _ => {
                let null_ptr = ctx
                    .context
                    .i8_type()
                    .ptr_type(Default::default())
                    .const_null();
                ctx.builder
                    .build_return(Some(&BasicValueEnum::PointerValue(null_ptr)));
            }
        }
    }

    ctx.pop_gc_root_scope();

    // Restore previous state
    ctx.current_function = prev_function;
    ctx.current_return_type = prev_return_type;
    if let Some(block) = prev_block {
        ctx.builder.position_at_end(block);
    }
    for (name, old) in prev_vars {
        ctx.variables.insert(name, old);
    }

    Ok(())
}
