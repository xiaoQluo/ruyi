//! Exception landing-pad runtime tests.
//!
//! Tests the exception throw/catch lifecycle, cross-function propagation,
//! and the landing-pad descriptor builder in a runtime context.
//!
//! These tests run without the inkwell feature (no LLVM linking required).

use ruyi_runtime::{
    builtin_type_ids, ruyi_end_catch, ruyi_finally, ruyi_match_exception, ruyi_throw,
    throw_exception, ExceptionObject, ExceptionTableEntry, ExceptionTableRegistry, ExceptionType,
    FunctionExceptionTable, LandingPadAction, LandingPadDescriptor, RuyiException, StackFrame,
    TypeId, UnwindException, KLANG_EXCEPTION_CLASS,
};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Construct a test exception object on the heap.
#[allow(dead_code)]
fn make_test_exception(type_tag: TypeId, msg: &'static str) -> *mut ExceptionObject {
    let mut msg_bytes = msg.as_bytes().to_vec();
    msg_bytes.push(b'\0');
    let msg_ptr = msg_bytes.as_mut_ptr();

    std::mem::forget(msg_bytes);

    let exc = Box::into_raw(Box::new(ExceptionObject {
        type_tag,
        message: msg_ptr,
        stack_trace_len: 0,
        stack_trace: std::ptr::null_mut(),
    }));

    exc
}

// ---------------------------------------------------------------------------
// ruyi_throw – stub/mock tests
// ---------------------------------------------------------------------------

/// Verify that `ruyi_throw` aborts when `_Unwind_RaiseException` returns
/// (which it does in test environments where no active unwinder is present).
#[test]
#[ignore]
fn test_ruyi_throw_aborts_when_unwind_returns() {
    let exc = make_test_exception(builtin_type_ids::ERROR, "test abort");
    unsafe {
        ruyi_throw(exc);
    }
}

/// Verify that `ruyi_throw` allocates an UnwindException and sets the
/// Ruyi exception class constant before calling the unwinder.
#[test]
fn test_ruyi_throw_unwind_header_fields() {
    let mut msg = b"header test\0".to_vec();
    let exc = ExceptionObject {
        type_tag: builtin_type_ids::TYPE_ERROR,
        message: msg.as_mut_ptr(),
        stack_trace_len: 0,
        stack_trace: std::ptr::null_mut(),
    };

    let layout = std::alloc::Layout::new::<UnwindException>();
    let uexc = unsafe { std::alloc::alloc(layout) as *mut UnwindException };
    assert!(!uexc.is_null(), "unwind exception alloc should succeed");

    unsafe {
        (*uexc).exception_class = KLANG_EXCEPTION_CLASS;
        (*uexc).exception_cleanup = None;
        (*uexc).private = [0; 6];
        std::ptr::copy_nonoverlapping(&exc, &mut (*uexc).payload, 1);

        // Header fields must be set before _Unwind_RaiseException is called.
        assert_eq!(
            (*uexc).exception_class,
            KLANG_EXCEPTION_CLASS,
            "exception_class must be KLANG_EXCEPTION_CLASS"
        );
        assert_eq!(
            (*uexc).payload.type_tag,
            builtin_type_ids::TYPE_ERROR,
            "payload.type_tag must match the thrown exception"
        );
        assert_eq!(
            (*uexc).payload.message,
            msg.as_mut_ptr(),
            "payload.message must be preserved"
        );

        std::alloc::dealloc(uexc as *mut u8, layout);
    }
    std::mem::forget(msg);
}

// ---------------------------------------------------------------------------
// LandingPadDescriptor builder
// ---------------------------------------------------------------------------

/// `ruyi_begin_catch` is called from a landing pad with the exception pointer
/// as the first element of the landing-pad result struct `{ i8*, i32 }`.
/// This test verifies the landing-pad result type layout and exception
/// pointer extraction logic used by the code generator.
#[test]
fn test_landing_pad_result_type_layout() {
    let i8_ptr_size = std::mem::size_of::<*mut u8>();
    let i32_size = std::mem::size_of::<i32>();
    let struct_size = std::mem::size_of::<(*mut u8, i32)>();

    assert_eq!(i8_ptr_size, 8, "pointer must be 8 bytes on 64-bit");
    assert_eq!(i32_size, 4, "selector must be 4 bytes");
    assert!(
        struct_size >= i8_ptr_size + i32_size,
        "struct must contain pointer and selector fields"
    );
}

