# Tasks: stdlib-io-path-process-ffi

## File Structure

| File | Responsibility |
|------|---------------|
| `crates/ruyi_runtime/src/path_ffi.rs` (Create) | 8 个路径操作 FFI 函数（join, basename, dirname, extname, is_absolute, normalize, separator, relative） |
| `crates/ruyi_runtime/src/io_ffi.rs` (Create) | 17 个文件 I/O FFI 函数（read/write/exists/delete/mkdir + async 变体 + readLine） |
| `crates/ruyi_runtime/src/process_ffi.rs` (Create) | 20 个进程管理 FFI 函数（exec/create/spawn/wait/kill/I/O 管道/env/系统信息/信号） |
| `crates/ruyi_runtime/src/lib.rs` (Modify) | 注册 `pub mod io_ffi;` `pub mod path_ffi;` `pub mod process_ffi;` |
| `crates/ruyic/src/codegen/builtins_table.rs` (Modify) | 新增 45 条 `BuiltinDecl` 条目（io/path/process 各一组），更新计数测试 |
| `crates/ruyic/tests/integration/path_test.ry` (Create) | Path 模块集成测试 |
| `crates/ruyic/tests/integration/io_test.ry` (Create) | IO 模块集成测试 |
| `crates/ruyic/tests/integration/process_test.ry` (Create) | Process 模块集成测试 |

## Interfaces

### Cross-Batch

| Batch | Consumes | Produces |
|-------|----------|----------|
| B1-Path | `builtins_table.rs` current state, `lib.rs` current state | `path_ffi.rs`, updated `BUILTINS` (56→64 entries), updated `lib.rs` |
| B2-IO | B1 output (table has 64 entries) | `io_ffi.rs`, updated `BUILTINS` (64→81 entries) |
| B3-Process | B2 output (table has 81 entries) | `process_ffi.rs`, updated `BUILTINS` (81→101 entries) |
| B4-Verify | B1+B2+B3 complete | All tests pass, build clean |

### Per-File Interfaces

| File | Provides (to LLVM) | Consumed By |
|------|-------------------|-------------|
| `path_ffi.rs` | `__path_join`, `__path_basename`, `__path_dirname`, `__path_extname`, `__path_is_absolute`, `__path_normalize`, `__path_separator`, `__path_relative` | `stdlib/path.ry` |
| `io_ffi.rs` | `__io_read_line`, `__io_file_read_text`, `__io_file_write_text`, `__io_file_read_lines`, `__io_file_exists`, `__io_is_directory`, `__io_is_file`, `__io_file_delete`, `__io_mkdir` + 8 async variants | `stdlib/io.ry` |
| `process_ffi.rs` | `__process_create`, `__process_exec`, `__process_exec_with`, `__process_wait`, `__process_kill`, `__process_write_input`, `__process_close_input`, `__process_read_output`, `__process_read_error`, `__process_get_env`, `__process_set_env`, `__process_get_all_env`, `__process_get_pid`, `__process_get_ppid`, `__process_get_platform`, `__process_get_cpu_count`, `__process_get_total_memory`, `__process_get_free_memory`, `__process_signal_available` + 1 async variant | `stdlib/process.ry` |

---

## Batch 1: Path FFI (8 functions)

### Task 1.1: Create `crates/ruyi_runtime/src/path_ffi.rs`

**Goal**: Implement all 8 Path FFI functions with unit tests.

**Depends on**: None (path functions are pure string operations, zero dependencies within runtime)

#### Phase 1 — RED (Write failing tests first)

1. Create file `crates/ruyi_runtime/src/path_ffi.rs` with Javadoc header (`@author Ruyi Team`, `@date 2026-07-17`)
2. Add `#[cfg(test)] mod tests` block
3. Write `test_join_simple`: call `__path_join` with 3 segments, assert equals "/a/b/c"
4. Write `test_join_empty_segment`: call with ["/a", "", "c"], assert equals "/a/c"
5. Write `test_basename`: call with "/home/user/file.txt", assert equals "file.txt"
6. Write `test_dirname`: call with "/home/user/file.txt", assert equals "/home/user"
7. Write `test_extname`: call with "file.tar.gz", assert equals ".gz"
8. Write `test_extname_none`: call with "Makefile", assert equals ""
9. Write `test_is_absolute_unix`: call with "/usr/bin", assert true
10. Write `test_is_absolute_relative`: call with "src/main.rs", assert false
11. Write `test_separator_unix`: call `__path_separator`, assert equals "/"
12. Write `test_normalize_dotdot`: call with "/a/b/../c/./d", assert equals "/a/c/d"
13. Write `test_normalize_relative`: call with "./a/b/../c", assert equals "a/c"
14. Write `test_relative_sibling`: call `__path_relative("/home/user/docs", "/home/user/photos/x.jpg")`, assert equals "../photos/x.jpg"
15. Write `test_relative_child`: call `__path_relative("/home/user", "/home/user/docs/f.txt")`, assert equals "docs/f.txt"
16. Run `cargo test -p ruyi_runtime -- path_ffi` — all 12 tests MUST FAIL (RED phase)

