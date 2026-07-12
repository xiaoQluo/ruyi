/**
 * Tests for the stub allocator (`cc_alloc`).
 *
 * `cc_alloc` is the placeholder allocator used by `--gc=stub` mode. The
 * compiler emits `call i8* @cc_alloc(i64 %size)` and the static linker
 * resolves this symbol from `libruyi_runtime.a`. The implementation must
 * satisfy three contracts:
 *
 * 1. **Zero-size semantics**: `cc_alloc(0)` returns null (matches malloc).
 * 2. **Writability**: returned pointers must point to writable memory of
 *    at least `size` bytes.
 * 3. **8-byte alignment**: Ruyi object layouts assume 8-byte aligned
 *    heap allocations; `cc_alloc` must preserve that invariant.
 *
 * @author luozegang
 * @date 2026-07-10
 */
use ruyi_runtime::cc_alloc;

#[test]
fn cc_alloc_zero_returns_null() {
    assert!(cc_alloc(0).is_null());
}

#[test]
fn cc_alloc_non_zero_returns_writable() {
    unsafe {
        let ptr = cc_alloc(64);
        assert!(!ptr.is_null());
        // 写 8 字节
        (ptr as *mut u64).write(0xDEADBEEFCAFEBABE);
        // 读回
        let value = (ptr as *const u64).read();
        assert_eq!(value, 0xDEADBEEFCAFEBABE);
    }
}

#[test]
fn cc_alloc_returns_8_byte_aligned() {
    unsafe {
        for size in [1, 7, 8, 15, 16, 100, 1024] {
            let ptr = cc_alloc(size);
            assert!(!ptr.is_null(), "cc_alloc({}) returned null", size);
            assert_eq!(ptr as usize % 8, 0, "cc_alloc({}) not 8-byte aligned", size);
        }
    }
}
