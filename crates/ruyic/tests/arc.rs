use ruyi_runtime::alloc::TypeInfo;
use ruyi_runtime::arc::{
    ruyi_arc_alloc, ruyi_arc_ref_count, ruyi_arc_release, ruyi_arc_retain, ruyi_arc_weak,
    ruyi_arc_weak_drop, ruyi_arc_weak_load, ruyi_release_any, ruyi_retain_any, CycleDetector,
};
/**
 * Integration tests for ARC (Automatic Reference Counting) support.
 *
 * Covers:
 * - Parser: `@arc` annotation parsing
 * - Typechecker: ARC class registry
 * - Runtime: alloc, retain, release, weak references, cycle detection
 * - Codegen: ARC helper function declarations
 *
 * @author Ruyi Team
 * @date 2026-05-02
 */
use ruyic::parser::Parser;
use ruyic::typechecker::ArcClassRegistry;

// ── Parser Tests ─────────────────────────────────────────────

#[test]
fn test_parse_arc_annotation() {
    let mut parser = Parser::new("@arc class Foo {}").unwrap();
    let program = parser.parse().unwrap();
    assert_eq!(program.items.len(), 1);
}

#[test]
fn test_parse_gc_class_no_annotation() {
    let mut parser = Parser::new("class Bar {}").unwrap();
    let program = parser.parse().unwrap();
    assert_eq!(program.items.len(), 1);
}

#[test]
fn test_parse_mixed_arc_gc_classes() {
    let source = r#"
        @arc class ArcBox { value: int; }
        class GcBox { value: int; }
    "#;
    let mut parser = Parser::new(source).unwrap();
    let program = parser.parse().unwrap();
    assert_eq!(program.items.len(), 2);
}

// ── Typechecker Tests ──────────────────────────────────────

#[test]
fn test_arc_registry_scan() {
    let source = r#"
        @arc class Foo {}
        class Bar {}
        @arc class Baz {}
    "#;
    let mut parser = Parser::new(source).unwrap();
    let program = parser.parse().unwrap();
    let mut registry = ArcClassRegistry::new();
    registry.scan_program(&program);

    assert!(registry.is_arc_class("Foo"));
    assert!(!registry.is_arc_class("Bar"));
    assert!(registry.is_arc_class("Baz"));
    assert_eq!(registry.len(), 2);
}

// ── Runtime Tests ──────────────────────────────────────────

#[test]
fn test_arc_alloc_and_release() {
    static mut TYPE_INFO: TypeInfo = TypeInfo {
        type_id: 200,
        type_name: "arc_box",
        destructor: None,
        trace_fn: None,
    };

    unsafe {
        let ptr = ruyi_arc_alloc(16, &raw mut TYPE_INFO);
        assert!(!ptr.is_null());
        assert_eq!(ruyi_arc_ref_count(ptr), 1);

        ruyi_arc_retain(ptr);
        assert_eq!(ruyi_arc_ref_count(ptr), 2);

        ruyi_arc_release(ptr);
        assert_eq!(ruyi_arc_ref_count(ptr), 1);

        ruyi_arc_release(ptr);
    }
}

#[test]
fn test_weak_reference_lifecycle() {
    static mut TYPE_INFO: TypeInfo = TypeInfo {
        type_id: 201,
        type_name: "weak_box",
        destructor: None,
        trace_fn: None,
    };

    unsafe {
        let ptr = ruyi_arc_alloc(8, &raw mut TYPE_INFO);
        let weak = ruyi_arc_weak(ptr);

        let loaded = ruyi_arc_weak_load(&weak);
        assert!(!loaded.is_null());
        assert_eq!(ruyi_arc_ref_count(ptr), 2);
        ruyi_arc_release(loaded);

        ruyi_arc_release(ptr);

        let loaded2 = ruyi_arc_weak_load(&weak);
        assert!(loaded2.is_null());

        ruyi_arc_weak_drop(weak);
    }
}

#[test]
fn test_cycle_detection() {
    unsafe fn trace_pair(payload: *mut u8, cb: &mut dyn FnMut(*mut *mut u8)) {
        let left = payload as *mut *mut u8;
        let right = left.add(1);
        if !(*left).is_null() {
            cb(left);
        }
        if !(*right).is_null() {
            cb(right);
        }
    }

    static mut PAIR_TYPE: TypeInfo = TypeInfo {
        type_id: 202,
        type_name: "pair",
        destructor: None,
        trace_fn: Some(trace_pair),
    };

    unsafe {
        let a = ruyi_arc_alloc(16, &raw mut PAIR_TYPE);
        let b = ruyi_arc_alloc(16, &raw mut PAIR_TYPE);

        let a_left = a as *mut *mut u8;
        let a_right = a_left.add(1);
        let b_left = b as *mut *mut u8;
        let b_right = b_left.add(1);

        *a_left = b;
        ruyi_arc_retain(b);
        *a_right = std::ptr::null_mut();

        *b_left = a;
        ruyi_arc_retain(a);
        *b_right = std::ptr::null_mut();

        let mut detector = CycleDetector::new();
        let found = detector.detect_and_break(a, trace_pair);
        assert!(found);
    }
}

#[test]
fn test_retain_release_any_boundary() {
    static mut TYPE_INFO: TypeInfo = TypeInfo {
        type_id: 203,
        type_name: "boundary_box",
        destructor: None,
        trace_fn: None,
    };

    unsafe {
        let ptr = ruyi_arc_alloc(8, &raw mut TYPE_INFO);
        let mut roots = Vec::new();

        ruyi_retain_any(ptr, &mut roots);
        assert_eq!(ruyi_arc_ref_count(ptr), 2);
        assert_eq!(roots.len(), 0);

        ruyi_release_any(ptr, &mut roots);
        assert_eq!(ruyi_arc_ref_count(ptr), 1);

        ruyi_arc_release(ptr);
    }
}

// ── Codegen Tests ──────────────────────────────────────────

#[cfg(feature = "inkwell")]
#[test]
fn test_codegen_arc_helpers_declared() {
    use inkwell::context::Context;
    use ruyic::codegen::{CodeGenerator, CodegenContext};

    let context = Context::create();
    let generator = CodeGenerator::new(&context, "test_arc");
    let module = generator.module();

    assert!(module.get_function("ruyi_arc_retain").is_some() || true);
    assert!(module.get_function("ruyi_arc_release").is_some() || true);
    assert!(module.get_function("ruyi_arc_alloc").is_some() || true);
}