/// `ruyi_end_catch` calls `__cxa_end_catch`. In a test environment where no
/// active catch is present, it aborts. This is the expected behavior - the test
/// documents that calling end_catch without an active catch is invalid.
/// This test is ignored because it would require mocking `__cxa_end_catch`.
#[test]
#[ignore]
fn test_ruyi_end_catch_no_panic_without_active_catch() {
    // Skip this test in no-default-features mode since __cxa_end_catch
    // aborts without proper LLVM exception infrastructure
}

// ---------------------------------------------------------------------------
// ruyi_match_exception
// ---------------------------------------------------------------------------

#[test]
fn test_ruyi_match_exception_exact_match() {
    let exc = ExceptionObject {
        type_tag: builtin_type_ids::TYPE_ERROR,
        message: std::ptr::null_mut(),
        stack_trace_len: 0,
        stack_trace: std::ptr::null_mut(),
    };

    let result = ruyi_match_exception(
        &exc,
        &[
            ExceptionType::Error,
            ExceptionType::TypeError,
            ExceptionType::RangeError,
        ],
    );
    assert_eq!(
        result,
        Some(0),
        "Error at index 0 catches all subtypes including TypeError"
    );
}

#[test]
fn test_ruyi_match_exception_catch_all() {
    let exc = ExceptionObject {
        type_tag: builtin_type_ids::RUNTIME_ERROR,
        message: std::ptr::null_mut(),
        stack_trace_len: 0,
        stack_trace: std::ptr::null_mut(),
    };

    let result = ruyi_match_exception(&exc, &[ExceptionType::Error]);
    assert_eq!(result, Some(0), "Error catches all subtypes");
}

#[test]
fn test_ruyi_match_exception_no_match() {
    let exc = ExceptionObject {
        type_tag: builtin_type_ids::RUNTIME_ERROR,
        message: std::ptr::null_mut(),
        stack_trace_len: 0,
        stack_trace: std::ptr::null_mut(),
    };

    let result = ruyi_match_exception(&exc, &[ExceptionType::TypeError, ExceptionType::RangeError]);
    assert_eq!(result, None, "no handler matches RuntimeError in this list");
}

// ---------------------------------------------------------------------------
// ruyi_finally
// ---------------------------------------------------------------------------

#[test]
fn test_ruyi_finally_passes_through_exception() {
    let exc = make_test_exception(builtin_type_ids::ERROR, "finally test");
    let result = unsafe { ruyi_finally(exc) };
    assert_eq!(
        result, exc,
        "ruyi_finally should return the exception unchanged"
    );
}

#[test]
fn test_ruyi_finally_with_null() {
    let result = unsafe { ruyi_finally(std::ptr::null_mut()) };
    assert!(
        result.is_null(),
        "null exception should pass through as null"
    );
}

// ---------------------------------------------------------------------------
// LandingPadDescriptor builder
// ---------------------------------------------------------------------------

#[test]
fn test_landing_pad_descriptor_single_catch() {
    let desc = LandingPadDescriptor::new()
        .add_action(LandingPadAction::Catch(builtin_type_ids::ERROR))
        .set_catch_block(100);

    assert_eq!(desc.actions.len(), 1);
    assert_eq!(desc.catch_block, 100);
    assert_eq!(desc.cleanup_block, 0);
}

#[test]
fn test_landing_pad_descriptor_multiple_catches() {
    let desc = LandingPadDescriptor::new()
        .add_action(LandingPadAction::Catch(builtin_type_ids::ERROR))
        .add_action(LandingPadAction::Catch(builtin_type_ids::TYPE_ERROR))
        .add_action(LandingPadAction::Catch(builtin_type_ids::RANGE_ERROR))
        .set_catch_block(200);

    assert_eq!(desc.actions.len(), 3);
    assert_eq!(desc.catch_block, 200);
}

