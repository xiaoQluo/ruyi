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

use inkwell::builder::Builder;
use inkwell::context::Context;
use inkwell::module::Module;
use inkwell::targets::{CodeModel, FileType, InitializationConfig, RelocMode, Target, TargetMachine};
use inkwell::values::FunctionValue;
use inkwell::OptimizationLevel;
use crate::driver::OptLevel;

fn opt_level_to_inkwell(level: OptLevel) -> OptimizationLevel {
    match level {
        OptLevel::O0 => OptimizationLevel::None,
        OptLevel::O1 => OptimizationLevel::Less,
        OptLevel::O2 => OptimizationLevel::Aggressive,
    }
}

use crate::parser::ast::Program;
use crate::typechecker::generics::MonomorphizationTracker;
use crate::typechecker::types::Type;
use super::builtins::declare_builtins;
use super::decl::compile_declaration;
use super::monomorph::{MonomorphizationContext, MonomorphizedFunction};
use super::stmt::compile_block;
use super::types::{ruyi_type_to_llvm, function_type_from_ruyi};

/// Context for code generation, holding LLVM constructs and variable mappings.
pub struct CodegenContext<'ctx, 'm> {
    pub context: &'ctx Context,
    pub module: &'m Module<'ctx>,
    pub builder: Builder<'ctx>,
    pub variables: HashMap<String, (inkwell::values::PointerValue<'ctx>, Type)>,
    pub current_function: Option<FunctionValue<'ctx>>,
    pub loop_stack: Vec<(inkwell::basic_block::BasicBlock<'ctx>, inkwell::basic_block::BasicBlock<'ctx>)>,
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
        }
    }
}

/// Main code generator for Ruyi programs.
pub struct CodeGenerator<'ctx> {
    context: &'ctx Context,
    module: Module<'ctx>,
    builder: Builder<'ctx>,
}

impl<'ctx> CodeGenerator<'ctx> {
    pub fn new(context: &'ctx Context, name: &str) -> Self {
        let module = context.create_module(name);
        let builder = context.create_builder();
        Self { context, module, builder }
    }

    /// Generate LLVM IR from a typed AST program.
    pub fn generate(&self, program: &Program) -> Result<(), String> {
        self.generate_with_tracker(program, &MonomorphizationTracker::new())
    }

    /// Generate LLVM IR from a typed AST program with monomorphization tracker.
    pub fn generate_with_tracker(&self, program: &Program, tracker: &MonomorphizationTracker) -> Result<(), String> {
        let mut ctx = CodegenContext::new(self.context, &self.module, self.context.create_builder());

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

        let i32_ty = self.context.i32_type();
        let main_type = i32_ty.fn_type(&[], false);
        let main_fn = ctx.module.add_function("main", main_type, None);
        let entry_bb = ctx.context.append_basic_block(main_fn, "entry");
        ctx.builder.position_at_end(entry_bb);
        ctx.current_function = Some(main_fn);

        for item in &program.items {
            match item {
                crate::parser::ast::ModuleItem::Declaration(decl) => {
                    compile_declaration(&mut ctx, decl)?;
                }
                crate::parser::ast::ModuleItem::Statement(stmt) => {
                    compile_block(&mut ctx, std::slice::from_ref(stmt))?;
                }
                _ => {}
            }
        }

        let current_bb = ctx.builder.get_insert_block().unwrap();
        if current_bb.get_terminator().is_none() {
            let zero = i32_ty.const_int(0, false);
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
    pub fn compile_to_object_with_opt(&self, path: &std::path::Path, opt_level: OptLevel) -> Result<(), String> {
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
    pub fn compile_to_binary_with_opt(&self, path: &std::path::Path, opt_level: OptLevel) -> Result<(), String> {
        let temp_obj = std::env::temp_dir().join("ruyi_temp.o");
        self.compile_to_object_with_opt(&temp_obj, opt_level)?;

        std::process::Command::new("cc")
            .arg(&temp_obj)
            .arg("-o")
            .arg(path)
            .arg("-lm")
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
    let fn_type = function_type_from_ruyi(ctx.context, &mono_func.param_types, &mono_func.return_type);
    let function = ctx.module.add_function(&mono_func.mangled_name, fn_type, None);

    let entry_bb = ctx.context.append_basic_block(function, "entry");
    let prev_function = ctx.current_function;
    ctx.current_function = Some(function);
    let prev_block = ctx.builder.get_insert_block();
    ctx.builder.position_at_end(entry_bb);

    // Allocate parameters
    let mut prev_vars = std::collections::HashMap::new();
    for (i, param_ty) in mono_func.param_types.iter().enumerate() {
        let param_name = format!("arg_{}", i);
        let llvm_ty = ruyi_type_to_llvm(ctx.context, param_ty);
        let ptr = ctx.builder.build_alloca(llvm_ty, &param_name);

        if let Some(param_value) = function.get_nth_param(i as u32) {
            ctx.builder.build_store(ptr, param_value);
        }

        if let Some(old) = ctx.variables.insert(param_name.clone(), (ptr, param_ty.clone())) {
            prev_vars.insert(param_name, old);
        }
    }

    // Ensure the function has a terminator
    let current_bb = ctx.builder.get_insert_block().unwrap();
    if current_bb.get_terminator().is_none() {
        use inkwell::values::BasicValueEnum;
        match &mono_func.return_type {
            Type::Void | Type::Never => {
                ctx.builder.build_return(None);
            }
            Type::Int => {
                let zero = ctx.context.i64_type().const_int(0, true);
                ctx.builder.build_return(Some(&BasicValueEnum::IntValue(zero)));
            }
            Type::Float => {
                let zero = ctx.context.f64_type().const_float(0.0);
                ctx.builder.build_return(Some(&BasicValueEnum::FloatValue(zero)));
            }
            Type::Bool => {
                let zero = ctx.context.bool_type().const_int(0, false);
                ctx.builder.build_return(Some(&BasicValueEnum::IntValue(zero)));
            }
            _ => {
                let null_ptr = ctx.context.i8_type().ptr_type(Default::default()).const_null();
                ctx.builder.build_return(Some(&BasicValueEnum::PointerValue(null_ptr)));
            }
        }
    }

    // Restore previous state
    ctx.current_function = prev_function;
    if let Some(block) = prev_block {
        ctx.builder.position_at_end(block);
    }
    for (name, old) in prev_vars {
        ctx.variables.insert(name, old);
    }

    Ok(())
}