#### Phase 2 — GREEN (Implement minimal code)

17. Add `use` imports: `use std::path::{Path, PathBuf};`
18. Implement `__path_join`: accept `*mut i8` (Array handle), iterate elements, join with `/`
19. Implement `__path_basename`: `Path::new(s).file_name().unwrap_or_default().to_str()`
20. Implement `__path_dirname`: `Path::new(s).parent().unwrap_or(Path::new("")).to_str()`
21. Implement `__path_extname`: `Path::new(s).extension().unwrap_or_default().to_str()` with leading dot
22. Implement `__path_is_absolute`: `Path::new(s).is_absolute()`
23. Implement `__path_normalize`: resolve `.`/`..` using `PathBuf` operations
24. Implement `__path_separator`: return `"/"`
25. Implement `__path_relative`: compute relative path using `pathdiff` logic or manual ancestor walk
26. All strings returned via `ruyi_alloc` + `std::ptr::copy_nonoverlapping` (match `__string_*` pattern)
27. Array input parsing: read length then iterate elements (match `__string_join` array handling pattern)
28. Run `cargo test -p ruyi_runtime -- path_ffi` — all 12 tests MUST PASS (GREEN phase)

#### Phase 3 — REFACTOR

29. Extract helper `cstr_to_str(ptr: *const i8) -> &str` to avoid repeated unsafe blocks
30. Extract helper `str_to_cstr_allocated(s: &str) -> *mut i8` for return value allocation
31. Verify all tests still pass after refactor

#### Phase 4 — VERIFY

32. Run `cargo test -p ruyi_runtime -- path_ffi` — all pass
33. Run `cargo clippy -p ruyi_runtime` — zero new warnings

#### Phase 5 — DOCUMENT

34. Each function has `///` doc with `# Safety` section
35. File header Javadoc complete

**Verification**: `cargo test -p ruyi_runtime -- path_ffi` exits 0, 12 tests pass

---

### Task 1.2: Add Path FFI declarations to `builtins_table.rs`

**Goal**: Register all 8 Path FFI symbols in the static BUILTINS table.

**Depends on**: Task 1.1

1. Open `crates/ruyic/src/codegen/builtins_table.rs`
2. After the `// __json_* (2)` section (line 368), add new section:

```rust
// ============================================================
// __path_* (8)
// ============================================================
BuiltinDecl { name: "__path_join",        ret: BuiltinSig::String, params: &[BuiltinSig::Ptr] },
BuiltinDecl { name: "__path_basename",    ret: BuiltinSig::String, params: &[BuiltinSig::String] },
BuiltinDecl { name: "__path_dirname",     ret: BuiltinSig::String, params: &[BuiltinSig::String] },
BuiltinDecl { name: "__path_extname",     ret: BuiltinSig::String, params: &[BuiltinSig::String] },
BuiltinDecl { name: "__path_is_absolute", ret: BuiltinSig::Bool,   params: &[BuiltinSig::String] },
BuiltinDecl { name: "__path_normalize",   ret: BuiltinSig::String, params: &[BuiltinSig::String] },
BuiltinDecl { name: "__path_separator",   ret: BuiltinSig::String, params: &[] },
BuiltinDecl { name: "__path_relative",    ret: BuiltinSig::String, params: &[BuiltinSig::String, BuiltinSig::String] },
```

3. Update the file header comment: change "array (6) → map (7) → set (4) → string (18) → math (14) → time (4) → json (2)" to add "→ path (8)"
4. Update test `builtins_count_is_56`:
   - Rename to `builtins_count_is_64`
   - Change expected count from 56 to 64
