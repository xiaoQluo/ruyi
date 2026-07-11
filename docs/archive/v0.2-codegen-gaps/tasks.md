# Tasks: v0.2-codegen-gaps

> Part of change `v0.2-codegen-gaps`. Atomic, dependency-ordered implementation
> tasks. Each step is 2-5 min, no TBD/TODO/placeholders. All 19 requirements
> are mapped to specific tasks.

## File Structure

### Modify

| File | One-sentence responsibility |
|------|------------------------------|
| `crates/ruyi_runtime/src/builtins.rs` | Add `ruyi_obj_get` and `ruyi_obj_keys` FFI symbols |
| `crates/ruyi_runtime/Cargo.toml` | No change required (`std::collections::HashMap` is in std) |
| `crates/ruyic/src/codegen/expr.rs` | Fix `compile_new` size, detect int-literal array index |
| `crates/ruyic/src/codegen/stmt.rs` | Wire `Statement::Labeled` to push label to `loop_stack` |
| `crates/ruyic/src/codegen/generator.rs` | Extend `loop_stack` tuple to include `Option<String>` |
| `crates/ruyic/src/typechecker/inference.rs` | Fix `Expr::SelfExpr` synthesis; extend `synthesize_member_access` |
| `crates/ruyic/tests/codegen.rs` | Remove `#[ignore]` from now-passing tests (22 total) |
| `examples/run_examples.sh` | Add 5 new examples to the run list |
| `docs/roadmap.md` | Update v0.2 task status from ❌ to ✅ |
| `docs/roadmap-zh.md` | Mirror English version status update |

### Create

| File | One-sentence responsibility |
|------|------------------------------|
| `examples/class_basics.ry` | Class with fields, `self`, methods, `new` |
| `examples/object_literal.ry` | Object literal `{k:v}` and bracket access |
| `examples/array_operations.ry` | Array literal, index, push, pop, length |
| `examples/member_access.ry` | `obj.field`, `obj?.field`, `obj["key"]` |
| `examples/labeled_loops.ry` | `break <label>` and `continue <label>` |
| `crates/ruyic/tests/integration/cases/codegen/class_layout.ry` | Integration test: class with 3+ fields |
| `crates/ruyic/tests/integration/cases/codegen/class_layout.expected` | Expected output of class_layout.ry |
| `crates/ruyic/tests/integration/cases/codegen/method_call.ry` | Integration test: `obj.method(args)` with self |
| `crates/ruyic/tests/integration/cases/codegen/method_call.expected` | Expected output of method_call.ry |
| `crates/ruyic/tests/integration/cases/codegen/labeled_loops.ry` | Integration test: labeled break/continue |
| `crates/ruyic/tests/integration/cases/codegen/labeled_loops.expected` | Expected output of labeled_loops.ry |
| `crates/ruyic/tests/integration/cases/codegen/for_in_obj.ry` | Integration test: `for...in` over object |
| `crates/ruyic/tests/integration/cases/codegen/for_in_obj.expected` | Expected output of for_in_obj.ry |
| `crates/ruyi_runtime/tests/obj_get.rs` | Unit test: `ruyi_obj_get` FFI behavior |
| `crates/ruyi_runtime/tests/obj_keys.rs` | Unit test: `ruyi_obj_keys` FFI behavior |

## Interfaces (Cross-Batch)

### Batch 1 Provides → Batch 2 Consumes

| Provides | Type / Signature | Consumer |
|----------|-------------------|----------|
| `ruyi_obj_get` symbol | `#[no_mangle] pub extern "C" fn ruyi_obj_get(obj: *mut i8, key: *const i8) -> *mut i8` | T3 array index fallback path; T8 integration tests |
| `ruyi_obj_keys` symbol | `#[no_mangle] pub extern "C" fn ruyi_obj_keys(obj: *mut i8) -> *mut i8` | T8 for_in_obj integration test |
| `self` type | `Type::Named(<enclosing_class_name>)` from `synthesize` of `Expr::SelfExpr` | T6 class field/method resolution |
| `compile_new` size | LLVM `BasicValue` pointer to allocation sized by `class_struct_types[name].size()` | T8 class_layout integration test |

### Batch 2 Provides → Batch 3 Consumes

| Provides | Type / Signature | Consumer |
|----------|-------------------|----------|
| Direct array GEP | `MemberProperty::Expr(IntLiteral(i))` on `Type::Array` → `__builtin_array_get(arr, i)` | T7 un-ignore array_literal test |
| Labeled break/continue | `break <label>` / `continue <label>` jump to matching loop | T7 un-ignore; T8 labeled_loops integration test |
| Class field resolution | `synthesize_member_access` for `Type::Named` returns field/method type | T8 class_layout, method_call integration tests |