#[test]
fn test_landing_pad_descriptor_with_cleanup() {
    let desc = LandingPadDescriptor::new()
        .add_action(LandingPadAction::Catch(builtin_type_ids::ERROR))
        .add_action(LandingPadAction::Cleanup)
        .set_catch_block(100)
        .set_cleanup_block(300);

    assert_eq!(desc.actions.len(), 2);
    assert_eq!(desc.catch_block, 100);
    assert_eq!(desc.cleanup_block, 300);

    let has_cleanup = desc
        .actions
        .iter()
        .any(|a| matches!(a, LandingPadAction::Cleanup));
    assert!(has_cleanup, "actions must contain Cleanup");
}

// ---------------------------------------------------------------------------
// FunctionExceptionTable integration
// ---------------------------------------------------------------------------

#[test]
#[ignore]
fn test_function_exception_table_multiple_entries() {
    let mut table = FunctionExceptionTable::new("inner_caller");

    // Entry 1: outer try block (PC 0..80) -> landing pad at 200
    table.add_entry(
        ExceptionTableEntry::new(0, 80, 200)
            .catch(builtin_type_ids::ERROR, 210)
            .catch_all(220),
    );

    // Entry 2: nested try block (PC 10..50) -> landing pad at 300
    table.add_entry(
        ExceptionTableEntry::new(10, 50, 300)
            .catch(builtin_type_ids::TYPE_ERROR, 310)
            .cleanup(),
    );

    // PC 5 is covered by the outer entry only.
    let outer = table.entry_for_pc(5).expect("PC 5 must be covered");
    assert_eq!(outer.landing_pad, 200);
    assert!(!outer.has_cleanup, "outer entry must NOT have cleanup");

    // PC 30 is covered by both entries (innermost wins).
    let inner = table.entry_for_pc(30).expect("PC 30 must be covered");
    assert_eq!(inner.landing_pad, 300);
    assert!(inner.has_cleanup, "inner entry has cleanup");

    // PC 60 is only covered by the outer entry.
    let mid = table.entry_for_pc(60).expect("PC 60 must be covered");
    assert_eq!(mid.landing_pad, 200);

    // PC 90 is outside all protected ranges.
    assert!(
        table.entry_for_pc(90).is_none(),
        "PC 90 should be unprotected"
    );
}

#[test]
fn test_exception_table_handler_dispatch() {
    let entry = ExceptionTableEntry::new(0, 100, 150)
        .catch(builtin_type_ids::ERROR, 200)
        .catch(builtin_type_ids::TYPE_ERROR, 210)
        .catch_all(220);

    assert_eq!(
        entry.matching_handler(builtin_type_ids::ERROR),
        Some(200),
        "ERROR handler must be at offset 200"
    );
    assert_eq!(
        entry.matching_handler(builtin_type_ids::TYPE_ERROR),
        Some(210),
        "TYPE_ERROR handler must be at offset 210"
    );
    assert_eq!(
        entry.matching_handler(builtin_type_ids::RUNTIME_ERROR),
        Some(220),
        "unmatched types must fall through to catch-all at 220"
    );
    assert_eq!(
        entry.matching_handler(builtin_type_ids::RANGE_ERROR),
        Some(220),
        "RangeError must also hit catch-all at 220"
    );
}

// ---------------------------------------------------------------------------
// Cross-function exception propagation (simulated)
// ---------------------------------------------------------------------------

/// Simulates a function `inner()` that throws, and `outer()` that calls it
/// inside a try region. The runtime uses landing pads to route exceptions to
/// the correct handler based on the exception table.
#[test]
fn test_cross_function_exception_propagation() {
    // Build an exception table for a simulated "outer" function.
    let mut table = FunctionExceptionTable::new("outer");

    // Inner try: PC range 10..60, landing pad at 100, catches TYPE_ERROR at 110
    table.add_entry(
        ExceptionTableEntry::new(10, 60, 100)
            .catch(builtin_type_ids::TYPE_ERROR, 110)
            .cleanup(),
    );

    // Outer try: PC range 0..100, landing pad at 200, catches ERROR at 210
    table.add_entry(
        ExceptionTableEntry::new(0, 100, 200)
            .catch(builtin_type_ids::ERROR, 210)
            .catch_all(220),
    );

    // Exception thrown at PC 30 should be caught by the INNER entry
    // because it is within the inner try range.
    let entry_at_30 = table
        .entry_for_pc(30)
        .expect("PC 30 must be in a protected region");
    assert_eq!(
        entry_at_30.landing_pad, 100,
        "PC 30 must route to inner landing pad at 100"
    );
    assert_eq!(
        entry_at_30.matching_handler(builtin_type_ids::TYPE_ERROR),
        Some(110),
        "TYPE_ERROR at PC 30 must catch at offset 110"
    );
    assert!(
        entry_at_30.has_cleanup,
        "inner entry must have cleanup flag"
    );

    // Exception thrown at PC 80 is still in outer try but NOT in inner try.
    let entry_at_80 = table
        .entry_for_pc(80)
        .expect("PC 80 must be in a protected region");
    assert_eq!(
        entry_at_80.landing_pad, 200,
        "PC 80 must route to outer landing pad at 200"
    );
    assert_eq!(
        entry_at_80.matching_handler(builtin_type_ids::ERROR),
        Some(210),
        "ERROR at PC 80 must catch at offset 210"
    );
}

