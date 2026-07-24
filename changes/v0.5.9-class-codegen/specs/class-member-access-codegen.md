# Spec: Class Instance Member Access Codegen

## ADDED Requirements

### REQ-CG-MA-001: Field Read on Arbitrary Expression

`compile_simple_member_access` SHALL accept any AST expression as the object (not just `Expr::Identifier`). It SHALL compile the object expression first, then use the resulting type information and LLVM value to perform GEP-based field offset access.

**#### Scenario: Field read on function call result**
- WHEN source is `get_point().x` where `get_point()` returns a class instance
- THEN codegen SHALL compile `get_point()` into an LLVM value, look up `class_fields["Point"]["x"]`, compute the field offset via GEP, and load the field value

**#### Scenario: Field read on method chain result**
- WHEN source is `Point.new(3, 4).x` where `Point.new()` returns a class instance
- THEN codegen SHALL produce the LLVM IR that loads field `x` from the value returned by `Point_new`

**#### Scenario: Field read on variable remains working**
- WHEN source is `let p = Point.new(3, 4); print(p.x);`
- THEN codegen SHALL produce the same LLVM IR as before (no regression for Identifier objects)

---

### REQ-CG-MA-002: Field Write on Arbitrary Expression

Field assignment (`Expr::Assignment` with `Expr::Member` on LHS) SHALL support arbitrary expression objects. The codegen SHALL compile the object expression, then GEP to the field offset and store the RHS value.

**#### Scenario: Field write on method chain**
- WHEN source is `Wide.new().a = 1`
- THEN codegen SHALL compile `Wide.new()`, GEP to field `a` offset, and store `1`

**#### Scenario: Field write on variable remains working**
- WHEN source is `let w = Wide.new(); w.a = 1;`
- THEN field write codegen SHALL produce correct LLVM store instruction

---

### REQ-CG-MA-003: Method Call on Class Instance

`compile_call` SHALL handle `Expr::Call { callee: Expr::Member { object, property } }` where object compiles to a class instance. It SHALL route to `compile_new` when property is `"new"` and object resolves to a class name, and SHALL route to instance method invocation otherwise.

**#### Scenario: Constructor call via ClassName.new()**
- WHEN source is `Point.new(3, 4)` where `Point` is a class name
- THEN codegen SHALL allocate a struct sized to `class_struct_types["Point"]` and call `Point_new`

**#### Scenario: Instance method call**
- WHEN source is `obj.format()` where `obj` is a class instance with method `format`
- THEN codegen SHALL look up `class_methods` for the method, pass `self` as first arg, and generate the LLVM call

**#### Scenario: Method chain**
- WHEN source is `Point.new(3, 4).format()`
- THEN codegen SHALL first compile `Point.new(3, 4)` to produce a class instance value, then compile `.format()` on that value

---

### REQ-CG-MA-004: Safe Access (?.) on Nullable Class Instance

`compile_optional_member_access` SHALL handle `?.field` on nullable class instances. When the nullable value is non-null, it SHALL dereference and access the field. When null, it SHALL return 0 (default value) without crashing.

**#### Scenario: Safe access on non-null value**
- WHEN source is `let maybe: Point? = Point.new(3, 4); print(maybe?.x);`
- THEN codegen SHALL produce a null check branch, access field `x` on the non-null path, and output `3`

**#### Scenario: Safe access on null value**
- WHEN source is `let null_p: Point? = null; print(null_p?.x);`
- THEN codegen SHALL produce a null check branch, return 0 on the null path, and output `0`

---

### REQ-CG-MA-005: Bracket Access by String Key on Class Instance

When `MemberProperty::Expr(key)` is used with a class instance object, codegen SHALL look up `class_fields` by the string key name and perform GEP-based field access, rather than falling back to the generic `ruyi_obj_get` runtime call.

**#### Scenario: Bracket access with string literal**
- WHEN source is `let p = Point.new(3, 4); print(p["x"]);`
- THEN codegen SHALL resolve `"x"` to field index 0 in `class_fields["Point"]`, GEP to offset, load, and output `3`

**#### Scenario: Array bracket access remains unchanged**
- WHEN source is `let arr = [10, 20, 30]; print(arr[0]);`
- THEN codegen SHALL continue to route to `__builtin_array_get` (no regression)

---

### REQ-CG-MA-006: Multi-Field Class Allocation Size

`compile_new` SHALL allocate the correct LLVM struct size for classes with multiple fields. An 8-field `Wide` class SHALL allocate 8 × 8 = 64 bytes (8 i64 fields).

**#### Scenario: 8-field class allocation**
- WHEN source defines `class Wide { a: int; b: int; ... h: int; fn new() {} }` and calls `Wide.new()`
- THEN the LLVM struct type SHALL have 8 i64 fields, and `alloca` SHALL allocate the correct size

---

### REQ-CG-MA-007: No Typecheck Regression

The typechecker test suite SHALL continue to pass with zero failures.

**#### Scenario: Full typechecker suite**
- WHEN running `cargo test -p ruyic --test typechecker`
- THEN all 222 tests SHALL pass, 0 ignored

---

### REQ-CG-MA-008: Ignored Tests Enabled

The three codegen integration tests (`codegen_class_creation`, `test_new_class_8_fields`, `codegen_fixture_member_access`) SHALL be un-ignored and SHALL pass.

**#### Scenario: codegen_class_creation**
- WHEN source is `class Point { x:int; y:int; fn new(x,y){...} fn format(self):string{...} } print(Point.new(3,4).format());`
- THEN `assert_output(source, "(3, 4)")` SHALL succeed

**#### Scenario: test_new_class_8_fields**
- WHEN source defines an 8-field `Wide` class, creates an instance, assigns and reads all 8 fields
- THEN `assert_output(source, "1\n2\n3\n4\n5\n6\n7\n8")` SHALL succeed

**#### Scenario: codegen_fixture_member_access**
- WHEN running the `member_access.ry` fixture (`.field`, `?.field`, `["key"]` on class instances)
- THEN output SHALL match `member_access.expected` exactly (`3\n4\n3\n3\n0`)