### Batch 3 Provides → Done

All work products from Batches 1-2 are exercised by T7 (un-ignore) and T8 (new tests/examples).

## Dependency Graph

```
T1 (FFI) ──────────┐
T2 (compile_new) ──┼─→ T3 (array GEP) ───┐
T5 (self type) ────┤                      ├─→ T7 (un-ignore) ──→ DONE
                    └─→ T6 (field res) ───┤
T5 (self type) ────────→ T4 (labeled) ────┘
                                            └─→ T8 (new tests) ──→ DONE
```

## Per-Task Detail

---

### Task T1: Add `ruyi_obj_get` and `ruyi_obj_keys` runtime FFI

**Depends on**: none
**Requirements**: REQ-FFI-001, REQ-FFI-002, REQ-FFI-003
**Files**:
- Modify: `crates/ruyi_runtime/src/builtins.rs`
- Create: `crates/ruyi_runtime/tests/obj_get.rs`
- Create: `crates/ruyi_runtime/tests/obj_keys.rs`

#### Phase 1: Write failing unit test for `ruyi_obj_get`

File: `crates/ruyi_runtime/tests/obj_get.rs`

- [x] **Step 1.1** (3 min): Create file with `use ruyi_runtime::*;` and `extern "C" { fn ruyi_obj_get(obj: *mut i8, key: *const i8) -> *mut i8; }`
- [x] **Step 1.2** (2 min): Add `#[test] fn test_get_existing_key()` that allocates `{x: 42}` via the object literal FFI (or direct memory layout), then calls `ruyi_obj_get(obj, b"x\0".as_ptr() as *const i8)` and asserts return != null
- [x] **Step 1.3** (2 min): Add `#[test] fn test_get_missing_key()` that calls `ruyi_obj_get` for a non-existent key and asserts return == null
- [x] **Step 1.4** (2 min): Add `#[test] fn test_get_null_object()` and `test_get_null_key()` that pass null pointers and assert return == null
- [x] **Step 1.5** (3 min): Run `cargo test -p ruyi_runtime --test obj_get` and confirm 4 tests fail with "undefined symbol: ruyi_obj_get"

#### Phase 2: Implement `ruyi_obj_get` (GREEN)

File: `crates/ruyi_runtime/src/builtins.rs`

- [x] **Step 2.1** (3 min): Add the function signature at the end of the file:
  ```rust
  #[no_mangle]
  pub extern "C" fn ruyi_obj_get(obj: *mut i8, key: *const i8) -> *mut i8 {
      // Stub: always return null
      std::ptr::null_mut()
  }
  ```
- [x] **Step 2.2** (2 min): Add `use std::collections::HashMap;` to imports
- [x] **Step 2.3** (5 min): Define an internal helper `object_field_map(obj: *mut i8) -> Option<HashMap<String, *mut i8>>` that reads the first 8 bytes as field count and the next `count * 8` bytes as (key_ptr, value_ptr) pairs, then parses each key as a UTF-8 C string and inserts into the map
- [x] **Step 2.4** (5 min): Implement the real `ruyi_obj_get`:
  ```rust
  #[no_mangle]
  pub extern "C" fn ruyi_obj_get(obj: *mut i8, key: *const i8) -> *mut i8 {
      if obj.is_null() || key.is_null() { return std::ptr::null_mut(); }
      let key_str = unsafe { std::ffi::CStr::from_ptr(key) }.to_string_lossy().into_owned();
      match object_field_map(obj) {
          Some(map) => map.get(&key_str).copied().unwrap_or(std::ptr::null_mut()),
          None => std::ptr::null_mut(),
      }
  }
  ```
- [x] **Step 2.5** (3 min): Run `cargo test -p ruyi_runtime --test obj_get` and confirm all 4 tests pass

#### Phase 3: Write failing unit test for `ruyi_obj_keys`

File: `crates/ruyi_runtime/tests/obj_keys.rs`

- [x] **Step 3.1** (2 min): Create file with `extern "C" { fn ruyi_obj_keys(obj: *mut i8) -> *mut i8; }` and `extern "C" { fn ruyi_array_length(arr: *mut i8) -> i64; }`
- [x] **Step 3.2** (3 min): Add `#[test] fn test_keys_of_2_field_object()` that creates `{x: 1, y: 2}`, calls `ruyi_obj_keys`, then asserts `ruyi_array_length` returns 2
- [x] **Step 3.3** (2 min): Add `#[test] fn test_keys_of_empty_object()` that creates `{}`, calls `ruyi_obj_keys`, asserts length 0
- [x] **Step 3.4** (2 min): Add `#[test] fn test_keys_of_null()` that calls `ruyi_obj_keys(null)`, asserts length 0 and no crash
- [x] **Step 3.5** (2 min): Run `cargo test -p ruyi_runtime --test obj_keys` and confirm 3 tests fail with "undefined symbol: ruyi_obj_keys"

