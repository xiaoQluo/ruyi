//! LLVM landing-pad generation for Ruyi exception handling.
//!
//! Provides helpers to emit `landingpad`, `invoke`, and `resume`
//! instructions using inkwell.

#[cfg(feature = "inkwell")]
pub mod llvm {
    use inkwell::basic_block::BasicBlock;
    use inkwell::builder::Builder;
    use inkwell::context::Context;
    use inkwell::module::Module;
    use inkwell::values::{BasicValue, BasicValueEnum, FunctionValue, IntValue, PointerValue};
    use inkwell::AddressSpace;
    use inkwell::IntPredicate;

    use crate::exception::TypeId;

    /// Generator for LLVM landing-pad instructions.
    pub struct LandingPadGenerator<'ctx, 'm, 'b> {
        context: &'ctx Context,
        module: &'m Module<'ctx>,
        builder: &'b Builder<'ctx>,
    }

    impl<'ctx, 'm, 'b> LandingPadGenerator<'ctx, 'm, 'b> {
        /// Create a new landing-pad generator.
        pub fn new(
            context: &'ctx Context,
            module: &'m Module<'ctx>,
            builder: &'b Builder<'ctx>,
        ) -> Self {
            Self {
                context,
                module,
                builder,
            }
        }

        /// Build a `landingpad` instruction.
        ///
        /// Returns a `{ i8*, i32 }` value. Each entry in `catch_type_ids`
        /// produces a `catch` clause. If `has_cleanup` is `true`, the
        /// landing pad also has a `cleanup` clause (used for `finally`).
        pub fn build_landing_pad(
            &self,
            catch_type_ids: &[TypeId],
            has_cleanup: bool,
            name: &str,
        ) -> BasicValueEnum<'ctx> {
            let i8_ptr = self.context.i8_type().ptr_type(AddressSpace::default());
            let i32_ty = self.context.i32_type();
            let lpad_ty = self.context.struct_type(&[i8_ptr.into(), i32_ty.into()], false);

            let personality = self.get_personality_function();

            let mut clauses: Vec<BasicValueEnum<'ctx>> = Vec::new();
            for &type_id in catch_type_ids {
                let type_info = self.get_type_info_global(type_id);
                clauses.push(type_info.as_basic_value_enum());
            }

            self.builder.build_landing_pad(
                lpad_ty,
                personality,
                &clauses,
                has_cleanup,
                name,
            )
        }

        /// Build an `invoke` instruction.
        ///
        /// Unlike `call`, `invoke` specifies both a normal return block
        /// and an unwind landing-pad block, which is required for any
        /// call inside a `try` region.
        pub fn build_invoke(
            &self,
            fn_val: FunctionValue<'ctx>,
            args: &[BasicValueEnum<'ctx>],
            then_bb: BasicBlock<'ctx>,
            catch_bb: BasicBlock<'ctx>,
            name: &str,
        ) -> inkwell::values::CallSiteValue<'ctx> {
            self.builder.build_invoke(fn_val, args, then_bb, catch_bb, name)
        }

        /// Build a `resume` instruction.
        ///
        /// Used when an exception is not caught by any handler and must
        /// continue propagating up the stack.
        pub fn build_resume(&self, landing_pad_val: BasicValueEnum<'ctx>) {
            self.builder.build_resume(landing_pad_val);
        }

        /// Extract the exception pointer from a landing-pad result.
        pub fn extract_exception_ptr(
            &self,
            landing_pad_val: BasicValueEnum<'ctx>,
        ) -> PointerValue<'ctx> {
            self.builder
                .build_extract_value(landing_pad_val.into_struct_value(), 0, "exc.ptr")
                .unwrap()
                .into_pointer_value()
        }

        /// Extract the selector value from a landing-pad result.
        pub fn extract_selector(&self, landing_pad_val: BasicValueEnum<'ctx>) -> IntValue<'ctx> {
            self.builder
                .build_extract_value(landing_pad_val.into_struct_value(), 1, "exc.selector")
                .unwrap()
                .into_int_value()
        }

        /// Build a call to the `llvm.eh.typeid.for` intrinsic.
        ///
        /// Returns an `i32` selector value that the landing-pad selector
        /// is compared against to determine whether a catch clause matches.
        pub fn build_eh_typeid_for(&self, type_id: TypeId) -> IntValue<'ctx> {
            let i8_ptr = self.context.i8_type().ptr_type(AddressSpace::default());
            let i32_ty = self.context.i32_type();
            let fn_type = i32_ty.fn_type(&[i8_ptr.into()], false);

            let intrinsic = get_or_insert_function(self.module, "llvm.eh.typeid.for", fn_type);
            let type_info = self.get_type_info_global(type_id);

            self.builder
                .build_call(intrinsic, &[type_info.as_basic_value_enum().into()], "typeid")
                .try_as_basic_value()
                .left()
                .unwrap()
                .into_int_value()
        }

        /// Build a simple catch-dispatch chain.
        ///
        /// For each `(type_id, handler_bb)` pair, compares the landing-pad
        /// selector against `llvm.eh.typeid.for(type_id)` and branches to
        /// `handler_bb` on equality. If no clause matches, falls through
        /// to `resume_bb` (or `cleanup_bb` when provided).
        pub fn build_catch_dispatch(
            &self,
            landing_pad_val: BasicValueEnum<'ctx>,
            catch_handlers: &[(TypeId, BasicBlock<'ctx>)],
            cleanup_bb: Option<BasicBlock<'ctx>>,
            resume_bb: BasicBlock<'ctx>,
        ) {
            let selector = self.extract_selector(landing_pad_val);
            let func = self
                .builder
                .get_insert_block()
                .unwrap()
                .get_parent()
                .unwrap();

            for (idx, &(type_id, handler_bb)) in catch_handlers.iter().enumerate() {
                let typeid_for = self.build_eh_typeid_for(type_id);
                let matches = self.builder.build_int_compare(
                    IntPredicate::EQ,
                    selector,
                    typeid_for,
                    &format!("catch.matches.{}", idx),
                );

                let next_bb = if idx + 1 < catch_handlers.len() {
                    self.context
                        .append_basic_block(func, &format!("catch.check.{}", idx + 1))
                } else if let Some(cleanup) = cleanup_bb {
                    cleanup
                } else {
                    resume_bb
                };

                self.builder
                    .build_conditional_branch(matches, handler_bb, next_bb);

                if idx + 1 < catch_handlers.len() {
                    self.builder.position_at_end(next_bb);
                }
            }

            if catch_handlers.is_empty() {
                if let Some(cleanup) = cleanup_bb {
                    self.builder.build_unconditional_branch(cleanup);
                } else {
                    self.builder.build_unconditional_branch(resume_bb);
                }
            }
        }

        fn get_personality_function(&self) -> FunctionValue<'ctx> {
            let i32_ty = self.context.i32_type();
            let personality_ty = i32_ty.fn_type(&[], false);
            get_or_insert_function(self.module, "__gxx_personality_v0", personality_ty)
        }

        fn get_type_info_global(&self, type_id: TypeId) -> PointerValue<'ctx> {
            let name = format!("__ruyi_type_info_{}", type_id);
            let i8_ptr = self.context.i8_type().ptr_type(AddressSpace::default());

            if let Some(global) = self.module.get_global(&name) {
                global.as_pointer_value()
            } else {
                let global = self.module.add_global(i8_ptr, None, &name);
                global.set_linkage(inkwell::module::Linkage::External);
                global.as_pointer_value()
            }
        }
    }

    fn get_or_insert_function<'ctx>(
        module: &Module<'ctx>,
        name: &str,
        fn_type: inkwell::types::FunctionType<'ctx>,
    ) -> FunctionValue<'ctx> {
        module.get_function(name).unwrap_or_else(|| module.add_function(name, fn_type, None))
    }
}