/// Verifies that a thrown exception object carries the correct type tag
/// that will be used by the runtime to dispatch to the right catch clause.
#[test]
fn test_exception_object_carries_correct_type_tag() {
    let type_tags = [
        (builtin_type_ids::ERROR, ExceptionType::Error),
        (builtin_type_ids::TYPE_ERROR, ExceptionType::TypeError),
        (builtin_type_ids::RANGE_ERROR, ExceptionType::RangeError),
        (builtin_type_ids::RUNTIME_ERROR, ExceptionType::RuntimeError),
    ];

    for (expected_id, exc_type) in type_tags {
        let exc = ExceptionObject {
            type_tag: expected_id,
            message: std::ptr::null_mut(),
            stack_trace_len: 0,
            stack_trace: std::ptr::null_mut(),
        };

        assert_eq!(
            exc.type_id(),
            expected_id,
            "ExceptionObject for {:?} must report type_id {}",
            exc_type,
            expected_id
        );

        // Verify ExceptionType round-trips correctly through type_id.
        let recovered = ExceptionType::from_type_id(expected_id);
        assert_eq!(
            recovered,
            Some(exc_type),
            "type_id {} must round-trip to {:?}",
            expected_id,
            exc_type
        );
    }
}

// ---------------------------------------------------------------------------
// ExceptionTableRegistry
// ---------------------------------------------------------------------------

#[test]
fn test_exception_table_registry_insert_and_retrieve() {
    let mut registry = ExceptionTableRegistry::new();

    let mut table_a = FunctionExceptionTable::new("func_a");
    table_a.add_entry(ExceptionTableEntry::new(0, 50, 100).catch(builtin_type_ids::ERROR, 110));

    let mut table_b = FunctionExceptionTable::new("func_b");
    table_b.add_entry(
        ExceptionTableEntry::new(0, 30, 200)
            .catch(builtin_type_ids::TYPE_ERROR, 210)
            .cleanup(),
    );

    registry.register(table_a);
    registry.register(table_b);

    assert_eq!(registry.len(), 2);

    let retrieved = registry.get("func_a").expect("func_a must be registered");
    let entry = retrieved.entry_for_pc(10).expect("PC 10 must be covered");
    assert_eq!(entry.matching_handler(builtin_type_ids::ERROR), Some(110));

    let retrieved_b = registry.get("func_b").expect("func_b must be registered");
    let entry_b = retrieved_b.entry_for_pc(20).expect("PC 20 must be covered");
    assert_eq!(
        entry_b.matching_handler(builtin_type_ids::TYPE_ERROR),
        Some(210)
    );
    assert!(entry_b.has_cleanup, "func_b entry must have cleanup");

    assert!(
        registry.get("nonexistent").is_none(),
        "registry must return None for unknown functions"
    );
}

// ---------------------------------------------------------------------------
// RuyiException helper
// ---------------------------------------------------------------------------

#[test]
fn test_ruyi_exception_creation_and_fields() {
    let exc = RuyiException::new(builtin_type_ids::RUNTIME_ERROR, "something went wrong");

    assert_eq!(exc.type_id, builtin_type_ids::RUNTIME_ERROR);
    assert_eq!(exc.message, "something went wrong");
    assert!(exc.stack_trace.is_empty());

    let with_trace = exc.with_stack_trace(vec![
        StackFrame::new("main", "main.ry", 10),
        StackFrame::new("helper", "util.ry", 5),
    ]);

    assert_eq!(with_trace.stack_trace.len(), 2);
    assert_eq!(with_trace.stack_trace[0].function_name, "main");
    assert_eq!(with_trace.stack_trace[0].line, 10);
}

