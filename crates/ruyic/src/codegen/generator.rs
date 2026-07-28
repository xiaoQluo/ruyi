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

use crate::cli::gc_mode::GcMode;
use crate::driver::OptLevel;
use inkwell::builder::Builder;
use inkwell::context::Context;
use inkwell::module::Module;
use inkwell::targets::{
    CodeModel, FileType, InitializationConfig, RelocMode, Target, TargetMachine,
};
use inkwell::values::{BasicValueEnum, FunctionValue};
use inkwell::OptimizationLevel;
use ruyi_exception::landing_pad::LandingPadGenerator;

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
use super::traits::VTableRegistry;
use super::types::{function_type_from_ruyi, ruyi_type_to_llvm};
use crate::parser::ast::Program;
use crate::typechecker::generics::MonomorphizationTracker;
use crate::typechecker::types::Type;
use crate::typechecker::ArcClassRegistry;

/// Extract a `Declaration` reference from a `ModuleItem`, handling both
/// direct declarations and exported declarations.
fn extract_declaration(
    item: &crate::parser::ast::ModuleItem,
) -> Option<&crate::parser::ast::Declaration> {
    match item {
        crate::parser::ast::ModuleItem::Declaration(decl) => Some(decl),
        crate::parser::ast::ModuleItem::Export(crate::parser::ast::ExportDecl::Declaration(
            decl,
        )) => Some(decl),
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

/// Represents a single try-block's exception-handling frame on the invoke stack.
///
/// Used by `compile_try` (T4) to register an unwind target, and by `compile_call`
/// (T5) to decide whether to emit `invoke` instead of `call`.
pub struct TryFrame<'ctx> {
    pub landing_pad_bb: inkwell::basic_block::BasicBlock<'ctx>,
    pub catch_bb: Option<inkwell::basic_block::BasicBlock<'ctx>>,
    pub finally_bb: Option<inkwell::basic_block::BasicBlock<'ctx>>,
    pub exception_ptr: inkwell::values::PointerValue<'ctx>,
}

/// Context for code generation, holding LLVM constructs and variable mappings.
pub struct CodegenContext<'ctx, 'm, 'env> {
    pub context: &'ctx Context,
    pub module: &'m Module<'ctx>,
    pub(crate) builder: Builder<'ctx>,
    pub(crate) variables: HashMap<String, (inkwell::values::PointerValue<'ctx>, Type)>,
    pub(crate) globals: HashMap<String, inkwell::values::GlobalValue<'ctx>>,
    pub(crate) current_function: Option<FunctionValue<'ctx>>,
    pub(crate) loop_stack: Vec<(
        inkwell::basic_block::BasicBlock<'ctx>,
        inkwell::basic_block::BasicBlock<'ctx>,
        Option<String>,
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
    pub(crate) try_stack: Vec<TryContext<'ctx>>,
    pub try_frame_stack: Vec<TryFrame<'ctx>>,
    pub class_fields: HashMap<String, Vec<(String, Type)>>,
    pub class_struct_types: HashMap<String, inkwell::types::StructType<'ctx>>,
    pub enum_struct_types: HashMap<String, inkwell::types::StructType<'ctx>>,
    /// Maps child class name to parent class name (for super.new() support)
    pub class_extends: HashMap<String, String>,
    /// Counter for generating unique anonymous arrow function names.
    pub arrow_counter: u64,
    pub anon_counter: u64,
    pub async_arrow_counter: u64,
    pub function_types: HashMap<String, Type>,
    /// Maps function name to (rest_param_index, element_type) for rest parameter handling.
    pub rest_params: HashMap<String, (usize, Type)>,
    /// Tracks the return type of the current function for null sentinel handling.
    pub(crate) current_return_type: Option<Type>,
    /// Tracks the expected type for the current expression being compiled (for null literal handling).
    pub(crate) expected_expr_type: Option<Type>,
    /// When true, codegen errors for class/trait/impl declarations are logged as warnings
    /// instead of failing. Used when compiling stdlib modules that may use unsupported patterns.
    pub(crate) allow_partial_codegen: bool,
    /// Optional type environment from the type checker. When present, variable type lookups
    /// prioritize the type checker's inferred types over annotation-derived types.
    pub(crate) type_environment: Option<&'env crate::typechecker::environment::TypeEnvironment>,
    /// Label pending to be attached to the next loop push (set by `Statement::Labeled`,
    /// consumed by the loop compile functions like `compile_for`/`compile_while`).
    pub(crate) pending_loop_label: Option<String>,
    /// Active GC mode (`stub` or `real`) used by `GcAllocFn::for_mode`
    /// to choose between `call @cc_alloc` and `call @ruyi_gc_alloc`.
    pub(crate) gc_mode: GcMode,
    /// Registry of `@arc`-annotated class names. Consulted by `compile_new`
    /// to decide whether to emit `ruyi_arc_alloc` instead of GC allocation.
    pub(crate) arc_registry: ArcClassRegistry,
    /// Generic class templates: class name -> (type parameter names, class body).
    /// Used by call sites to instantiate specialized method copies on demand.
    pub generic_classes: HashMap<String, (Vec<String>, Vec<crate::parser::ast::ClassElement>)>,
    /// Trait-impl method signatures: mangled fn name -> (impl type parameter
    /// names, `for` type annotation). Used to substitute type parameters in
    /// declared return types at call sites.
    pub impl_method_sigs: HashMap<String, (Vec<String>, crate::parser::ast::TypeAnnotation)>,
    /// Specializations already attempted (successfully or not), keyed by the
    /// specialized function name, to avoid repeated failing instantiations.
    pub attempted_specializations: std::collections::HashSet<String>,
    /// Static field registry: maps `{Class}_{field}` to the field type.
    /// The backing LLVM global is looked up via `module.get_global()`.
    pub static_fields: HashMap<String, Type>,
    /// Getter registry: maps class_name -> set of getter property names.
    pub class_getters: HashMap<String, std::collections::HashSet<String>>,
    /// Setter registry: maps class_name -> set of setter property names.
    pub class_setters: HashMap<String, std::collections::HashSet<String>>,
    /// Runtime type ID registry for `instanceof` support.
    /// Maps class name to a unique numeric type ID.
    pub type_ids: HashMap<String, u64>,
    /// Next available type ID for class registration.
    pub next_type_id: u64,
    /// Pending return value slot for return-inside-try/finally support.
    /// When a `return` is executed inside a `try` block, the value is stored here
    /// and control branches to the finally block instead of returning directly.
    pub pending_return_value: Option<inkwell::values::PointerValue<'ctx>>,
    /// Pending return flag for return-inside-try/finally support.
    /// i1 alloca: true if a return is pending after finally execution.
    pub pending_return_flag: Option<inkwell::values::PointerValue<'ctx>>,
    /// Pending break target for break-inside-try/finally support.
    /// When a `break` is executed inside a `try` block, the target is stored here
    /// and control branches to the finally block first.
    pub pending_break_target: Option<inkwell::basic_block::BasicBlock<'ctx>>,
    /// Pending continue target for continue-inside-try/finally support.
    pub pending_continue_target: Option<inkwell::basic_block::BasicBlock<'ctx>>,
    /// Current class name being compiled (for super.new() resolution).
    /// Set by `compile_class` before compiling methods, cleared after.
    pub current_class_name: Option<String>,
    /// VTable registry for trait-based dynamic dispatch.
    /// Populated by `generate_vtables()` before declaration compilation.
    pub vtable_registry: Option<VTableRegistry<'ctx>>,
}

impl<'ctx, 'm, 'env> CodegenContext<'ctx, 'm, 'env> {
    pub fn new(
        context: &'ctx Context,
        module: &'m Module<'ctx>,
        builder: Builder<'ctx>,
        type_environment: Option<&'env crate::typechecker::environment::TypeEnvironment>,
    ) -> Self {
        Self::with_gc_mode(
            context,
            module,
            builder,
            type_environment,
            GcMode::default(),
        )
    }

    pub fn with_gc_mode(
        context: &'ctx Context,
        module: &'m Module<'ctx>,
        builder: Builder<'ctx>,
        type_environment: Option<&'env crate::typechecker::environment::TypeEnvironment>,
        gc_mode: GcMode,
    ) -> Self {
        Self {
            context,
            module,
            builder,
            variables: HashMap::new(),
            globals: HashMap::new(),
            current_function: None,
            loop_stack: Vec::new(),
            gc_roots: Vec::new(),
            async_state_field_ptr: None,
            async_result_ptr: None,
            async_return_bb: None,
            waker_ptr: None,
            try_stack: Vec::new(),
            try_frame_stack: Vec::new(),
            class_fields: HashMap::new(),
            class_struct_types: HashMap::new(),
            enum_struct_types: HashMap::new(),
            class_extends: HashMap::new(),
            arrow_counter: 0,
            anon_counter: 0,
            async_arrow_counter: 0,
            function_types: HashMap::new(),
            rest_params: HashMap::new(),
            current_return_type: None,
            expected_expr_type: None,
            allow_partial_codegen: false,
            type_environment,
            pending_loop_label: None,
            gc_mode,
            arc_registry: ArcClassRegistry::new(),
            generic_classes: HashMap::new(),
            impl_method_sigs: HashMap::new(),
            attempted_specializations: std::collections::HashSet::new(),
            static_fields: HashMap::new(),
            class_getters: HashMap::new(),
            class_setters: HashMap::new(),
            type_ids: HashMap::new(),
            next_type_id: 1,
            pending_return_value: None,
            pending_return_flag: None,
            pending_break_target: None,
            pending_continue_target: None,
            current_class_name: None,
            vtable_registry: None,
        }
    }

    /// Push a new GC root scope for a function.
    ///
    /// Prefer [`CodegenContext::gc_root_scope`] which returns a RAII guard
    /// guaranteeing the matching `pop_gc_root_scope` runs on every exit
    /// path (including `?` propagation and panics). This bare method is
    /// kept for internal use by the guard itself.
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
                        .builder()
                        .build_load(*ptr, "root_val")
                        .into_pointer_value();
                    super::builtins::build_gc_remove_root(self.builder(), self.module, loaded);
                }
            }
        }
    }

    /// Pop the current GC root scope.
    ///
    /// Prefer using the [`GcRootScopeGuard`] returned by
    /// [`CodegenContext::gc_root_scope`]. Calling this bare method is
    /// only safe when the surrounding code is guaranteed to reach the
    /// call site on every exit path; otherwise `gc_roots` will leak a
    /// stale scope and the GC may hold onto dead objects.
    pub fn pop_gc_root_scope(&mut self) {
        self.gc_roots.pop();
    }

    /// Push a new GC root scope and return a RAII guard that will pop it
    /// automatically on drop, even when the surrounding code exits via
    /// `?` propagation, panic, or an explicit `return` of a `Result::Err`.
    ///
    /// # Safety
    ///
    /// The guard holds a raw pointer to `self` so that other `&mut ctx`
    /// borrows remain possible while the guard is alive. The caller must
    /// ensure that `self` outlives the guard and is not moved during the
    /// guard's lifetime. In practice this is satisfied by binding the
    /// guard in a local scope where `self` is a stable `&mut` borrow.
    pub unsafe fn gc_root_scope(&mut self) -> GcRootScopeGuard<'ctx, 'm, 'env> {
        GcRootScopeGuard::push(self)
    }

    /// Register a local variable as a GC root and emit `ruyi_gc_add_root`.
    pub fn add_gc_root(&mut self, ptr: inkwell::values::PointerValue<'ctx>, ty: Type) {
        if let Some(scope) = self.gc_roots.last_mut() {
            scope.push((ptr, ty.clone()));
            let llvm_ty = super::types::ruyi_type_to_llvm(self.context, &ty);
            if llvm_ty.is_pointer_type() {
                let loaded = self
                    .builder()
                    .build_load(ptr, "root_val")
                    .into_pointer_value();
                super::builtins::build_gc_add_root(self.builder(), self.module, loaded);
            }
        }
    }

    /// Get a reference to the LLVM IR builder.
    pub fn builder(&self) -> &Builder<'ctx> {
        &self.builder
    }

    /// Get a reference to the variable map.
    pub fn variables(&self) -> &HashMap<String, (inkwell::values::PointerValue<'ctx>, Type)> {
        &self.variables
    }

    /// Get a mutable reference to the variable map.
    pub fn variables_mut(
        &mut self,
    ) -> &mut HashMap<String, (inkwell::values::PointerValue<'ctx>, Type)> {
        &mut self.variables
    }

    /// Look up a variable by name.
    ///
    /// Locals (parameters, let bindings) shadow module-level globals. The
    /// `type_environment` retains only the global scope at codegen time, so
    /// it must never override a local's recorded type: consulting it for a
    /// local named like a user global would poison the type (and previously
    /// the pointer) with the global's. It is used only as a fallback when
    /// the recorded type is `Dynamic`, and for genuine global accesses.
    pub fn lookup_variable(
        &self,
        name: &str,
    ) -> Option<(inkwell::values::PointerValue<'ctx>, Type)> {
        if let Some((ptr, ty)) = self.variables.get(name) {
            let final_ty = if matches!(ty, Type::Dynamic) {
                self.type_environment
                    .and_then(|env| env.lookup(name))
                    .cloned()
                    .unwrap_or_else(|| ty.clone())
            } else {
                ty.clone()
            };
            return Some((*ptr, final_ty));
        }
        if let Some(global) = self.globals.get(name) {
            let ptr = global.as_pointer_value();
            let final_ty = self
                .type_environment
                .and_then(|env| env.lookup(name))
                .cloned()
                .unwrap_or(crate::typechecker::types::Type::Dynamic);
            return Some((ptr, final_ty));
        }
        None
    }

    /// Resolve user-defined type aliases (e.g. `StringArray` -> `Array<string>`)
    /// by consulting the type checker's environment. Type aliases are recorded
    /// there as ordinary bindings via `declare_let`. Resolution is recursive so
    /// nested aliases and aliases inside Array/Nullable/Generic/Function are
    /// expanded. Used by method dispatch to map an alias receiver type to its
    /// underlying builtin/class so the correct method symbol is selected.
    pub fn resolve_type_aliases(&self, ty: &Type) -> Type {
        match ty {
            Type::Named(name, _) => {
                if let Some(env) = self.type_environment {
                    if let Some(resolved) = env.lookup(name) {
                        match resolved {
                            // A class/alias resolving to itself is not an alias.
                            Type::Named(n, _) if n == name => ty.clone(),
                            // Function-typed bindings are values, not type aliases.
                            Type::Function { .. } => ty.clone(),
                            _ => self.resolve_type_aliases(resolved),
                        }
                    } else {
                        ty.clone()
                    }
                } else {
                    ty.clone()
                }
            }
            Type::Array(inner) => Type::Array(Box::new(self.resolve_type_aliases(inner))),
            Type::Nullable(inner) => Type::Nullable(Box::new(self.resolve_type_aliases(inner))),
            Type::Generic { base, args } => Type::Generic {
                base: base.clone(),
                args: args.iter().map(|a| self.resolve_type_aliases(a)).collect(),
            },
            Type::Function {
                params,
                return_type,
            } => Type::Function {
                params: params
                    .iter()
                    .map(|p| self.resolve_type_aliases(p))
                    .collect(),
                return_type: Box::new(self.resolve_type_aliases(return_type)),
            },
            _ => ty.clone(),
        }
    }

    /// Define (insert) a variable into the map.
    pub fn define_variable(
        &mut self,
        name: String,
        val: (inkwell::values::PointerValue<'ctx>, Type),
    ) -> Option<(inkwell::values::PointerValue<'ctx>, Type)> {
        self.variables.insert(name, val)
    }

    /// Remove a variable from the map.
    pub fn remove_variable(
        &mut self,
        name: &str,
    ) -> Option<(inkwell::values::PointerValue<'ctx>, Type)> {
        self.variables.remove(name)
    }

    /// Get the current function being generated.
    pub fn current_function(&self) -> Option<FunctionValue<'ctx>> {
        self.current_function
    }

    /// Set the current function being generated.
    pub fn set_current_function(&mut self, func: Option<FunctionValue<'ctx>>) {
        self.current_function = func;
    }

    /// Push a new loop context (break target, continue target, optional label).
    pub fn push_loop(
        &mut self,
        break_bb: inkwell::basic_block::BasicBlock<'ctx>,
        continue_bb: inkwell::basic_block::BasicBlock<'ctx>,
        label: Option<String>,
    ) {
        self.loop_stack.push((break_bb, continue_bb, label));
    }

    /// Pop the innermost loop context.
    pub fn pop_loop(&mut self) {
        self.loop_stack.pop();
    }

    /// Get the innermost loop context.
    pub fn current_loop(
        &self,
    ) -> Option<(
        inkwell::basic_block::BasicBlock<'ctx>,
        inkwell::basic_block::BasicBlock<'ctx>,
        Option<String>,
    )> {
        self.loop_stack.last().cloned()
    }

    /// Push a new try context.
    pub fn push_try(&mut self, try_ctx: TryContext<'ctx>) {
        self.try_stack.push(try_ctx);
    }

    /// Pop the innermost try context.
    pub fn pop_try(&mut self) {
        self.try_stack.pop();
    }

    /// Get a reference to the innermost try context.
    pub fn current_try(&self) -> Option<&TryContext<'ctx>> {
        self.try_stack.last()
    }

    /// Check whether the try stack is empty.
    pub fn try_stack_is_empty(&self) -> bool {
        self.try_stack.is_empty()
    }

    /// Get the current function's return type.
    pub fn current_return_type(&self) -> Option<&Type> {
        self.current_return_type.as_ref()
    }

    /// Set the current function's return type.
    pub fn set_current_return_type(&mut self, ty: Option<Type>) {
        self.current_return_type = ty;
    }

    /// Get the expected expression type.
    pub fn expected_expr_type(&self) -> Option<&Type> {
        self.expected_expr_type.as_ref()
    }

    /// Set the expected expression type.
    pub fn set_expected_expr_type(&mut self, ty: Option<Type>) {
        self.expected_expr_type = ty;
    }

    /// Check whether partial codegen is allowed.
    pub fn allow_partial_codegen(&self) -> bool {
        self.allow_partial_codegen
    }

    /// Set whether partial codegen is allowed.
    pub fn set_allow_partial_codegen(&mut self, allow: bool) {
        self.allow_partial_codegen = allow;
    }
}