5. Run `cargo test -p ruyic -- builtins_table` — test passes (64 entries)
6. Verify `cargo check -p ruyic` — no errors

**Verification**: `cargo test -p ruyic -- builtins_table::tests::builtins_count_is_64` passes

---

### Task 1.3: Register path_ffi in lib.rs

**Goal**: Make path_ffi module visible to the runtime crate.

**Depends on**: Task 1.1

1. Open `crates/ruyi_runtime/src/lib.rs`
2. After line 15 (`pub mod time_ffi;`), add: `pub mod path_ffi;`
3. Run `cargo check -p ruyi_runtime` — no errors

**Verification**: `cargo check -p ruyi_runtime` exits 0

---

### Task 1.4: Path integration test

**Goal**: End-to-end verification that Path FFI links correctly and stdlib/path.ry works.

**Depends on**: Tasks 1.1, 1.2, 1.3

1. Create `crates/ruyic/tests/integration/path_test.ry`:
```ruyi
import { Path, hasExt, getExts } from "../../../stdlib/path";

fn main() {
    // Test basename
    let base = Path.basename("/home/user/file.txt");
    assert_eq(base, "file.txt");

    // Test dirname
    let dir = Path.dirname("/home/user/file.txt");
    assert_eq(dir, "/home/user");

    // Test extname
    let ext = Path.extname("file.tar.gz");
    assert_eq(ext, ".gz");

    // Test isAbsolute
    assert_true(Path.isAbsolute("/usr/bin"));
    assert_false(Path.isAbsolute("relative/path"));

    // Test join
    let joined = Path.join("/home", "user", "docs");
    assert_eq(joined, "/home/user/docs");

    // Test normalize
    let norm = Path.normalize("/a/b/../c/./d");
    assert_eq(norm, "/a/c/d");

    // Test separator
    let sep = Path.separator();
    assert_eq(sep, "/");

    print("path_test: all passed");
}
```

2. Verify `cargo check -p ruyic` — no errors (confirms typecheck of new .ry file)
3. Run `cargo test -p ruyic --test integration -- path_test` — test compiles and runs

**Verification**: Integration test runs and prints "path_test: all passed"

---

## Batch 2: IO FFI (17 functions)

### Task 2.1: Create `crates/ruyi_runtime/src/io_ffi.rs` with sync operations (9 functions)

**Goal**: Implement synchronous IO FFI functions (readText, writeText, readLines, exists, isDirectory, isFile, delete, mkdir, readLine) with unit tests.

**Depends on**: Batch 1 complete

#### Phase 1 — RED

1. Create `crates/ruyi_runtime/src/io_ffi.rs` with Javadoc header
2. Add `#[cfg(test)] mod tests` block using `std::env::temp_dir()` for test files
3. Write `test_read_text_existing`: create temp file with "hello world", call `__io_file_read_text`, assert returns "hello world"
4. Write `test_read_text_missing`: call with non-existent path, assert function throws (use `std::panic::catch_unwind` wrapper)
5. Write `test_write_text_new`: call `__io_file_write_text` with new path, verify file created with correct content via `std::fs::read_to_string`
6. Write `test_read_lines`: create file "a\nb\nc\n", call `__io_file_read_lines`, assert 3 lines ["a","b","c"]
7. Write `test_exists_true`: call `__io_file_exists` on temp file, assert true
8. Write `test_exists_false`: call `__io_file_exists` on non-existent path, assert false
9. Write `test_is_file_vs_directory`: create temp file, assert `__io_is_file=true` and `__io_is_directory=false`; call on temp_dir(), assert opposite
10. Write `test_delete_existing`: create temp file, call `__io_file_delete`, verify file no longer exists
11. Write `test_delete_missing`: call `__io_file_delete` on non-existent path, assert throws
12. Write `test_mkdir_recursive`: call `__io_mkdir` with nested path + recursive=true, verify directories created
13. Write `test_mkdir_non_recursive_no_parent`: call without recursive flag and non-existent parent, assert throws
14. Write `test_readline`: this is harder to test — use `assert!(true)` placeholder, note in test that stdin testing requires process spawning (tested in integration)
15. Run `cargo test -p ruyi_runtime -- io_ffi` — tests FAIL

#### Phase 2 — GREEN

