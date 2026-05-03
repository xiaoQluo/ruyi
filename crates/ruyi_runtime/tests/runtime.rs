use ruyi_runtime::*;

#[test]
fn test_ruyi_alloc_roundtrip() {
    static mut TYPE_INFO: TypeInfo = TypeInfo {
        type_id: 100,
        type_name: "test_object",
        destructor: None,
        trace_fn: None,
    };

    unsafe {
        let ptr = ruyi_alloc(32, &raw mut TYPE_INFO, MemoryStrategy::GC);
        assert!(!ptr.is_null());

        let header = GcObjectHeader::from_payload(ptr);
        assert_eq!((*header).size, 32);
        assert_eq!((*header).strategy(), MemoryStrategy::GC);
        assert_eq!((*(*header).type_info).type_id, 100);

        ruyi_dealloc(ptr);
    }
}

#[test]
fn test_ruyi_alloc_arc() {
    static mut TYPE_INFO: TypeInfo = TypeInfo {
        type_id: 101,
        type_name: "arc_object",
        destructor: None,
        trace_fn: None,
    };

    unsafe {
        let ptr = ruyi_alloc(16, &raw mut TYPE_INFO, MemoryStrategy::ARC);
        let header = GcObjectHeader::from_payload(ptr);
        assert_eq!((*header).strategy(), MemoryStrategy::ARC);
        assert_eq!((*header).ref_count(), 1);

        (*header).retain();
        assert_eq!((*header).ref_count(), 2);
        (*header).release();
        assert_eq!((*header).ref_count(), 1);

        ruyi_dealloc(ptr);
    }
}

#[test]
fn test_mark_sweep_lifecycle() {
    static mut TYPE_INFO: TypeInfo = TypeInfo {
        type_id: 200,
        type_name: "gc_box",
        destructor: None,
        trace_fn: None,
    };

    let collector = MarkSweepCollector::new();

    unsafe {
        let a = collector.allocate(8, &raw mut TYPE_INFO);
        let _b = collector.allocate(8, &raw mut TYPE_INFO);
        assert_eq!(collector.object_count(), 2);

        collector.add_root(a);
        collector.collect();
        assert_eq!(collector.object_count(), 1); // only a survives

        collector.remove_root(a);
        collector.collect();
        assert_eq!(collector.object_count(), 0); // all dead
    }
}

#[test]
fn test_exception_table_integration() {
    let mut table = FunctionExceptionTable::new("main");
    table.add_entry(
        ExceptionTableEntry::new(0, 50, 100)
            .catch(builtin_type_ids::ERROR, 200)
            .catch(builtin_type_ids::TYPE_ERROR, 300)
            .cleanup(),
    );

    assert!(table.entry_for_pc(25).is_some());
    assert!(table.entry_for_pc(50).is_none());

    let entry = table.entry_for_pc(10).unwrap();
    assert!(entry.has_cleanup);
    assert_eq!(
        entry.matching_handler(builtin_type_ids::ERROR),
        Some(200)
    );
    assert_eq!(
        entry.matching_handler(builtin_type_ids::RUNTIME_ERROR),
        None
    );
}

#[test]
fn test_landing_pad_descriptor_builder() {
    let desc = LandingPadDescriptor::new()
        .add_action(LandingPadAction::Catch(builtin_type_ids::ERROR))
        .add_action(LandingPadAction::Catch(builtin_type_ids::TYPE_ERROR))
        .add_action(LandingPadAction::Cleanup)
        .set_catch_block(100)
        .set_cleanup_block(200);

    assert_eq!(desc.actions.len(), 3);
    assert_eq!(desc.catch_block, 100);
    assert_eq!(desc.cleanup_block, 200);
}

#[cfg(feature = "inkwell")]
#[test]
fn test_ruyi_context_inkwell_types() {
    let ctx = RuyiContext::new();

    let int_ty = ctx.ruyi_type(RuyiType::Int);
    assert!(matches!(int_ty, inkwell::types::BasicTypeEnum::IntType(t) if t.get_bit_width() == 64));

    let float_ty = ctx.ruyi_type(RuyiType::Float);
    assert!(matches!(float_ty, inkwell::types::BasicTypeEnum::FloatType(_)));

    let bool_ty = ctx.ruyi_type(RuyiType::Bool);
    assert!(matches!(bool_ty, inkwell::types::BasicTypeEnum::IntType(t) if t.get_bit_width() == 1));

    let dyn_ty = ctx.ruyi_type(RuyiType::Dyn);
    assert!(matches!(dyn_ty, inkwell::types::BasicTypeEnum::StructType(t) if t.count_fields() == 2));
}