#### Phase 4: Implement `ruyi_obj_keys` (GREEN)

File: `crates/ruyi_runtime/src/builtins.rs`

- [x] **Step 4.1** (2 min): Add the function signature at the end of the file:
  ```rust
  #[no_mangle]
  pub extern "C" fn ruyi_obj_keys(obj: *mut i8) -> *mut i8 {
      // Stub: return empty array
      ruyi_array_alloc(0)
  }
  ```
- [x] **Step 4.2** (5 min): Implement the real `ruyi_obj_keys`:
  ```rust
  #[no_mangle]
  pub extern "C" fn ruyi_obj_keys(obj: *mut i8) -> *mut i8 {
      if obj.is_null() { return ruyi_array_alloc(0); }
      let map = match object_field_map(obj) {
          Some(m) => m,
          None => return ruyi_array_alloc(0),
      };
      let arr = ruyi_array_alloc(map.len() as i64);
      for (i, key) in map.keys().enumerate() {
          let cstr = std::ffi::CString::new(key.as_str()).unwrap();
          ruyi_array_set(arr, i as i64, cstr.into_raw() as i64);
      }
      arr
  }
  ```
- [x] **Step 4.3** (2 min): Run `cargo test -p ruyi_runtime --test obj_keys` and confirm all 3 tests pass

#### Phase 5: Symbol visibility verification (REQ-FFI-003)

- [x] **Step 5.1** (2 min): Run `cargo build -p ruyi_runtime --release`
- [x] **Step 5.2** (2 min): Run `nm target/release/libruyi_runtime.a | grep ruyi_obj` and confirm both `ruyi_obj_get` and `ruyi_obj_keys` are listed with uppercase 'T' (defined external text symbol)
- [x] **Step 5.3** (3 min): Run `cargo test -p ruyi_runtime` and confirm all tests still pass (no regressions in existing exception/GC/async tests)

---

### Task T2: Fix `compile_new` hardcoded 64-byte allocation

**Depends on**: none (can run in parallel with T1, T5)
**Requirements**: REQ-CAP1-001
**Files**:
- Modify: `crates/ruyic/src/codegen/expr.rs:2940` (the `compile_new` function)
- Modify: `crates/ruyic/tests/codegen.rs` (regression test)

#### Phase 1: Write failing integration test

File: `crates/ruyic/tests/codegen.rs` (new test, add to end of file)

- [x] **Step 1.1** (3 min): Add `#[test] fn test_new_class_8_fields()` that:
  - Defines a class with 8 i64 fields via Ruyi source string
  - Constructs an instance
  - Writes distinct values to all 8 fields
  - Reads them back
  - Asserts all reads return the values written
- [x] **Step 1.2** (2 min): Mark the test with `#[ignore]` for now (will un-ignore in T7)
- [x] **Step 1.3** (2 min): Run `cargo test -p ruyic --test codegen test_new_class_8_fields -- --ignored` and confirm it fails (compile error or runtime crash)

#### Phase 2: Investigate `compile_new` and `class_struct_types`

- [x] **Step 2.1** (2 min): Read `crates/ruyic/src/codegen/expr.rs:2940-2972` to confirm current hardcoded `const_int(64, false)`
- [x] **Step 2.2** (2 min): Read `crates/ruyic/src/codegen/generator.rs` to locate `class_struct_types: HashMap<String, StructType>` field
- [x] **Step 2.3** (2 min): Read `crates/ruyic/src/codegen/decl.rs:313` (`compile_class`) to confirm `class_struct_types[name] = struct_type` is set
- [x] **Step 2.4** (2 min): Read existing code that uses `class_struct_types` (search for `class_struct_types` in codegen) to learn the access pattern

#### Phase 3: Fix `compile_new` (GREEN)

File: `crates/ruyic/src/codegen/expr.rs:2940`

- [x] **Step 3.1** (3 min): Replace `let total_size = ctx.context.i64_type().const_int(64, false);` with:
  ```rust
  let struct_ty = ctx.class_struct_types.get(class_name)
      .ok_or_else(|| format!("unknown class: {}", class_name))?;
  let size_bytes = struct_ty.size_of().ok_or_else(|| "class has no size")?;
  let total_size = size_bytes;
  ```