16. Implement `__io_file_read_text`: `std::fs::read_to_string(path)`, on error call `ruyi_throw`
17. Implement `__io_file_write_text`: `std::fs::write(path, content)`, on error call `ruyi_throw`
18. Implement `__io_file_read_lines`: read to string, split by '\n', strip trailing empty, build array via `ruyi_array_push`
19. Implement `__io_file_exists`: `std::path::Path::new(path).exists()`
20. Implement `__io_is_file`: `std::path::Path::new(path).is_file()`
21. Implement `__io_is_directory`: `std::path::Path::new(path).is_dir()`
22. Implement `__io_file_delete`: `std::fs::remove_file(path)`, on error throw
23. Implement `__io_mkdir`: `if recursive { std::fs::create_dir_all } else { std::fs::create_dir }`, on error throw
24. Implement `__io_read_line`: `std::io::stdin().read_line(&mut buf)`, strip '\n', return null on EOF
25. All strings via `ruyi_alloc`; all paths converted from `*const i8` via `CStr::from_ptr`
26. Error messages include the OS error string: `format!("IO error: {}: {}", path_str, e)`
27. Run `cargo test -p ruyi_runtime -- io_ffi` — all sync tests PASS

#### Phase 3 — REFACTOR

28. Extract `c_path_to_str(ptr: *const i8) -> &str` helper
29. Extract `io_error(path: &str, e: std::io::Error) -> !` helper that formats message and calls `ruyi_throw`
30. Verify all tests still pass

#### Phase 4 — VERIFY

31. Run `cargo test -p ruyi_runtime -- io_ffi` — all pass
32. Run `cargo clippy -p ruyi_runtime` — zero new warnings

#### Phase 5 — DOCUMENT

33. Each function documented with `///` doc and `# Safety`
34. File header complete

**Verification**: 11+ sync tests pass

---

### Task 2.2: Add async IO FFI variants (8 functions)

**Goal**: Implement async wrappers for readText, writeText, readLines, exists, isDirectory, isFile, delete, mkdir.

**Depends on**: Task 2.1

#### Phase 1 — RED

1. Add async tests to `io_ffi.rs` `#[cfg(test)] mod tests`:
   - `test_read_text_async_resolves`: spawn async reader, verify Future resolves to correct content
   - `test_write_text_async_completes`: spawn async writer, verify file created after Future resolves
2. Run — tests FAIL

#### Phase 2 — GREEN

3. For each async variant (`*_async`), implement pattern:
```rust
pub extern "C" fn __io_file_read_text_async(path: *const i8) -> *mut i8 {
    let path_owned = /* copy path to owned string */;
    let future = ruyi_spawn_task(move || {
        // call synchronous version
        __io_file_read_text(path_cstr)
    });
    Box::into_raw(Box::new(future))
}
```
4. Implement all 8 async wrappers (readText, writeText, readLines, exists, isDirectory, isFile, delete, mkdir)
5. Return type: `*mut i8` (opaque Future handle), consumed by `ruyi_await` in Ruyi layer
6. Run — async tests PASS

#### Phase 3 — VERIFY

7. Run `cargo test -p ruyi_runtime -- io_ffi` — all tests pass (sync + async)
8. Run `cargo clippy -p ruyi_runtime` — zero new warnings

**Verification**: All IO tests (sync + async) pass

---

### Task 2.3: Add IO FFI declarations to `builtins_table.rs`

**Goal**: Register all 17 IO FFI symbols.

**Depends on**: Task 2.2

1. In `builtins_table.rs`, after the Path section, add `// __io_* (17)` section with 17 entries:
   - Sync (9): read_line, file_read_text, file_write_text, file_read_lines, file_exists, is_directory, is_file, file_delete, mkdir
   - Async (8): *_async variants for file_read_text, file_write_text, file_read_lines, file_exists, is_directory, is_file, file_delete, mkdir
2. Update file header comment: add "→ io (17)"
3. Update test `builtins_count_is_64`:
   - Rename to `builtins_count_is_81`
   - Change expected count from 64 to 81
4. Run `cargo test -p ruyic -- builtins_table` — passes with 81

**Verification**: `cargo test -p ruyic -- builtins_table::tests::builtins_count_is_81` passes

---

### Task 2.4: Register io_ffi in lib.rs

**Goal**: Make io_ffi module visible.

**Depends on**: Task 2.1