/// RAII guard that owns a single GC root scope pushed by
/// [`CodegenContext::push_gc_root_scope`] and pops it on `Drop`.
///
/// This guarantees that the matching `pop_gc_root_scope` is always
/// reached, including on `?` propagation, `Result::Err` early returns,
/// and panics — the cases where the bare push/pop pair is easy to leak.
///
/// The guard is `#[must_use]` so that an accidental
/// `ctx.gc_root_scope();` (which would drop the guard immediately and
/// undo the push) is a compile-time warning.
///
/// # Safety
///
/// Constructed via [`GcRootScopeGuard::push`], which is the only safe
/// construction path. The guard stores a raw pointer to the parent
/// context; the caller must ensure the context outlives the guard and
/// is not moved for the guard's lifetime. Local-scope use satisfies
/// both requirements.
#[must_use = "GcRootScopeGuard pops its scope on drop; binding it to `_` discards the scope immediately"]
pub struct GcRootScopeGuard<'ctx, 'm, 'env> {
    ctx: *mut CodegenContext<'ctx, 'm, 'env>,
}

impl<'ctx, 'm, 'env> GcRootScopeGuard<'ctx, 'm, 'env> {
    /// Push a new GC root scope on `ctx` and return a guard that will
    /// pop it on drop.
    ///
    /// # Safety
    ///
    /// `ctx` must outlive the returned guard and must not be moved for
    /// the guard's lifetime. The guard holds a raw pointer, so the
    /// borrow checker will not enforce this — the caller is responsible.
    pub unsafe fn push(ctx: &mut CodegenContext<'ctx, 'm, 'env>) -> Self {
        ctx.push_gc_root_scope();
        Self { ctx: ctx as *mut _ }
    }
}