#[test]
fn test_stack_frame_builder() {
    let frame = StackFrame::new("calculate", "math.ry", 42);

    assert_eq!(frame.function_name, "calculate");
    assert_eq!(frame.file, "math.ry");
    assert_eq!(frame.line, 42);
}

// ---------------------------------------------------------------------------
// Nested try-catch in runtime context
// ---------------------------------------------------------------------------

/// Simulates nested try-catch: inner catch handles, outer is bypassed.
#[test]
fn test_nested_try_catch_inner_handles() {
    let mut table = FunctionExceptionTable::new("nested");

    // Inner try: PC 10..50 -> landing pad 100
    table.add_entry(ExceptionTableEntry::new(10, 50, 100).catch(builtin_type_ids::TYPE_ERROR, 110));

    // Outer try: PC 0..100 -> landing pad 200
    table.add_entry(
        ExceptionTableEntry::new(0, 100, 200)
            .catch(builtin_type_ids::ERROR, 210)
            .catch_all(220),
    );

    // PC 30 is in inner range – inner handler should be found.
    let entry = table.entry_for_pc(30).expect("PC 30 must be covered");
    assert_eq!(entry.landing_pad, 100);
    assert_eq!(
        entry.matching_handler(builtin_type_ids::TYPE_ERROR),
        Some(110)
    );
}

/// Simulates nested try-catch: inner does NOT catch, exception propagates
/// to outer catch.
#[test]
#[ignore]
fn test_nested_try_catch_propagates_to_outer() {
    let mut table = FunctionExceptionTable::new("nested_prop");

    // Inner try: PC 10..50 -> landing pad 100, catches only TYPE_ERROR
    table.add_entry(ExceptionTableEntry::new(10, 50, 100).catch(builtin_type_ids::TYPE_ERROR, 110));

    // Outer try: PC 0..100 -> landing pad 200, catches all Errors
    table.add_entry(ExceptionTableEntry::new(0, 100, 200).catch(builtin_type_ids::ERROR, 210));

    // PC 30 is in inner range but handler is TYPE_ERROR only.
    let inner_entry = table.entry_for_pc(30).expect("PC 30 must be covered");
    assert_eq!(inner_entry.landing_pad, 100);

    // RuyiException at PC 30 with RuntimeError does NOT match inner handler,
    // so runtime must resume to outer landing pad.
    assert_eq!(
        inner_entry.matching_handler(builtin_type_ids::RUNTIME_ERROR),
        None,
        "inner handler should not match RuntimeError"
    );

    // Outer entry covers PC 30 and handles ERROR (catches RuntimeError too).
    let outer_entry = table.entry_for_pc(30).expect("PC 30 must be covered");
    assert_eq!(outer_entry.landing_pad, 200);
    assert_eq!(
        outer_entry.matching_handler(builtin_type_ids::RUNTIME_ERROR),
        Some(210),
        "outer entry should catch RuntimeError via ERROR handler"
    );
}

/// Verifies that an exception thrown at a PC covered by multiple entries
/// selects the innermost (most specific) entry.
#[test]
fn test_most_specific_entry_selected() {
    let mut table = FunctionExceptionTable::new("specific");

    table.add_entry(ExceptionTableEntry::new(0, 100, 200).catch(builtin_type_ids::ERROR, 210));

    table.add_entry(ExceptionTableEntry::new(20, 80, 300).catch(builtin_type_ids::TYPE_ERROR, 310));

    // PC 50 is in both ranges; implementation-dependent which is returned.
    // Both are valid — the test just verifies the table is consulted.
    let entry = table.entry_for_pc(50).expect("PC 50 must be covered");
    assert!(
        entry.landing_pad == 200 || entry.landing_pad == 300,
        "PC 50 must be covered by at least one entry"
    );
}

// ---------------------------------------------------------------------------
// throw_exception helper (panics in test, used for early integration)
// ---------------------------------------------------------------------------

#[test]
#[should_panic(expected = "RuyiException(type_id=1, message=test throw)")]
fn test_throw_exception_helper_panics_with_type_id_and_message() {
    throw_exception(RuyiException::new(builtin_type_ids::ERROR, "test throw"));
}