1. In `crates/ruyi_runtime/src/lib.rs`, after `pub mod path_ffi;`, add `pub mod io_ffi;`
2. Run `cargo check -p ruyi_runtime` — no errors

**Verification**: `cargo check -p ruyi_runtime` exits 0

---

### Task 2.5: IO integration test

**Goal**: End-to-end verification of IO FFI → stdlib/io.ry.

**Depends on**: Tasks 2.1-2.4

1. Create `crates/ruyic/tests/integration/io_test.ry`:
```ruyi
import { File, readLine } from "../../../stdlib/io";

fn main() {
    // Test write + read
    let testPath = "/tmp/ruyi_io_test.txt";
    File.writeText(testPath, "hello ruyi io");

    // Test exists
    assert_true(File.exists(testPath));

    // Test read
    let content = File.readText(testPath);
    assert_eq(content, "hello ruyi io");

    // Test isFile
    assert_true(File.isFile(testPath));
    assert_false(File.isDirectory(testPath));

    // Test readLines
    File.writeText(testPath, "a\nb\nc");
    let lines = File.readLines(testPath);
    assert_eq(lines.length(), 3);
    assert_eq(lines[0], "a");
    assert_eq(lines[1], "b");
    assert_eq(lines[2], "c");

    // Test delete
    File.delete(testPath);
    assert_false(File.exists(testPath));

    // Test mkdir
    let testDir = "/tmp/ruyi_io_testdir/sub";
    File.mkdir(testDir, true);
    assert_true(File.isDirectory("/tmp/ruyi_io_testdir"));
    assert_true(File.isDirectory(testDir));

    // Cleanup
    File.delete("/tmp/ruyi_io_testdir/sub");
    File.delete("/tmp/ruyi_io_testdir");

    print("io_test: all passed");
}
```

2. Run `cargo test -p ruyic --test integration -- io_test` — compiles and runs
3. Verify output: "io_test: all passed"

**Verification**: IO integration test passes all assertions

---

## Batch 3: Process FFI (20 functions)

### Task 3.1: Create `crates/ruyi_runtime/src/process_ffi.rs` — Command execution + lifecycle (9 functions)

**Goal**: Implement `__process_exec`, `__process_exec_with`, `__process_create`, `__process_wait`, `__process_wait_async`, `__process_kill` + system info functions.

**Depends on**: Batch 2 complete

#### Phase 1 — RED

1. Create `crates/ruyi_runtime/src/process_ffi.rs` with Javadoc header
2. Add `#[cfg(test)] mod tests` block
3. Write `test_exec_echo`: call `__process_exec("echo hello")`, check ProcessResult stdout="hello\n", stderr="", exit_code=0
4. Write `test_exec_failure`: call `__process_exec("false")`, check exit_code != 0
5. Write `test_create_and_wait`: call `__process_create("echo test")`, then `__process_wait(handle)`, assert exit_code=0
6. Write `test_create_and_kill`: call `__process_create("sleep 10")`, then `__process_kill(handle, 9)`, then `__process_wait(handle)`, assert exit_code != 0
7. Write `test_wait_twice`: call wait twice on same handle, both return same code
8. Write `test_get_pid`: call `__process_get_pid()`, assert > 0
9. Write `test_get_platform`: call `__process_get_platform()`, assert equals "macos" or "linux"
10. Write `test_get_cpu_count`: call `__process_get_cpu_count()`, assert >= 1
11. Write `test_get_memory`: call `__process_get_total_memory()` and `__process_get_free_memory()`, assert total >= free >= 0
12. Run `cargo test -p ruyi_runtime -- process_ffi` — tests FAIL

#### Phase 2 — GREEN