impl<'ctx, 'm, 'env> Drop for GcRootScopeGuard<'ctx, 'm, 'env> {
    fn drop(&mut self) {
        // SAFETY: The guard's lifetime is bound to the mutable borrow
        // of the context at the push site. The context is guaranteed
        // to be alive when the guard is dropped because the guard
        // lives in a scope where the context is still accessible.
        // SAFETY: pop_gc_root_scope guaranteed by GcRootScopeGuard Drop
        unsafe {
            (*self.ctx).pop_gc_root_scope();
        }
    }
}

#[must_use = "TryStackGuard pops its frame on drop; binding it to `_` discards the frame immediately"]
pub struct TryStackGuard<'ctx, 'm, 'env> {
    ctx: *mut CodegenContext<'ctx, 'm, 'env>,
}

impl<'ctx, 'm, 'env> TryStackGuard<'ctx, 'm, 'env> {
    /// Push a `TryFrame` onto `ctx.try_frame_stack` and return a guard
    /// that pops it on drop.
    ///
    /// # Safety
    ///
    /// `ctx` must outlive the returned guard and must not be moved for
    /// the guard's lifetime. The guard stores a raw pointer, so the
    /// borrow checker will not enforce this — the caller is responsible.
    pub unsafe fn push(ctx: &mut CodegenContext<'ctx, 'm, 'env>, frame: TryFrame<'ctx>) -> Self {
        ctx.try_frame_stack.push(frame);
        Self { ctx: ctx as *mut _ }
    }
}

