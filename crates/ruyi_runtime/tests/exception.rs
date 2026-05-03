use ruyi_runtime::*;

#[test]
fn test_exception_type_ids() {
    assert_eq!(ExceptionType::Error.type_id(), builtin_type_ids::ERROR);
    assert_eq!(
        ExceptionType::TypeError.type_id(),
        builtin_type_ids::TYPE_ERROR
    );
    assert_eq!(
        ExceptionType::RangeError.type_id(),
        builtin_type_ids::RANGE_ERROR
    );
    assert_eq!(
        ExceptionType::RuntimeError.type_id(),
        builtin_type_ids::RUNTIME_ERROR
    );
}

#[test]
fn test_exception_type_matching() {
    assert!(ExceptionType::Error.matches(ExceptionType::TypeError));
    assert!(ExceptionType::Error.matches(ExceptionType::Error));
    assert!(ExceptionType::TypeError.matches(ExceptionType::TypeError));
    assert!(!ExceptionType::TypeError.matches(ExceptionType::RangeError));
}

#[test]
fn test_exception_object_layout() {
    let exc = ExceptionObject {
        type_tag: ExceptionType::RuntimeError.type_id(),
        message: std::ptr::null_mut(),
        stack_trace_len: 0,
        stack_trace: std::ptr::null_mut(),
    };
    assert_eq!(exc.type_id(), ExceptionType::RuntimeError.type_id());
    assert!(exc.message_ptr().is_null());
}

#[test]
fn test_ruyi_match_exception() {
    let exc = ExceptionObject {
        type_tag: ExceptionType::RangeError.type_id(),
        message: std::ptr::null_mut(),
        stack_trace_len: 0,
        stack_trace: std::ptr::null_mut(),
    };

    assert_eq!(
        ruyi_match_exception(&exc, &[ExceptionType::Error, ExceptionType::RangeError]),
        Some(0)
    );
    assert_eq!(
        ruyi_match_exception(&exc, &[ExceptionType::TypeError]),
        None
    );
}

#[test]
fn test_ruyi_finally_preserves_exception() {
    let exc = ExceptionObject {
        type_tag: ExceptionType::Error.type_id(),
        message: std::ptr::null_mut(),
        stack_trace_len: 0,
        stack_trace: std::ptr::null_mut(),
    };

    unsafe {
        let result = ruyi_finally(&exc as *const _ as *mut _);
        assert!(!result.is_null());
        assert_eq!((*result).type_tag, ExceptionType::Error.type_id());
    }
}

#[test]
fn test_ruyi_finally_with_null() {
    unsafe {
        let result = ruyi_finally(std::ptr::null_mut());
        assert!(result.is_null());
    }
}

#[test]
fn test_exception_propagation_simulation() {
    let inner_exc = ExceptionObject {
        type_tag: ExceptionType::TypeError.type_id(),
        message: std::ptr::null_mut(),
        stack_trace_len: 0,
        stack_trace: std::ptr::null_mut(),
    };

    let outer_exc = ExceptionObject {
        type_tag: ExceptionType::RangeError.type_id(),
        message: std::ptr::null_mut(),
        stack_trace_len: 0,
        stack_trace: std::ptr::null_mut(),
    };

    let inner_catch = [ExceptionType::TypeError];
    assert_eq!(ruyi_match_exception(&inner_exc, &inner_catch), Some(0));
    assert_eq!(ruyi_match_exception(&outer_exc, &inner_catch), None);

    let outer_catch = [ExceptionType::Error];
    assert_eq!(ruyi_match_exception(&inner_exc, &outer_catch), Some(0));
    assert_eq!(ruyi_match_exception(&outer_exc, &outer_catch), Some(0));
}

#[test]
fn test_finally_guarantee_on_uncaught() {
    let exc = ExceptionObject {
        type_tag: ExceptionType::RuntimeError.type_id(),
        message: std::ptr::null_mut(),
        stack_trace_len: 0,
        stack_trace: std::ptr::null_mut(),
    };

    unsafe {
        let pending = ruyi_finally(&exc as *const _ as *mut _);
        assert!(!pending.is_null());
        assert_eq!((*pending).type_tag, ExceptionType::RuntimeError.type_id());
    }
}