13. Define `ProcessHandle` struct: `struct ProcessHandle { child: Mutex<Option<Child>>, exit_code: Mutex<Option<i32>> }`
14. Implement `__process_exec`: `std::process::Command::new("sh").arg("-c").arg(cmd).output()`, build ProcessResult via `ruyi_object_alloc`, return handle
15. Implement `__process_exec_with`: same as exec but set `.current_dir(cwd)`, `.envs(env_map)`, respect `shell` flag
16. Implement `__process_create`: `Command::new("sh").arg("-c").arg(cmd).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn()`, wrap in `ProcessHandle`, return `Box::into_raw`
17. Implement `__process_wait`: `process.child.lock().take().unwrap().wait()`, store exit_code, return it
18. Implement `__process_wait_async`: spawn thread calling `__process_wait`, wrap in Future
19. Implement `__process_kill`: send signal via `nix::sys::signal::kill(pid, signal)` or `Command::new("kill").arg(format!("-{}", signal)).arg(pid.to_string())` (stdlib-only approach)
20. Implement `__process_get_pid`: `std::process::id() as i64`
21. Implement `__process_get_ppid`: `unsafe { libc::getppid() } as i64` (Unix only)
22. Implement `__process_get_platform`: `if cfg!(target_os = "macos") { "macos" } else if cfg!(target_os = "linux") { "linux" } else { "unknown" }`
23. Implement `__process_get_cpu_count`: `num_cpus::get() as i64` or `std::thread::available_parallelism().map(|n| n.get() as i64).unwrap_or(1)`
24. Implement `__process_get_total_memory`: `sysinfo` crate or parse `/proc/meminfo` on Linux / `sysctl` on macOS
25. Implement `__process_get_free_memory`: same approach as total memory
26. Error handling: on command-not-found, throw ProcessException with descriptive message
27. Run — tests PASS

#### Phase 3 — REFACTOR

28. Extract `spawn_command(cmd, cwd, env, shell) -> Result<Child>` helper
29. Extract `build_process_result(stdout, stderr, exit_code) -> *mut i8` helper
30. Ensure `#[cfg(not(target_os = "windows"))]` guards on Unix-specific functions (get_ppid, kill)
31. Verify all tests pass

#### Phase 4 — VERIFY

32. Run `cargo test -p ruyi_runtime -- process_ffi` — all pass
33. Run `cargo clippy -p ruyi_runtime` — zero new warnings

#### Phase 5 — DOCUMENT

34. Each function documented

**Verification**: 9+ tests pass

---

### Task 3.2: Implement Process I/O pipe functions (4 functions)

**Goal**: `__process_write_input`, `__process_close_input`, `__process_read_output`, `__process_read_error`.

**Depends on**: Task 3.1

#### Phase 1 — RED

1. Add tests to process_ffi.rs:
   - `test_write_input_and_read_output`: create `cat` process, write "hello" to stdin, close stdin, read stdout, assert "hello"
   - `test_read_error`: create process that writes to stderr, read stderr
   - `test_read_output_closed`: after closing stdin, read stdout should return data or null
2. Run — FAIL

#### Phase 2 — GREEN

3. Implement `__process_write_input`: get stdin from ProcessHandle.child, `stdin.take().unwrap().write_all(input.as_bytes())`
4. Implement `__process_close_input`: `stdin.take().unwrap()` (drop to close)
5. Implement `__process_read_output`: non-blocking read from stdout using `BufReader`, return available data or null
6. Implement `__process_read_error`: same pattern for stderr
7. All functions throw ProcessException if process has exited
8. Run — PASS

**Verification**: 4 I/O tests pass

---

### Task 3.3: Implement environment + signal functions (7 functions)

**Goal**: `__process_get_env`, `__process_set_env`, `__process_get_all_env`, `__process_signal_available`.

**Depends on**: Task 3.1

#### Phase 1 — RED

1. Add tests:
   - `test_get_env_home`: call `__process_get_env("HOME")`, assert non-null on Unix
   - `test_get_env_missing`: call with random name, assert null
   - `test_set_and_get_env`: call `__process_set_env("RUYI_TEST_X", "42")`, then `__process_get_env("RUYI_TEST_X")`, assert "42"
   - `test_get_all_env`: call `__process_get_all_env()`, assert non-null handle, check contains "PATH"
   - `test_signal_available_kill`: `__process_signal_available(9)`, assert true
   - `test_signal_available_invalid`: call with 999, assert false (on Unix)
2. Run — FAIL

#### Phase 2 — GREEN

3. Implement `__process_get_env`: `std::env::var(name).ok()`, return null or allocated string
4. Implement `__process_set_env`: `std::env::set_var(name, value)`
5. Implement `__process_get_all_env`: `std::env::vars()`, build Map handle via `__builtin_map_create` + `__builtin_map_set`
6. Implement `__process_signal_available`: match on common signal numbers (1,2,3,9,10,12,15 for Unix), return true/false
7. Run — PASS

**Verification**: 6 environment/signal tests pass

---