- [x] **Step 3.2** (2 min): Ensure the rest of `compile_new` uses `total_size` (no other changes needed if existing code already used `total_size`)
- [x] **Step 3.3** (2 min): Build with `cargo build -p ruyic` and confirm 0 errors
- [x] **Step 3.4** (3 min): Run `cargo test -p ruyic --test codegen test_new_class_8_fields -- --ignored` and confirm it passes

#### Phase 4: Regression check

- [x] **Step 4.1** (3 min): Run `cargo test -p ruyic --test codegen -- --ignored` to see which existing tests now pass that didn't before
- [x] **Step 4.2** (2 min): For tests that now pass, note them for T7 (will remove `#[ignore]`)
- [x] **Step 4.3** (2 min): For tests that still fail, document why (likely waiting for T1 FFI or T5 self type)

---

### Task T5: Fix `self` type in typechecker

**Depends on**: none
**Requirements**: REQ-CAP1-002
**Files**:
- Modify: `crates/ruyic/src/typechecker/inference.rs:738` (`Expr::SelfExpr` synthesis)
- Modify: `crates/ruyic/tests/typechecker.rs`

#### Phase 1: Write failing typechecker test

File: `crates/ruyic/tests/typechecker.rs` (add to end)

- [x] **Step 1.1** (3 min): Add `#[test] fn test_self_in_method_has_class_type()` that:
  - Defines a class `Point { x: int, y: int }` with a method `fn sum(self): int { return self.x + self.y; }`
  - Asserts that `self` inside the method has type `Point` (not `dyn`)
  - Asserts that `self.x` has type `int`
- [x] **Step 1.2** (2 min): Add `#[test] fn test_self_outside_class_is_error()` that:
  - Has `let x = self;` at module level
  - Asserts a compile error E4002 is reported
- [x] **Step 1.3** (2 min): Add `#[test] fn test_self_in_nested_closure_is_dynamic()` that:
  - Has a method body with a nested fn that references `self`
  - Asserts the closure's `self` is `Type::Dynamic` (per REQ-CAP1-002 Scenario 3)
- [x] **Step 1.4** (2 min): Run `cargo test -p ruyic --test typechecker test_self` and confirm 3 tests fail

#### Phase 2: Implement fix (GREEN)

File: `crates/ruyic/src/typechecker/inference.rs`

- [x] **Step 2.1** (2 min): Read `inference.rs:51` to confirm `class_stack: Vec<String>` exists
- [x] **Step 2.2** (2 min): Read `inference.rs:348` (`infer_class_element`) to confirm where the class name is pushed onto `class_stack`
- [x] **Step 2.3** (2 min): Read `inference.rs:738` to confirm current `Expr::SelfExpr => Type::Dynamic` behavior
- [x] **Step 2.4** (3 min): Replace line 738 with:
  ```rust
  Expr::SelfExpr => {
      ctx.class_stack.last()
          .map(|name| Type::Named(name.clone()))
          .unwrap_or(Type::Dynamic)
  }
  ```
  (If the stack is empty, fallback to `Type::Dynamic` so the existing "self outside class" scenario becomes a non-error for backward compat — error reporting is added in step 2.6)
- [x] **Step 2.5** (3 min): Run `cargo test -p ruyic --test typechecker test_self_in_method_has_class_type` and confirm it passes
- [x] **Step 2.6** (5 min): Add explicit error for `self` outside class: in the synthesis of `Expr::SelfExpr`, if `class_stack` is empty AND the function isn't a method, emit a diagnostic `E4002 "self used outside of class method"`. Use the existing `DiagnosticEmitter` pattern from the same file.

#### Phase 3: Verify closure case

- [x] **Step 3.1** (3 min): Read `infer_class_element` to understand how nested function scopes interact with `class_stack`
- [x] **Step 3.2** (2 min): If `class_stack` is popped on entering a nested fn body, no change needed (closure sees empty stack → Dynamic, matching REQ-CAP1-002 Scenario 3)
- [x] **Step 3.3** (3 min): Run `cargo test -p ruyic --test typechecker test_self_in_nested_closure_is_dynamic` and confirm it passes
- [x] **Step 3.4** (2 min): Run all typechecker tests to confirm no regression: `cargo test -p ruyic --test typechecker`

---

### Task T3: Array index uses direct GEP for `IntLiteral` keys

**Depends on**: T1 (FFI needed for fallback path), T2 (compile_new — no direct dep but T8 integration tests need both)
**Requirements**: REQ-CAP3-002
**Files**:
- Modify: `crates/ruyic/src/codegen/expr.rs:635` (`compile_member_access`)
- Modify: `crates/ruyic/tests/codegen.rs`

#### Phase 1: Write failing integration test

File: `crates/ruyic/tests/codegen.rs`