impl<'ctx, 'm, 'env> Drop for TryStackGuard<'ctx, 'm, 'env> {
    fn drop(&mut self) {
        // SAFETY: The guard's lifetime is bound to the mutable borrow
        // of the context at the push site. The context is guaranteed
        // to be alive when the guard is dropped.
        unsafe {
            (*self.ctx)
                .try_frame_stack
                .pop()
                .expect("TryStackGuard: try_frame_stack underflow on drop");
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
    pub gc_mode: GcMode,
    /// Number of stdlib ModuleItems prepended before user code. Items at index
    /// `< stdlib_item_count` are treated as stdlib and tolerate partial codegen.
    pub stdlib_item_count: usize,
}

impl<'ctx> CodeGenerator<'ctx> {
    pub fn new(context: &'ctx Context, name: &str) -> Self {
        Self::with_gc_mode(context, name, GcMode::default())
    }

    pub fn with_gc_mode(context: &'ctx Context, name: &str, gc_mode: GcMode) -> Self {
        let module = context.create_module(name);
        let builder = context.create_builder();
        Self {
            context,
            module,
            builder,
            allow_partial_codegen: false,
            gc_mode,
            stdlib_item_count: 0,
        }
    }

    pub fn gc_mode(&self) -> GcMode {
        self.gc_mode
    }

    /// Generate LLVM IR from a typed AST program.
    pub fn generate(&self, program: &Program) -> Result<(), String> {
        self.generate_with_env(program, &MonomorphizationTracker::new(), None)
    }

    /// Generate LLVM IR from a typed AST program with monomorphization tracker.
    pub fn generate_with_tracker(
        &self,
        program: &Program,
        tracker: &MonomorphizationTracker,
    ) -> Result<(), String> {
        self.generate_with_env(program, tracker, None)
    }

    /// Generate LLVM IR from a typed AST program with monomorphization tracker
    /// and an optional type environment.
    pub fn generate_with_env(
        &self,
        program: &Program,
        tracker: &MonomorphizationTracker,
        type_env: Option<&crate::typechecker::environment::TypeEnvironment>,
    ) -> Result<(), String> {
        let mut ctx = CodegenContext::with_gc_mode(
            self.context,
            &self.module,
            self.context.create_builder(),
            type_env,
            self.gc_mode,
        );
        ctx.set_allow_partial_codegen(self.allow_partial_codegen);

        declare_builtins(self.context, &self.module, self.gc_mode);

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
        use inkwell::attributes::{Attribute, AttributeLoc};
        let main_fn = ctx
            .module
            .add_function(llvm_main_name, i32_ty.fn_type(&[], false), None);

        // Set personality and uwtable on main — required so the platform
        // unwinder can find landingpad handlers when throw originates from
        // a callee (e.g. innerThrow → ruyi_throw → _Unwind_RaiseException).
        let lp_gen = LandingPadGenerator::new(ctx.context, ctx.module, ctx.builder());
        main_fn.set_personality_function(lp_gen.get_personality_function());
        let uwtable_id = Attribute::get_named_enum_kind_id("uwtable");
        main_fn.add_attribute(
            AttributeLoc::Function,
            ctx.context.create_enum_attribute(uwtable_id, 0),
        );
        let entry_bb = ctx.context.append_basic_block(main_fn, "entry");
        ctx.builder().position_at_end(entry_bb);
        ctx.set_current_function(Some(main_fn));

        let top_level_lets = collect_top_level_lets(program, ctx.type_environment);
        for (name, ty) in &top_level_lets {
            let llvm_ty = ruyi_type_to_llvm(ctx.context, ty);
            let global = ctx.module.add_global(llvm_ty, None, name);
            global.set_linkage(inkwell::module::Linkage::Internal);
            let zero = ruyi_type_to_zero(ctx.context, ty);
            global.set_initializer(&zero);
            ctx.globals.insert(name.clone(), global);
        }

        // Save entry block position before compiling declarations
        let main_entry_bb = entry_bb;

        // Collect the main function body to compile into the entry point
        let mut main_body: Option<&[crate::parser::ast::Statement]> = None;
        let mut main_params: Option<&[crate::parser::ast::Param]> = None;
        // Compiled after main_body so the entry block stays terminator-free.
        let mut top_level_stmts: Vec<crate::parser::ast::Statement> = Vec::new();

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
            let decl = match extract_declaration(item) {
                Some(d) => d,
                None => continue,
            };
            if let crate::parser::ast::Declaration::Class { name, body, .. } = decl {
                // Register a unique type ID for instanceof support.
                let type_id = ctx.next_type_id;
                ctx.next_type_id += 1;
                ctx.type_ids.insert(name.clone(), type_id);

                // Predeclare struct type (with hidden __typeid field at index 0)
                let mut fields: Vec<(String, crate::typechecker::types::Type)> = Vec::new();
                fields.push(("__typeid".to_string(), crate::typechecker::types::Type::Int));
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
                let mut getters = std::collections::HashSet::new();
                let mut setters = std::collections::HashSet::new();
                for element in body {
                    if let crate::parser::ast::ClassElement::Method {
                        name: prop_name,
                        params,
                        return_type,
                        is_static: false,
                        is_async: false,
                        is_getter,
                        is_setter,
                        ..
                    } = element
                    {
                        if let crate::parser::ast::PropertyName::Ident(method) = prop_name {
                            if *is_getter {
                                getters.insert(method.clone());
                            }
                            if *is_setter {
                                setters.insert(method.clone());
                            }
                            let method_name = if *is_setter {
                                format!("{}_set_{}", name, method)
                            } else {
                                format!("{}_{}", name, method)
                            };
                            let mut method_params = vec![crate::parser::ast::Param {
                                pattern: crate::parser::ast::Pattern::Identifier(
                                    "self".to_string(),
                                ),
                                ty: Some(crate::parser::ast::TypeAnnotation::Identifier(
                                    name.clone(),
                                )),
                                init: None,
                                is_rest: false,
                                is_optional: false,
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
                if !getters.is_empty() {
                    ctx.class_getters.insert(name.clone(), getters);
                }
                if !setters.is_empty() {
                    ctx.class_setters.insert(name.clone(), setters);
                }

                // Predeclare static methods so other classes can
                // forward-reference them (e.g. Process.create calling
                // ProcessOptions.default()).
                for element in body {
                    if let crate::parser::ast::ClassElement::Method {
                        name: prop_name,
                        params,
                        return_type,
                        is_static: true,
                        is_async: false,
                        ..
                    } = element
                    {
                        if let crate::parser::ast::PropertyName::Ident(method) = prop_name {
                            let method_name = format!("{}_{}", name, method);
                            let method_params: Vec<_> = params
                                .iter()
                                .filter(|p| !matches!(&p.pattern, crate::parser::ast::Pattern::Identifier(n) if n == "self"))
                                .cloned()
                                .collect();
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

        // Register @arc classes and generate per-class TypeInfo globals.
        // The TypeInfo struct layout matches ruyi_runtime::alloc::TypeInfo:
        //   { i64 type_id, i8* type_name, i8* destructor, i8* trace_fn }
        // ARC classes skip GC tracing (destructor and trace_fn are null).
        let mut arc_type_id: u64 = 1000;
        let i64_ty = ctx.context.i64_type();
        let i8_ptr = ctx.context.i8_type().ptr_type(Default::default());
        let type_info_struct = ctx.context.struct_type(
            &[i64_ty.into(), i8_ptr.into(), i8_ptr.into(), i8_ptr.into()],
            false,
        );

        for item in &program.items {
            if let crate::parser::ast::ModuleItem::Declaration(
                crate::parser::ast::Declaration::Class {
                    name, annotations, ..
                },
            ) = item
            {
                if annotations.iter().any(|a| a == "arc") {
                    ctx.arc_registry.register(name);

                    // Create a constant string global for the class name.
                    // build_global_string_ptr generates a runtime GEP instruction
                    // that cannot appear in a global initializer; therefore we
                    // build the global and the const GEP by hand.
                    let str_bytes = name.as_bytes();
                    let str_len = str_bytes.len() as u32 + 1;
                    let str_array_ty = ctx.context.i8_type().array_type(str_len);
                    let str_global =
                        ctx.module
                            .add_global(str_array_ty, None, &format!("ti_str_{}", name));
                    str_global.set_initializer(&ctx.context.const_string(str_bytes, true));
                    str_global.set_linkage(inkwell::module::Linkage::Private);

                    // const GEP via const_cast: bitcast [N x i8]* to i8*
                    // Both addresses are identical; const_cast emits LLVMConstBitCast.
                    let str_ptr = str_global.as_pointer_value().const_cast(i8_ptr);

                    let type_id = arc_type_id;
                    arc_type_id += 1;

                    let global_name = format!("ruyi_type_info_{}", name);
                    let global = ctx.module.add_global(type_info_struct, None, &global_name);
                    global.set_linkage(inkwell::module::Linkage::Internal);
                    global.set_initializer(&type_info_struct.const_named_struct(&[
                        i64_ty.const_int(type_id, false).into(),
                        str_ptr.into(),
                        i8_ptr.const_null().into(),
                        i8_ptr.const_null().into(),
                    ]));
                }
            }
        }

        // Generate vtables for all trait impl declarations.
        // Must run before compile_declaration so that trait object creation
        // in let bindings / assignments can look up vtable info.
        ctx.vtable_registry = Some(super::traits::generate_vtables(&mut ctx, program));

        for (i, item) in program.items.iter().enumerate() {
            ctx.set_allow_partial_codegen(self.allow_partial_codegen || i < self.stdlib_item_count);
            match item {
                crate::parser::ast::ModuleItem::Declaration(decl) => {
                    if let crate::parser::ast::Declaration::Function { name, is_async, .. } = decl {
                        if name == "main" && !*is_async {
                            continue;
                        }
                    }
                    if matches!(
                        decl,
                        crate::parser::ast::Declaration::Let(_)
                            | crate::parser::ast::Declaration::Const(_)
                    ) {
                        continue;
                    }
                    if let Err(_e) = compile_declaration(&mut ctx, decl) {
                        match decl {
                            crate::parser::ast::Declaration::Class { .. }
                            | crate::parser::ast::Declaration::Impl { .. }
                            | crate::parser::ast::Declaration::Trait { .. } => {
                                if !ctx.allow_partial_codegen() {
                                    return Err(format!("codegen error: {}", _e));
                                }
                                log::warn!("Skipping declaration codegen: {}", _e);
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
                    if matches!(
                        decl,
                        crate::parser::ast::Declaration::Let(_)
                            | crate::parser::ast::Declaration::Const(_)
                    ) {
                        continue;
                    }
                    if let Err(_e) = compile_declaration(&mut ctx, decl) {
                        if !ctx.allow_partial_codegen() {
                            return Err(format!("codegen error: {}", _e));
                        }
                        log::warn!("Skipping export codegen: {}", _e);
                    }
                }
                crate::parser::ast::ModuleItem::Statement(stmt) => {
                    top_level_stmts.push(stmt.clone());
                }
                _ => {}
            }
        }

        // Emit vtable initializer globals now that all impl functions exist.
        if let Some(registry) = ctx.vtable_registry.clone() {
            super::traits::emit_vtable_initializers(&mut ctx, &registry);
        }

        // Restore builder position to entry main block
        ctx.builder().position_at_end(main_entry_bb);
        ctx.set_current_function(Some(main_fn));

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
                let ptr = ctx.builder().build_alloca(llvm_ty, &param_name);
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
                    Type::Byte => {
                        BasicValueEnum::IntValue(ctx.context.i8_type().const_int(0, false))
                    }
                    _ => BasicValueEnum::PointerValue(
                        ctx.context
                            .i8_type()
                            .ptr_type(Default::default())
                            .const_null(),
                    ),
                };
                ctx.builder().build_store(ptr, zero);
                if crate::codegen::builtins::is_gc_managed(&param_ty) {
                    ctx.add_gc_root(ptr, param_ty.clone());
                }
                ctx.define_variable(param_name, (ptr, param_ty));
            }
        }

        compile_top_level_let_inits(&mut ctx, program);

        // Compile main function body
        if let Some(body) = main_body {
            compile_block(&mut ctx, body)?;
        }

        for stmt in &top_level_stmts {
            compile_block(&mut ctx, std::slice::from_ref(stmt))?;
        }

        let current_bb = ctx.builder().get_insert_block().unwrap();
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
                    ctx.builder()
                        .build_call(spawn_fn, &[future_ptr.into()], "spawn_main");
                    let scheduler_fn = ctx
                        .module
                        .get_function("ruyi_run_scheduler")
                        .expect("ruyi_run_scheduler not declared");
                    ctx.builder().build_call(scheduler_fn, &[], "run_scheduler");
                }
            }
            let zero = BasicValueEnum::IntValue(i32_ty.const_int(0, false));
            ctx.builder().build_return(Some(&zero));
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
        // Verify the module before handing it to the LLVM backend.
        // Log as warning since some pre-existing issues may still cause
        // verification warnings.
        if let Err(msg) = self.module.verify() {
            eprintln!("LLVM verification warning: {}", msg.to_string());
        }

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
        // Use PID in the temp object filename to avoid collisions when
        // multiple ruyic processes run simultaneously (e.g. parallel tests).
        let temp_obj = std::env::temp_dir().join(format!("ruyi_temp_{}.o", std::process::id()));
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
    ctx: &mut CodegenContext<'ctx, '_, '_>,
    mono_func: &MonomorphizedFunction,
) -> Result<(), String> {
    let fn_type =
        function_type_from_ruyi(ctx.context, &mono_func.param_types, &mono_func.return_type);
    let function = ctx
        .module
        .add_function(&mono_func.mangled_name, fn_type, None);

    let entry_bb = ctx.context.append_basic_block(function, "entry");
    let prev_function = ctx.current_function();
    let prev_return_type = ctx.current_return_type().cloned();
    ctx.set_current_function(Some(function));
    ctx.set_current_return_type(Some(mono_func.return_type.clone()));
    let prev_block = ctx.builder().get_insert_block();
    ctx.builder().position_at_end(entry_bb);

    // Allocate parameters
    let mut prev_vars = std::collections::HashMap::new();
    // RAII guard: pop_gc_root_scope guaranteed by GcRootScopeGuard Drop
    let _gc_scope_guard = unsafe { ctx.gc_root_scope() };

    for (i, param_ty) in mono_func.param_types.iter().enumerate() {
        let param_name = format!("arg_{}", i);
        let llvm_ty = ruyi_type_to_llvm(ctx.context, param_ty);
        let ptr = ctx.builder().build_alloca(llvm_ty, &param_name);

        if let Some(param_value) = function.get_nth_param(i as u32) {
            ctx.builder().build_store(ptr, param_value);
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
    let current_bb = ctx.builder().get_insert_block().unwrap();
    if current_bb.get_terminator().is_none() {
        use inkwell::values::BasicValueEnum;
        ctx.emit_gc_root_removals();
        match &mono_func.return_type {
            Type::Void | Type::Never => {
                ctx.builder().build_return(None);
            }
            Type::Int => {
                let zero = ctx.context.i64_type().const_int(0, true);
                ctx.builder()
                    .build_return(Some(&BasicValueEnum::IntValue(zero)));
            }
            Type::Float => {
                let zero = ctx.context.f64_type().const_float(0.0);
                ctx.builder()
                    .build_return(Some(&BasicValueEnum::FloatValue(zero)));
            }
            Type::Bool => {
                let zero = ctx.context.bool_type().const_int(0, false);
                ctx.builder()
                    .build_return(Some(&BasicValueEnum::IntValue(zero)));
            }
            Type::Byte => {
                let zero = ctx.context.i8_type().const_int(0, false);
                ctx.builder()
                    .build_return(Some(&BasicValueEnum::IntValue(zero)));
            }
            _ => {
                // Generate a zero value matching the LLVM function's actual
                // return type (which may differ from the Ruyi type due to
                // generic type erasure, e.g. Nullable(Dynamic) → i64).
                let llvm_ret_ty = ctx
                    .builder()
                    .get_insert_block()
                    .and_then(|bb| bb.get_parent())
                    .and_then(|f| f.get_type().get_return_type());
                let default_val: BasicValueEnum<'ctx> = match llvm_ret_ty {
                    Some(t) if t.is_int_type() => {
                        BasicValueEnum::IntValue(t.into_int_type().const_int(0, false))
                    }
                    Some(t) if t.is_struct_type() => {
                        BasicValueEnum::StructValue(t.into_struct_type().const_zero())
                    }
                    Some(t) if t.is_float_type() => {
                        BasicValueEnum::FloatValue(t.into_float_type().const_float(0.0))
                    }
                    _ => {
                        let null_ptr = ctx
                            .context
                            .i8_type()
                            .ptr_type(Default::default())
                            .const_null();
                        BasicValueEnum::PointerValue(null_ptr)
                    }
                };
                ctx.builder().build_return(Some(&default_val));
            }
        }
    }

    // Restore previous state
    ctx.set_current_function(prev_function);
    ctx.set_current_return_type(prev_return_type);
    if let Some(block) = prev_block {
        ctx.builder().position_at_end(block);
    }
    for (name, old) in prev_vars {
        ctx.define_variable(name, old);
    }

    Ok(())
}

fn compile_top_level_let_inits<'ctx>(
    ctx: &mut CodegenContext<'ctx, '_, '_>,
    program: &crate::parser::ast::Program,
) {
    use super::expr::compile_expr;
    use crate::parser::ast::{Declaration, ExportDecl, ModuleItem, Pattern};
    use crate::typechecker::types::Type;
    use inkwell::values::BasicValueEnum;

    // Track already-initialized names to avoid duplicate stores when the same
    // const/let name appears in multiple merged modules (e.g. HEX_CHARS in
    // both encoding.ry and buffer.ry).  Only the first occurrence is
    // initialized; subsequent duplicates share the same LLVM global.
    let mut initialized: std::collections::HashSet<String> = std::collections::HashSet::new();

    for item in &program.items {
        let decl_opt = match item {
            ModuleItem::Declaration(decl) => Some(decl),
            ModuleItem::Export(ExportDecl::Declaration(decl)) => Some(decl),
            _ => None,
        };
        if let Some(Declaration::Let(bindings) | Declaration::Const(bindings)) = decl_opt {
            for b in bindings {
                let name = match &b.pattern {
                    Pattern::Identifier(n) => n.clone(),
                    _ => continue,
                };
                // Skip duplicate names across modules — the first binding
                // owns the global; later ones are shadowed copies.
                if !initialized.insert(name.clone()) {
                    continue;
                }
                let Some(init) = &b.init else { continue };
                let ty = if let Some(annotation) = &b.ty {
                    Type::from_annotation(annotation)
                } else if let Some(env) = ctx.type_environment {
                    env.lookup(&name).cloned().unwrap_or(Type::Dynamic)
                } else {
                    Type::Dynamic
                };
                // Look up the global via ctx.globals (authoritative mapping
                // created in generate_with_env).  Fall back to module lookup
                // only for names not registered there (shouldn't happen).
                let global = match ctx.globals.get(&name) {
                    Some(g) => *g,
                    None => {
                        let llvm_ty = ruyi_type_to_llvm(ctx.context, &ty);
                        let g = ctx.module.add_global(llvm_ty, None, &name);
                        g.set_linkage(inkwell::module::Linkage::Internal);
                        let zero = ruyi_type_to_zero(ctx.context, &ty);
                        g.set_initializer(&zero);
                        g
                    }
                };
                let init_result = match compile_expr(ctx, init) {
                    Ok(r) => r,
                    Err(_) => continue,
                };
                let value = if ty == Type::Dynamic && init_result.ty != Type::Dynamic {
                    // Dynamic boxing: construct {i64, i8*} struct
                    super::expr::build_box_dynamic(ctx, init_result.value, &init_result.ty)
                } else {
                    init_result.value
                };
                ctx.builder().build_store(global.as_pointer_value(), value);
                // Record the declared type; keep Dynamic when annotated as dyn
                // so later reads know to extract struct fields.
                let recorded_ty = ty;
                ctx.define_variable(name, (global.as_pointer_value(), recorded_ty));
                let _ = BasicValueEnum::IntValue(ctx.context.i64_type().const_int(0, false));
            }
        }
    }
}

fn collect_top_level_lets(
    program: &crate::parser::ast::Program,
    type_env: Option<&crate::typechecker::environment::TypeEnvironment>,
) -> Vec<(String, crate::typechecker::types::Type)> {
    use crate::parser::ast::{Declaration, ExportDecl, ModuleItem, Pattern};
    use crate::typechecker::types::Type;
    let mut result = Vec::new();
    // Deduplicate: when multiple merged modules define the same top-level
    // name (e.g. HEX_CHARS in both encoding.ry and buffer.ry), only the
    // first binding is kept.  Subsequent duplicates share the same LLVM
    // global and must not create a second one.
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    for item in &program.items {
        let decl_opt = match item {
            ModuleItem::Declaration(decl) => Some(decl),
            ModuleItem::Export(ExportDecl::Declaration(decl)) => Some(decl),
            _ => None,
        };
        if let Some(Declaration::Let(bindings) | Declaration::Const(bindings)) = decl_opt {
            for b in bindings {
                let name = match &b.pattern {
                    Pattern::Identifier(n) => n.clone(),
                    _ => continue,
                };
                if !seen.insert(name.clone()) {
                    continue;
                }
                let ty = if let Some(annotation) = &b.ty {
                    Type::from_annotation(annotation)
                } else if let Some(env) = type_env {
                    env.lookup(&name).cloned().unwrap_or(Type::Dynamic)
                } else {
                    Type::Dynamic
                };
                result.push((name, ty));
            }
        }
    }
    result
}

pub(crate) fn ruyi_type_to_zero<'ctx>(
    context: &'ctx inkwell::context::Context,
    ty: &crate::typechecker::types::Type,
) -> inkwell::values::BasicValueEnum<'ctx> {
    use crate::typechecker::types::Type;
    use inkwell::values::BasicValueEnum;
    match ty {
        Type::Int => BasicValueEnum::IntValue(context.i64_type().const_int(0, false)),
        Type::Float => BasicValueEnum::FloatValue(context.f64_type().const_float(0.0)),
        Type::Bool => BasicValueEnum::IntValue(context.bool_type().const_int(0, false)),
        Type::Byte => BasicValueEnum::IntValue(context.i8_type().const_int(0, false)),
        // Pointer types must be initialized with null pointer, not integer 0.
        Type::String
        | Type::Null
        | Type::BigInt
        | Type::Array(_)
        | Type::Object(_)
        | Type::Function { .. }
        | Type::Generic { .. }
        | Type::TypeVar(_) => BasicValueEnum::PointerValue(
            context.i8_type().ptr_type(Default::default()).const_null(),
        ),
        Type::Nullable(_) => {
            // Nullable wraps an inner type; use null pointer.
            BasicValueEnum::PointerValue(
                context.i8_type().ptr_type(Default::default()).const_null(),
            )
        }
        Type::Dynamic => {
            // Dynamic is {i64, i8*} — zero-initialized struct
            let dyn_ty =
                super::types::ruyi_type_to_llvm(context, &Type::Dynamic).into_struct_type();
            BasicValueEnum::StructValue(dyn_ty.const_zero())
        }
        _ => BasicValueEnum::IntValue(context.i64_type().const_int(0, false)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_try_stack_push_pop() {
        let context = Context::create();
        let module = context.create_module("test");
        let builder = context.create_builder();
        let mut ctx = CodegenContext::new(&context, &module, builder, None);

        let func_type = context.void_type().fn_type(&[], false);
        let func = module.add_function("test_fn", func_type, None);
        let bb = context.append_basic_block(func, "entry");
        ctx.builder().position_at_end(bb);

        let i8_ptr_type = context.i8_type().ptr_type(Default::default());
        let exception_ptr = ctx.builder().build_alloca(i8_ptr_type, "exc_ptr");

        let frame = TryFrame {
            landing_pad_bb: bb,
            catch_bb: Some(bb),
            finally_bb: None,
            exception_ptr,
        };

        assert_eq!(ctx.try_frame_stack.len(), 0);

        {
            let _guard = unsafe { TryStackGuard::push(&mut ctx, frame) };
            assert_eq!(ctx.try_frame_stack.len(), 1);
        }

        assert_eq!(ctx.try_frame_stack.len(), 0);
    }
}