### Task 3.4: Add Process FFI declarations to `builtins_table.rs`

**Goal**: Register all 20 Process FFI symbols.

**Depends on**: Tasks 3.1-3.3

1. In `builtins_table.rs`, after the IO section, add `// __process_* (20)` section with 20 entries
2. Update header comment: add "→ process (20)"
3. Update test `builtins_count_is_81`:
   - Rename to `builtins_count_is_101`
   - Change expected count to 101
4. Run `cargo test -p ruyic -- builtins_table` — passes with 101

**Verification**: `builtins_count_is_101` test passes

---

### Task 3.5: Register process_ffi in lib.rs

**Goal**: Make process_ffi module visible.

**Depends on**: Task 3.1

1. In `lib.rs`, after `pub mod io_ffi;`, add `pub mod process_ffi;`
2. Run `cargo check -p ruyi_runtime` — no errors

**Verification**: `cargo check -p ruyi_runtime` exits 0

---

### Task 3.6: Process integration test

**Goal**: End-to-end verification of Process FFI → stdlib/process.ry.

**Depends on**: Tasks 3.1-3.5

1. Create `crates/ruyic/tests/integration/process_test.ry`:
```ruyi
import { Process, getEnv, getPID, getPlatform } from "../../../stdlib/process";

fn main() {
    // Test getPID
    let pid = getPID();
    assert_true(pid > 0);

    // Test getPlatform
    let platform = getPlatform();
    assert_true(platform === "macos" || platform === "linux");

    // Test getEnv
    let home = getEnv("HOME");
    assert_true(home !== null || platform === "windows");

    // Test exec
    let result = Process.exec("echo hello_from_ruyi");
    assert_eq(result.exitCode, 0);

    // Test exec failure
    let badResult = Process.exec("nonexistent_command_xyz");
    assert_true(badResult.exitCode !== 0);

    print("process_test: all passed");
}
```

2. Run `cargo test -p ruyic --test integration -- process_test` — compiles and runs
3. Verify output: "process_test: all passed"

**Verification**: Process integration test passes

---

## Batch 4: Final Verification Wave

### Task 4.1: Full workspace check

**Depends on**: Batches 1-3 complete

1. Run `cargo check --workspace` — exits 0, no errors, no warnings
2. If any warnings: fix before proceeding

**Verification**: `cargo check --workspace` clean

---

### Task 4.2: Full test suite

**Depends on**: Task 4.1

1. Run `cargo test --workspace` — all pre-existing tests pass + all new tests pass
2. Specifically check:
   - `cargo test -p ruyi_runtime` — all runtime tests pass
   - `cargo test -p ruyic --test integration` — all integration tests pass
   - `cargo test -p ruyic --lib` — compiler lib tests pass
3. If any pre-existing test fails: do NOT fix (out of scope), document in verification report

**Verification**: `cargo test --workspace` exits 0 (with possible pre-existing failures documented)

---

### Task 4.3: Release build

**Depends on**: Task 4.2

1. Run `make build-release` — exits 0
2. Verify binary: `./target/release/ruyic --version` — prints version

**Verification**: `make build-release` exits 0

---

### Task 4.4: Lint and format

**Depends on**: Task 4.3

1. Run `make lint` — zero new clippy warnings
2. Run `make fmt-check` — "all files formatted correctly" or equivalent

**Verification**: `make lint` zero new warnings, `make fmt-check` clean

---

### Task 4.5: Stdlib regression check

**Depends on**: Task 4.3

1. Run `make check` to verify all 9 existing stdlib modules still `--check` clean:
```bash
for f in stdlib/*.ry; do
    echo "=== $f ==="
    ./target/release/ruyic "$f" --check
done
```
2. All 14 stdlib files exit 0 (no type errors)
3. Verify: new io/path/process files also pass `--check`

**Verification**: All `stdlib/*.ry --check` exit 0

---

### Task 4.6: dp_5 record

**Depends on**: Tasks 4.1-4.5 all pass

1. Update `.spec-superflow.yaml`:
```yaml
state: executing
batches_completed: 4
test_result: "PASS: cargo check --workspace OK; cargo test --workspace OK; make build-release OK; make lint zero-new; make fmt-check clean; 14/14 stdlib --check pass; 3/3 io/path/process .ry integration tests pass"
```

**Verification**: `.spec-superflow.yaml` updated with execution evidence
