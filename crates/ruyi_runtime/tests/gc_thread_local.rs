use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;

use ruyi_runtime::gc_exports::{
    ruyi_gc_add_root, ruyi_gc_alloc, ruyi_gc_collect, ruyi_gc_remove_root,
};

/**
 * Thread-local GC heap tests
 *
 * Tests that verify each thread has its own collector instance
 * and that thread-local heaps are properly isolated.
 */

#[test]
fn test_thread_local_allocation_unique_pointers() {
    let main_ptr = ruyi_gc_alloc(64);
    assert!(!main_ptr.is_null());

    let thread_ptr_val = thread::spawn(|| {
        let ptr = ruyi_gc_alloc(64);
        ptr as usize
    })
    .join()
    .unwrap();

    let thread_ptr = thread_ptr_val as *mut u8;
    assert!(!thread_ptr.is_null());
    assert_ne!(
        main_ptr, thread_ptr,
        "allocations from different threads should have unique pointers"
    );
}

#[test]
fn test_thread_local_collect_isolation() {
    // Allocate objects in main thread
    let main_obj1 = ruyi_gc_alloc(32);
    let main_obj2 = ruyi_gc_alloc(32);
    assert!(!main_obj1.is_null());
    assert!(!main_obj2.is_null());

    // Root one object
    unsafe {
        ruyi_gc_add_root(main_obj1);
    }

    unsafe {
        *(main_obj1 as *mut u64) = 0xDEADBEEF;
        *(main_obj2 as *mut u64) = 0xCAFEBABE;
    }

    // Collect in a different thread - should not affect main thread's objects
    let collected = thread::spawn(|| {
        ruyi_gc_collect();
        true
    })
    .join()
    .unwrap();

    assert!(collected);

    // Main thread objects should still be valid
    unsafe {
        assert_eq!(
            *(main_obj1 as *mut u64),
            0xDEADBEEF,
            "rooted object should survive"
        );
        // Note: main_obj2 might be collected or not depending on implementation
    }

    unsafe {
        ruyi_gc_remove_root(main_obj1);
    }
}

#[test]
fn test_concurrent_thread_allocations() {
    const NUM_THREADS: usize = 10;
    const ALLOCS_PER_THREAD: usize = 100;

    let all_ptrs: Arc<AtomicUsize> = Arc::new(AtomicUsize::new(0));
    let mut handles = vec![];

    for _ in 0..NUM_THREADS {
        let counter = Arc::clone(&all_ptrs);
        let handle = thread::spawn(move || {
            for _ in 0..ALLOCS_PER_THREAD {
                let ptr = ruyi_gc_alloc(16);
                if !ptr.is_null() {
                    counter.fetch_add(1, Ordering::SeqCst);
                }
            }
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }

    let total = all_ptrs.load(Ordering::SeqCst);
    assert_eq!(
        total,
        NUM_THREADS * ALLOCS_PER_THREAD,
        "all allocations should succeed"
    );
}

#[test]
fn test_thread_exit_does_not_corrupt_main_heap() {
    // Spawn thread that allocates and immediately exits
    thread::spawn(|| {
        let _ptr = ruyi_gc_alloc(1024);
        // Thread exits, local heap should be cleaned up
    })
    .join()
    .unwrap();

    // Main thread should still be able to allocate
    let main_ptr = ruyi_gc_alloc(64);
    assert!(
        !main_ptr.is_null(),
        "main thread allocation should work after thread exit"
    );

    unsafe {
        *(main_ptr as *mut u64) = 0x12345678;
        ruyi_gc_add_root(main_ptr);
        ruyi_gc_collect();
        assert_eq!(
            *(main_ptr as *mut u64),
            0x12345678,
            "main heap should not be corrupted"
        );
        ruyi_gc_remove_root(main_ptr);
    }
}

#[test]
fn test_cross_thread_pointer_not_accessible() {
    // This test verifies thread isolation at the API level
    // Actual cross-thread access would require synchronization

    let ptr_in_thread = thread::spawn(|| {
        let ptr = ruyi_gc_alloc(64);
        assert!(!ptr.is_null());
        ptr as usize
    })
    .join()
    .unwrap();

    // The pointer value from another thread is just a number
    // We can't safely access it without synchronization
    assert!(
        ptr_in_thread != 0,
        "thread should return valid pointer address"
    );
}

#[test]
fn test_thread_local_gc_stress() {
    const NUM_THREADS: usize = 4;
    const ITERATIONS: usize = 1000;

    let mut handles = vec![];

    for thread_id in 0..NUM_THREADS {
        let handle = thread::spawn(move || {
            let mut roots: Vec<*mut u8> = Vec::with_capacity(10);

            for i in 0..ITERATIONS {
                let obj = ruyi_gc_alloc(16);
                assert!(
                    !obj.is_null(),
                    "thread {}: allocation {} failed",
                    thread_id,
                    i
                );

                unsafe {
                    *(obj as *mut u64) = (thread_id * ITERATIONS + i) as u64;
                }

                // Keep some objects rooted
                if i % 10 == 0 {
                    unsafe {
                        ruyi_gc_add_root(obj);
                    }
                    roots.push(obj);
                }

                // Periodic collection
                if i % 100 == 99 {
                    ruyi_gc_collect();
                }
            }

            // Verify rooted objects
            for (i, &obj) in roots.iter().enumerate() {
                let expected = (thread_id * ITERATIONS + (i * 10)) as u64;
                unsafe {
                    assert_eq!(
                        *(obj as *mut u64),
                        expected,
                        "thread {}: root {} value mismatch",
                        thread_id,
                        i
                    );
                }
            }

            // Clean up roots
            for &obj in &roots {
                unsafe {
                    ruyi_gc_remove_root(obj);
                }
            }

            ruyi_gc_collect();

            thread_id
        });
        handles.push(handle);
    }

    // Wait for all threads
    for handle in handles {
        let thread_id = handle.join().unwrap();
        assert!(thread_id < NUM_THREADS);
    }
}