#[cfg(feature = "inkwell")]
mod inkwell_tests {
    use super::*;
    use inkwell::context::Context;
    use ruyi_runtime::ExceptionRuntime;
    use ruyi_runtime::LandingPadGenerator;

    #[test]
    fn test_landing_pad_generation() {
        let context = Context::create();
        let module = context.create_module("test");
        let builder = context.create_builder();

        let fn_type = context.void_type().fn_type(&[], false);
        let func = module.add_function("test_fn", fn_type, None);
        let bb = context.append_basic_block(func, "entry");
        builder.position_at_end(bb);

        let lpad_gen = LandingPadGenerator::new(&context, &module, &builder);

        let lpad = lpad_gen.build_landing_pad(
            &[builtin_type_ids::ERROR, builtin_type_ids::TYPE_ERROR],
            true,
            "lpad",
        );

        assert!(lpad.is_struct_value());
    }

    #[test]
    fn test_runtime_function_declarations() {
        let context = Context::create();
        let module = context.create_module("test");
        let builder = context.create_builder();

        let fn_type = context.void_type().fn_type(&[], false);
        let func = module.add_function("test_fn", fn_type, None);
        let bb = context.append_basic_block(func, "entry");
        builder.position_at_end(bb);

        let _exc_runtime = ExceptionRuntime::new(&context, &module, &builder);

        assert!(module.get_function("ruyi_throw").is_some());
        assert!(module.get_function("ruyi_begin_catch").is_some());
        assert!(module.get_function("ruyi_end_catch").is_some());
        assert!(module.get_function("ruyi_finally").is_some());
    }

    #[test]
    fn test_type_info_globals() {
        let context = Context::create();
        let module = context.create_module("test");
        let builder = context.create_builder();

        let fn_type = context.void_type().fn_type(&[], false);
        let func = module.add_function("test_fn", fn_type, None);
        let bb = context.append_basic_block(func, "entry");
        builder.position_at_end(bb);

        let lpad_gen = LandingPadGenerator::new(&context, &module, &builder);
        let _typeid = lpad_gen.build_eh_typeid_for(builtin_type_ids::ERROR);

        assert!(module.get_global("__ruyi_type_info_1").is_some());
    }

    #[test]
    fn test_extract_exception_ptr_and_selector() {
        let context = Context::create();
        let module = context.create_module("test");
        let builder = context.create_builder();

        let fn_type = context.void_type().fn_type(&[], false);
        let func = module.add_function("test_fn", fn_type, None);
        let bb = context.append_basic_block(func, "entry");
        builder.position_at_end(bb);

        let lpad_gen = LandingPadGenerator::new(&context, &module, &builder);
        let lpad = lpad_gen.build_landing_pad(&[builtin_type_ids::ERROR], false, "lpad");

        let _exc_ptr = lpad_gen.extract_exception_ptr(lpad);
        let _selector = lpad_gen.extract_selector(lpad);
    }

    #[test]
    fn test_try_region_landing_pad() {
        let context = Context::create();
        let module = context.create_module("test");
        let builder = context.create_builder();

        let fn_type = context.void_type().fn_type(&[], false);
        let func = module.add_function("test_fn", fn_type, None);
        let entry = context.append_basic_block(func, "entry");
        let catch_bb = context.append_basic_block(func, "catch");
        let finally_bb = context.append_basic_block(func, "finally");
        let resume_bb = context.append_basic_block(func, "resume");

        builder.position_at_end(entry);

        let exc_runtime = ExceptionRuntime::new(&context, &module, &builder);
        let lpad = exc_runtime.build_try_region(
            &[(ExceptionType::Error, catch_bb)],
            Some(finally_bb),
            resume_bb,
        );

        assert!(lpad.is_struct_value());
    }
}