- [x] **Step 1.1** (3 min): Add `#[test] fn test_array_index_int_literal_uses_gep()` that:
  - Creates an array `[10, 20, 30]`
  - Accesses `arr[0]`, `arr[1]`, `arr[2]` and prints each
  - Expects output `10\n20\n30`
- [x] **Step 1.2** (2 min): Add `#[test] fn test_array_index_variable_uses_runtime_call()` that:
  - Creates an array `[10, 20, 30]`
  - Iterates with `for (let i = 0; i < 3; i = i + 1) { print(arr[i]); }`
  - Expects output `10\n20\n30`
- [x] **Step 1.3** (2 min): Add `#[test] fn test_array_index_out_of_bounds_no_crash()` that:
  - Creates an array `[1]`
  - Accesses `arr[100]`
  - Expects either 0, null, or some sentinel — but **not** a segfault
- [x] **Step 1.4** (2 min): Mark with `#[ignore]` for now
- [x] **Step 1.5** (2 min): Run `cargo test -p ruyic --test codegen test_array_index -- --ignored` and confirm tests fail (link error or wrong output)

#### Phase 2: Implement GEP path (GREEN)

File: `crates/ruyic/src/codegen/expr.rs:635` (`compile_member_access`)

- [x] **Step 2.1** (3 min): Read current `compile_member_access` to understand the `MemberProperty::Expr(key_expr)` branch
- [x] **Step 2.2** (2 min): Add a match arm before the generic `ruyi_obj_get` call:
  ```rust
  MemberProperty::Expr(Expr::IntLiteral(i)) => {
      // Direct array GEP: [0, 0] then [0, i] on array struct
      let zero = ctx.context.i32_type().const_int(0, false);
      let idx = ctx.context.i32_type().const_int(*i as u64, false);
      let elem_ptr = unsafe {
          obj_ptr.const_in_bounds_gep(
              array_struct_ty,
              &[zero, idx],
          )
      };
      ctx.builder.build_load(elem_ty, elem_ptr, "arr_elem")
  }
  ```
- [x] **Step 2.3** (2 min): Verify the `array_struct_ty` is available in context (add to `CodegenContext` if not present)
- [x] **Step 2.4** (3 min): Build with `cargo build -p ruyic` and confirm 0 errors
- [x] **Step 2.5** (3 min): Run `cargo test -p ruyic --test codegen test_array_index_int_literal_uses_gep -- --ignored` and confirm it passes

#### Phase 3: Verify variable and OOB cases

- [x] **Step 3.1** (3 min): Run `cargo test -p ruyic --test codegen test_array_index_variable_uses_runtime_call -- --ignored` — should pass (existing runtime call path)
- [x] **Step 3.2** (3 min): Run `cargo test -p ruyic --test codegen test_array_index_out_of_bounds_no_crash -- --ignored` — should pass (existing bounds check in runtime)
- [x] **Step 3.3** (2 min): Inspect the generated LLVM IR for `arr[0]` using `cargo run -- -emit-llvm <test>` and confirm a `getelementptr` is present (not a runtime call)

---

### Task T6: Class field/method resolution in typechecker

**Depends on**: T5 (self type)
**Requirements**: REQ-CAP1-002 (extension), REQ-CAP7-001
**Files**:
- Modify: `crates/ruyic/src/typechecker/inference.rs:1372` (`synthesize_member_access`)
- Modify: `crates/ruyic/tests/typechecker.rs`

#### Phase 1: Write failing typechecker test

File: `crates/ruyic/tests/typechecker.rs`

- [x] **Step 1.1** (3 min): Add `#[test] fn test_class_field_via_member_access()` that:
  - Defines `class Point { x: int }`
  - In a function, accesses `instance.x` (where instance is `Point`)
  - Asserts the type is `int` (not `dyn`)
- [x] **Step 1.2** (3 min): Add `#[test] fn test_class_own_method_via_member_access()` that:
  - Defines `class Point { x: int; fn getX(self): int { return self.x; } }`
  - In a function, accesses `instance.getX`
  - Asserts the type is `function (Point) -> int`
- [x] **Step 1.3** (2 min): Run tests and confirm they fail (current behavior: `Type::Dynamic`)

#### Phase 2: Implement field/method lookup (GREEN)

File: `crates/ruyic/src/typechecker/inference.rs:1372`

- [x] **Step 2.1** (2 min): Read current `synthesize_member_access` to understand the `Type::Named` arm
- [x] **Step 2.2** (2 min): Find where class declarations store their fields/methods (likely in a class environment in `InferenceContext` or a global table)
- [x] **Step 2.3** (5 min): Add a field-lookup step before the trait resolution:
  ```rust
  Type::Named(name) => {
      // Try class fields first
      if let Some(fields) = ctx.class_fields.get(name) {
          if let Some(field_ty) = fields.get(&property_name) {
              return field_ty.clone();
          }
      }
      // Then class methods
      if let Some(methods) = ctx.class_methods.get(name) {
          if let Some(method_ty) = methods.get(&property_name) {
              return method_ty.clone();
          }
      }
      // Fallback: trait impl
      resolve_impl_method(...)
  }
  ```
- [x] **Step 2.4** (3 min): Build with `cargo build -p ruyic` and confirm 0 errors
- [x] **Step 2.5** (3 min): Run `cargo test -p ruyic --test typechecker test_class_field_via_member_access test_class_own_method_via_member_access` and confirm both pass

#### Phase 3: Regression check

- [x] **Step 3.1** (3 min): Run `cargo test -p ruyic --test typechecker` to confirm no existing test regressed
- [x] **Step 3.2** (3 min): Run `cargo test -p ruyic --test codegen -- --ignored` and check that trait-related tests still pass (we didn't break trait resolution)

---

### Task T4: Labeled break/continue

**Depends on**: T5 (modifies typechecker which affects codegen)
**Requirements**: REQ-CAP8-001, REQ-CAP8-002
**Files**:
- Modify: `crates/ruyic/src/codegen/generator.rs:80-83` (`loop_stack` type)
- Modify: `crates/ruyic/src/codegen/stmt.rs:73-90` (`Break`, `Continue`, `Labeled` arms)
- Modify: `crates/ruyic/tests/codegen.rs`

#### Phase 1: Write failing integration test

File: `crates/ruyic/tests/codegen.rs`

- [x] **Step 1.1** (3 min): Add `#[test] fn test_labeled_break_exits_outer_loop()` that:
  - Has `outer: for (let i = 0; i < 3; i = i + 1) { for (let j = 0; j < 3; j = j + 1) { break outer; } print("never"); } print("done");`
  - Expects output `done\n` (and no `never` lines)
- [x] **Step 1.2** (3 min): Add `#[test] fn test_labeled_continue_resumes_outer()` that:
  - Has `outer: for (let i = 0; i < 3; i = i + 1) { for (let j = 0; j < 3; j = j + 1) { continue outer; } print("never"); }`
  - Expects output (i updates 3 times, no `never` prints)
- [x] **Step 1.3** (3 min): Add `#[test] fn test_break_undefined_label_is_error()` that:
  - Has `break nonexistent;` outside any labeled loop
  - Expects compile error E3003
- [x] **Step 1.4** (2 min): Mark with `#[ignore]` and run to confirm failure

#### Phase 2: Extend `loop_stack` type

File: `crates/ruyic/src/codegen/generator.rs`

- [x] **Step 2.1** (2 min): Find `loop_stack: Vec<(BasicBlock, BasicBlock)>` at lines ~80-83
- [x] **Step 2.2** (2 min): Change to `loop_stack: Vec<(BasicBlock, BasicBlock, Option<String>)>` (3-tuple with label)
- [x] **Step 2.3** (2 min): Update `push_loop` and `pop_loop` helper functions to take/return the new tuple
- [x] **Step 2.4** (3 min): Update all `push_loop((bb1, bb2))` call sites to `push_loop((bb1, bb2, None))` — search for `push_loop` in codegen
- [x] **Step 2.5** (3 min): Build with `cargo build -p ruyic` and confirm 0 errors

#### Phase 3: Wire `Statement::Labeled`

File: `crates/ruyic/src/codegen/stmt.rs`

- [x] **Step 3.1** (2 min): Read `Statement::Labeled { body, .. } => compile_stmt(ctx, body)` at line 90
- [x] **Step 3.2** (5 min): Replace with:
  ```rust
  Statement::Labeled { name, body } => {
      // Push the label onto the loop_stack if body is a loop statement
      match body.as_ref() {
          Statement::For { .. } | Statement::ForIn { .. } | Statement::ForOf { .. } | Statement::While { .. } => {
              // Push current label to current loop entry; this requires capturing the loop's end_bb and cond_bb
              // Implementation: compile_stmt but with a label-setter that wraps the next push_loop call
              compile_labeled_loop(ctx, name.clone(), body)
          }
          _ => compile_stmt(ctx, body), // Non-loop label: no special handling needed
      }
  }
  ```
- [x] **Step 3.3** (5 min): Implement `compile_labeled_loop` helper that:
  - Pushes `Some(name.clone())` as the label into the next loop's `loop_stack` entry
  - Recursively compiles the body
  - Pops the entry on exit
- [x] **Step 3.4** (2 min): Update `Break(Some(label))` and `Continue(Some(label))` arms to walk the `loop_stack` from top to bottom and find the entry where the label matches

#### Phase 4: Error reporting

File: `crates/ruyic/src/codegen/stmt.rs`

- [x] **Step 4.1** (3 min): If no matching label is found in `loop_stack`, emit diagnostic E3003 "undefined label" via the existing `DiagnosticEmitter`
- [x] **Step 4.2** (2 min): Build with `cargo build -p ruyic` and confirm 0 errors
- [x] **Step 4.3** (3 min): Run `cargo test -p ruyic --test codegen test_labeled -- --ignored` and confirm tests pass

#### Phase 5: Verify existing behavior

- [x] **Step 5.1** (3 min): Run `cargo test -p ruyic --test codegen -- --ignored` and confirm unlabeled break/continue tests still pass
- [x] **Step 5.2** (3 min): Run examples: `bash examples/run_examples.sh` and confirm 34/34 still pass (no regression from changing `loop_stack` shape)

---

### Task T7: Remove `#[ignore]` from now-passing codegen tests

**Depends on**: T1, T2, T3, T4, T5, T6 (all previous tasks)
**Requirements**: (verification of all CAP requirements)
**Files**:
- Modify: `crates/ruyic/tests/codegen.rs`

#### Phase 1: Run all ignored tests

- [x] **Step 1.1** (2 min): Run `cargo test -p ruyic --test codegen -- --ignored 2>&1 | tee /tmp/codegen_results.txt`
- [x] **Step 1.2** (3 min): Count passing vs failing tests in the output

#### Phase 2: Un-ignore passing tests

- [x] **Step 2.1** (5 min): For each passing test, remove the `#[ignore]` attribute on the line above it
- [x] **Step 2.2** (3 min): Build with `cargo build -p ruyic --tests` to confirm no compilation errors
- [x] **Step 2.3** (3 min): Run `cargo test -p ruyic --test codegen` and confirm previously-passing-now-un-ignored tests still pass (and they would block CI if they failed)

#### Phase 3: Document still-failing tests

- [x] **Step 3.1** (3 min): For each test that still fails, add a comment above the `#[ignore]` explaining the blocker:
  ```rust
  // TODO: blocked by T-XXX (link error: ruyi_obj_get missing)
  #[ignore]
  #[test]
  fn test_object_bracket_access() { ... }
  ```
- [x] **Step 3.2** (2 min): File a follow-up issue listing these blockers (in `.spec-superflow/changes/` or roadmap) so they aren't forgotten

---

### Task T8: Add 5 examples + 8 integration tests

**Depends on**: T1, T2, T3, T4, T5, T6
**Requirements**: (verification + documentation)
**Files**:
- Create: 5 `.ry` examples in `examples/`
- Create: 8 integration test fixtures in `crates/ruyic/tests/integration/cases/codegen/`
- Modify: `examples/run_examples.sh`
- Modify: `docs/roadmap.md`, `docs/roadmap-zh.md`

#### Phase 1: Create 5 example .ry files

- [x] **Step 1.1** (5 min): Create `examples/class_basics.ry` with:
  ```ruyi
  class Point {
      x: int;
      y: int;
      fn new(x: int, y: int) { self.x = x; self.y = y; }
      fn sum(self): int { return self.x + self.y; }
  }
  let p = new Point();
  p.new(3, 4);
  print(p.sum());
  ```
- [x] **Step 1.2** (3 min): Create `examples/object_literal.ry` with:
  ```ruyi
  let o = { x: 10, y: 20 };
  print(o["x"]);
  print(o["y"]);
  ```
- [x] **Step 1.3** (3 min): Create `examples/array_operations.ry` with:
  ```ruyi
  let arr = [1, 2, 3, 4, 5];
  print(arr[0]);
  print(arr.length);
  arr.push(6);
  print(arr.length);
  ```
- [x] **Step 1.4** (3 min): Create `examples/member_access.ry` with:
  ```ruyi
  let p = { name: "Alice", age: 30 };
  print(p.name);
  print(p["age"]);
  let q: dyn = null;
  print(q?.["missing"]);
  ```
- [x] **Step 1.5** (3 min): Create `examples/labeled_loops.ry` with:
  ```ruyi
  outer: for (let i = 0; i < 3; i = i + 1) {
      for (let j = 0; j < 3; j = j + 1) {
          if (i == 1 && j == 1) { break outer; }
          print(i * 10 + j);
      }
  }
  ```

#### Phase 2: Create 8 integration test fixtures

For each, create `.ry` and `.expected` files. Use small focused programs.

- [x] **Step 2.1** (4 min): `class_layout.ry` + `.expected` — class with 3 fields, read/write each, print
- [x] **Step 2.2** (4 min): `object_literal.ry` + `.expected` — `{k:v}` + bracket access + spread
- [x] **Step 2.3** (4 min): `array_literal.ry` + `.expected` — `[1,2,3]` + arr[0]/arr[1]/arr[2]
- [x] **Step 2.4** (4 min): `string_concat.ry` + `.expected` — `+` with strings, ints, mixed
- [x] **Step 2.5** (4 min): `for_loop.ry` + `.expected` — C-style, for-of array, for-in object
- [x] **Step 2.6** (4 min): `member_access.ry` + `.expected` — `.field`, `?.field`, `["key"]`
- [x] **Step 2.7** (4 min): `method_call.ry` + `.expected` — `obj.method(args)` with self
- [x] **Step 2.8** (4 min): `labeled_loops.ry` + `.expected` — labeled break + continue
- [x] **Step 2.9** (3 min): `for_in_obj.ry` + `.expected` — `for...in obj` iteration
- [x] **Step 2.10** (2 min): Add `for_in_obj.ry` test case to `crates/ruyic/tests/codegen.rs` with a `#[test]` function that compares the .ry output to the .expected

#### Phase 3: Update `run_examples.sh`

File: `examples/run_examples.sh`

- [x] **Step 3.1** (3 min): Add 5 new examples to the example list in the script
- [x] **Step 3.2** (3 min): Run `bash examples/run_examples.sh` and confirm 39/39 pass (34 existing + 5 new)

#### Phase 4: Update roadmap

File: `docs/roadmap.md` and `docs/roadmap-zh.md`

- [x] **Step 4.1** (3 min): Update v0.2 task status table: 1.1, 1.2, 1.3, 1.5, 1.6, 1.12, 1.13 from ❌ to ✅
- [x] **Step 4.2** (2 min): Mirror the same updates in `docs/roadmap-zh.md`
- [x] **Step 4.3** (2 min): Update the v0.2 "Completion Estimate" in the roadmap

#### Phase 5: Final verification

- [x] **Step 5.1** (3 min): `cargo build --workspace` — 0 errors
- [x] **Step 5.2** (5 min): `cargo test -p ruyi_runtime` — all pass
- [x] **Step 5.3** (5 min): `cargo test -p ruyic --lib` — all pass
- [x] **Step 5.4** (5 min): `cargo test -p ruyic --tests` — all pass
- [x] **Step 5.5** (5 min): `bash examples/run_examples.sh` — 39/39 pass
- [x] **Step 5.6** (2 min): `cargo clippy --workspace -- -D warnings` — 0 warnings

## Requirement-to-Task Mapping

| Requirement | Tasks |
|-------------|-------|
| REQ-CAP1-001 (compile_new size) | T2 |
| REQ-CAP1-002 (self type) | T5 |
| REQ-CAP1-003 (self.field GEP) | T5 (verification) |
| REQ-CAP2-001 (object literal) | T8 (integration test) |
| REQ-CAP2-002 (bracket access) | T1, T8 |
| REQ-CAP3-001 (array literal) | T8 (integration test) |
| REQ-CAP3-002 (array index GEP) | T3, T8 |
| REQ-CAP5-001 (for...in) | T1, T8 |
| REQ-CAP5-002 (for...of) | T8 (integration test) |
| REQ-CAP6-001 (obj.field) | T6, T8 |
| REQ-CAP6-002 (obj?.field) | T8 (integration test) |
| REQ-CAP7-001 (method call) | T6, T8 |
| REQ-CAP8-001 (labeled break) | T4, T8 |
| REQ-CAP8-002 (unlabeled break) | T4, T8 (regression) |
| REQ-FFI-001 (ruyi_obj_get) | T1 |
| REQ-FFI-002 (ruyi_obj_keys) | T1 |
| REQ-FFI-003 (symbol visibility) | T1 |

## Completion Criteria

The change is complete when:

- [x] All 8 tasks marked completed in `.superpowers/sdd/progress.md`
- [x] `cargo test --workspace` passes with 0 failures
- [x] `cargo clippy --workspace -- -D warnings` reports 0 warnings
- [x] `bash examples/run_examples.sh` reports 39/39 passed
- [x] 0 new `unimplemented!()` or `todo!()` introduced
- [x] `docs/roadmap.md` and `docs/roadmap-zh.md` v0.2 section reflects 7/11 P0 tasks ✅
- [x] 19 requirements from specs/ all have at least one passing test
- [x] Memory layout of objects/arrays verified in LLVM IR
