# Ruyi Roadmap

> **Version**: 0.5.10 | **Date**: 2026-07-25 | **Status**: Phase 1 Complete
>
> [中文版](roadmap-zh.md)

## TL;DR

Ruyi is a compiled programming language targeting native code via LLVM. This roadmap defines three phases:

1. **Foundation Library (v0.2–v0.5)** — Fix broken stdlib, fill core gaps (math/time/json/regex/random), complete codegen (classes/objects/try-catch/for-loops), wire runtime (GC/async/exceptions)
2. **Ecosystem & Package Management (v0.6–v0.9)** — Package manager (`rupm`), registry, lockfile, workspace support, build system
3. **Developer Tooling (v1.0+)** — LSP, formatter, linter, test runner, doc generator, debugger, IDE plugins

---

## Version Release Status

| Version | Branch | Status | Release Date | Tag |
|---------|--------|--------|-------------|-----|
| v0.2 | dev/v0.2 | ✅ Released | 2026-05 | v0.2.0 (待补打) |
| v0.3 | dev/v0.3 | ✅ Released | 2026-05 | v0.3.0 (待补打) |
| v0.4 | dev/v0.4 | ✅ Released | 2026-05 | v0.4.0 |
| v0.5 | dev/v0.5 | ✅ Released | 2026-05 | v0.5.1 |
| v0.5.2 | dev/v0.5.2 | ✅ Released | 2026-05 | v0.5.2 |
| v0.5.3 | dev/v0.5.3 | ✅ Released | 2026-05 | v0.5.3 |
| v0.5.4 | dev/v0.5.4 | ✅ Released | 2026-07 | v0.5.4 |
| v0.5.5 | dev/v0.5.5 | ✅ Released | 2026-07 | v0.5.5 |
| v0.5.6 | dev/v0.5.6-housekeeping | ✅ Released | 2026-07 | (无 tag，housekeeping) |
| v0.5.7 | dev/v0.5.7-p1-defects | ✅ Released | 2026-07-12 | v0.5.7 |
| v0.5.8 | dev/v0.5.8 | ✅ Released | 2026-07-12 | v0.5.8 |
| v0.5.9 | dev/v0.5.9-stdlib-cleanup | ✅ Released | 2026-07-12 | v0.5.9 |
| v0.5.10 | dev/v0.5.10 | ✅ Released | 2026-07-25 | v0.5.10 |

---

## Current State Assessment

### Compiler Modules

| Module | Completeness | Key Gaps |
|--------|-------------|----------|
| **Lexer** | ~95% | Doc comments not specially handled |
| **Parser** | ~65% | Match guards, computed property names, generic syntax need verification |
| **Typechecker** | ~95% | Trait bounds enforced (v0.5.5); supertraits unchecked; `impl Trait for Type` basic support (v0.5.5) |
| **Codegen** | ~88% | Member access, array/object literals, template strings, for-loops, try/catch, break, class layout supported; compound assignment (5 ops), anonymous functions, async arrows, complex new (.new pattern), array index assignment supported; BigInt, indirect calls, spread arguments not yet supported |
| **Macro Expand** | ~60% | Complex repetition patterns, hygiene edge cases |
| **Driver** | ~85% | Runtime statically linked (v0.5.5); module system inlines rather than proper imports |
| **GC** | ~85% (compiler) / 100% (runtime) | Dual mode: --gc=stub (default) + --gc=real (v0.5.5) |
| **Runtime** | ~75% (compiler) / 98% (library) | `ruyi_await` is real async + spawn builtin (v0.5.5); thread support (Channel/Thread/RWLock/TLS/spawn_blocking, v0.5.10) |

### Standard Library

| Module | Lines | Status | Gaps |
|--------|-------|--------|------|
| `core.ry` | 219 | ✅ Complete | String/Int/Float/Bool builtins working |
| `string.ry` | 312 | ✅ Complete | Overlaps with core.ry String module |
| `io.ry` | 192 | ✅ Complete | Console I/O + File class with async variants |
| `error.ry` | 230 | ✅ Complete | 9 error types + assert/asserNotNull |
| `option.ry` | 175 | ✅ Complete | Option\<T\> enum with all combinators |
| `result.ry` | 190 | ✅ Complete | Result\<T,E\> enum with all combinators |
| `process.ry` | 509 | ✅ Complete | Process management + environment |
| `path.ry` | 262 | ✅ Complete | Path manipulation with async variants |
| `collections.ry` | 529 | ⚠️ Partial | **SetIterator.next() is a broken stub**; missing sort/contains/indexOf/first/last |
| `encoding.ry` | 803 | ✅ Complete | Base64/Base64URL/Hex/URL encode/decode (all pure .ry) |
| `bigint.ry` | 638 | ✅ Complete | Big integer type with basic arithmetic |
| `random.ry` | 120 | ✅ Complete | Xorshift PRNG wrapper (wired to `random_ffi`) |
| `json.ry` | 149 | ✅ Complete | JSON parse and serialization |
| `uuid.ry` | 155 | ✅ Complete | UUID v4 generation (depends on `random.ry`) |
| `datetime.ry` | 983 | ✅ Complete | Date class + datetime utilities (depends on `time` FFI) |
| `sort.ry` | 526 | ✅ Complete | Pure .ry sorting algorithms (quicksort/insertion/merge) |
| `buffer.ry` | 1,392 | ✅ Complete | Buffer class: endian read/write, UTF-8, Base64/Hex, float (pure .ry) |
| `fs.ry` | 1,329 | ✅ Complete | File system module: 70 exports, walkDir/copyDir/ensureDir |
| `crypto.ry` | 1,782 | ✅ Complete | SHA-256/512/1 + MD5 + HMAC + PBKDF2 + CSPRNG (1 extern FFI) |
| `net.ry` | 253 | ✅ Complete | TCPSocket/TCPServer/UDPSocket: TCP client/server + UDP (15 extern FFI) |
| `regex.ry` | 390 | ✅ Complete | Regex engine: Thompson NFA, capture groups, quantifiers, char classes (pure .ry) |
| `fmt.ry` | 120 | ✅ Complete | Format strings |
| `test.ry` | 180 | ✅ Complete | Built-in test framework: @test attribute + assertion helpers |
| `thread.ry` | 78 | ✅ Complete | Thread: spawn/join/detach/id/cpuCount/sleep (v0.5.10) |
| `channel.ry` | 101 | ✅ Complete | Channel: bounded/unbounded MPSC + select (v0.5.10) |
| `rwlock.ry` | 113 | ✅ Complete | RWLock: concurrent read/write lock (v0.5.10) |
| `thread_local.ry` | 54 | ✅ Complete | ThreadLocal: per-thread key-value storage (v0.5.10) |

**Completed modules**: `math`, `datetime`, `json`, `random`, `fmt`, `test`, `encoding`, `bigint`, `uuid`, `sort`, `buffer`, `fs`, `crypto`, `thread`, `channel`, `rwlock`, `thread_local`
**Critical Missing Modules**: `http` (HTTP/HTTPS client)

### Test Infrastructure

| Area | Count | Status |
|------|-------|--------|
| Unit tests (lexer/parser/typechecker/etc.) | ~2400+ | ✅ Solid |
| Integration tests (.ry files) | 58 cases | ⚠️ Only ~30% of spec surface |
| Runtime tests | 3 files | ⚠️ Basic coverage |
| Benchmarks | criterion suite | ✅ Exists |
| CI/CD | ❌ None | No GitHub Actions |
| Property testing | ❌ None | No proptest |
| Fuzzing | ❌ None | No cargo-fuzz |

**Spec features with ZERO integration tests**: class/OOP, trait system, macros, import/export, type aliases, deep pattern matching, destructuring, `for-of`/`for-in`, bigint, `never` type, ARC classes

---

## Phase 1: Foundation Library (v0.2–v0.5)

### Goal

Make Ruyi capable of writing real programs end-to-end: classes work, exceptions work, async actually runs, and the stdlib covers the 80% use case.

### v0.2 — Codegen Completion (Priority: CRITICAL)

> Without member access and class layout, no real program can compile.

| # | Task | Description | Priority |
|---|------|-------------|----------|
| 1.1 | **Class layout & member access** | Implement `compile_class` (currently no-op): field layout, `self.field` access, `new` constructor, method dispatch | P0 ✅ |
| 1.2 | **Object literal codegen** | Compile `{ key: value }` expressions to runtime structures | P0 ✅ |
| 1.3 | **Array literal codegen** | Compile `[1, 2, 3]` to runtime array with `push`/`pop`/index access | P0 ✅ |
| 1.4 | **String concatenation** | `+` operator for strings (currently only numeric `+` works) | P0 ✅ |
| 1.5 | **For loop codegen** | C-style `for`, `for-in`, `for-of` (all currently unsupported) | P0 ✅ |
| 1.6 | **Break/continue** | Already have `loop_stack`, just need codegen | P1 ✅ |
| 1.7 | **Try/catch/finally** | Landing pad support exists in `ruyi_runtime`; wire it into codegen | P0 ✅ |
| 1.8 | **Throw expression** | Map to runtime `throw_exception` call | P1 ✅ |
| 1.9 | **Match statement** | Compile match to chained if-else or switch | P1 ✅ |
| 1.10 | **Template literals** | Compile `` `Hello ${name}` `` to string concatenation | P1 ✅ |
| 1.11 | **BigInt literal** | Compile `100n` to runtime bigint type | P2 |
| 1.12 | **Member expression** | `obj.prop` and `obj?.prop` codegen (currently unsupported) | P0 ✅ |
| 1.13 | **Method call** | `obj.method(args)` codegen with `self` binding | P0 ✅ |

**Status (2026-07-11, post-v0.5.5-residual-fixes)**:

Batch 1+2 codegen work landed on `dev/v0.2-codegen-gaps` (merged to `dev/v0.5.5` via `b2853fc`):
- **T2** (`65f514c`) sized class allocation correctly (1.1 partial).
- **T3** (`bed00d7`) resolved class fields and own methods in member access (1.12, 1.13).
- **T4** (`6618b11`) wired labeled `break`/`continue` via `loop_stack` (1.6).
- **T6** (`fc01bcb`) added `ruyi_obj_get` / `ruyi_obj_keys` FFI (1.2).
- **T8** added 5 examples + 8 integration test fixtures exercising each capability.
- **T9** (`809e6c9`) recognized `RangeError` / `ArrayIterator` as Named types but did not make them callable as constructors.

T9 closure (v0.5.5-residual-fixes, shipped with v0.5.5):
- **T-1.3.1** (`21028de`) made `RangeError` and `ArrayIterator` constructible via `.new(...)` pattern.
- **T-1.3.2** (`3245ae2`) enabled 21 codegen integration tests previously blocked by T9.
- **T-1.3.3** (`9e1d30a`) fixed template literal `value_to_i8_ptr` for non-string interpolation (`int`/`float` → runtime converters); un-ignored `codegen_template_literal` (closes 1.10).
- **T-1.1** (`c625b9f`) enabled 2 throw-unreachable tests (closes 1.8 codegen path).
- **T-1.2** (`0a35a71`) enabled 12 try_catch_invoke tests (closes 1.8 downstream).
- **T-1.4** (`10ca3c7`) enforced trait bounds (unblocks 3.2 supertrait follow-on).

Net result: 1.8 Throw / 1.9 Match / 1.10 Template are FULL in `crates/ruyic/src/codegen/`. Remaining P1 items live in Typechecker (3.2/3.4/3.5/3.6), Runtime (2.6), and Stdlib (4.5/4.6/4.8/4.9) — tracked separately under `v0.5.7-p1-defects` (deferred).

### v0.3 — Runtime Integration (Priority: CRITICAL)

> The runtime GC, async scheduler, and exception handling exist but aren't wired to codegen.

| # | Task | Description | Priority |
|---|------|-------------|----------|
| 2.1 | **Link runtime library** | Driver must link `ruyi_runtime` into produced binaries (currently uses bare `cc`) | P0 |
| 2.2 | **GC allocation wired** | Replace placeholder allocators with `ruyi_gc_alloc`/`ruyi_gc_collect` | P0 |
| 2.3 | **Async actually async** | Replace no-op `ruyi_await` with real future polling via work-stealing scheduler | P0 |
| 2.4 | **`spawn` built-in** | Implement `spawn(fn)` to launch green threads on the scheduler | P0 |
| 2.5 | **Exception landing pads** | Call `ruyi_exception_try`/`ruyi_exception_catch` from try/catch codegen | P0 |
| 2.6 | **Async GC roots** | `register_async_roots` currently no-op; register suspended tasks | P1 ✅ (v0.5.7) |
| 2.7 | **Thread-local GC heaps** | Wire multi-threaded GC to async runtime | P2 ✅ (v0.5.10) |

### v0.4 — Typechecker Hardening (Priority: HIGH)

| # | Task | Description | Priority |
|---|------|-------------|----------|
| 3.1 | **Enforce trait bounds** | `check_bounds()` in generics.rs currently returns true; actually verify impl exists | P0 ✅ |
| 3.2 | **Supertrait checking** | Populate and validate `supertraits` field | P1 ✅ (v0.5.7) |
| 3.3 | **Full `impl Trait for Type`** | Support standalone `impl Printable for string { ... }` (currently incomplete) | P0 ✅ |
| 3.4 | **Type narrowing beyond null** | Narrowing after `instanceof`, `typeof`, match patterns | P1 ✅ (v0.5.7) |
| 3.5 | **Exhaustiveness checking** | Verify match arms cover all cases; warn on incomplete patterns | P1 ✅ (v0.5.7) |
| 3.6 | **Self-referential type checking** | Classes referencing `self` in field types | P1 ✅ (v0.5.7) |

### v0.5 — Standard Library Expansion (Priority: HIGH)

| # | Task | Description | Priority |
|---|------|-------------|----------|
| 4.1 | **Fix SetIterator** | `SetIterator.next()` — implement proper set iteration (fixed in v0.5.9) | P0 ✅ |
| 4.2 | **`math.ry`** | Pi, E, sqrt, pow, sin, cos, tan, asin, acos, atan, log, log10, exp, abs, min, max | P0 ✅ |
| 4.3 | **`time.ry`** | Duration, Timestamp, sleep (sync + async), Date formatting | P0 ✅ |
| 4.4 | **`json.ry`** | JSON.parse, JSON.stringify with type-safe deserialization | P0 ✅ |
| 4.5 | **`random.ry`** | Random.nextInt, nextFloat, nextBool, nextBytes, seed | P1 ✅ (v0.5.7) |
| 4.6 | **`fmt.ry`** | Format strings: `fmt.format("{} is {} years old", name, age)` | P1 ✅ (v0.5.7) |
| 4.7 | **`regex.ry`** | Regex class with match, replace, split (Thompson NFA, pure .ry) | P2 ✅ (v0.5.9 Phase 6) |
| 4.8 | **`test.ry`** | Built-in test framework: `@test` attribute, assert, assertEq, assertThrows | P1 ✅ (v0.5.7) |
| 4.9 | **Expand `collections.ry`** | Array.sort, .contains, .indexOf, .first, .last, .slice, .concat; Iterator.takeWhile, .skipWhile, .chain, .enumerate, .zip, .sum, .product, .any, .all | P1 ✅ (v0.5.7) |
| 4.10 | **Merge `core.ry` + `string.ry`** | Duplicate String methods; consolidate into one module | P2 |
| 4.11 | **`buffer.ry`** | Buffer/ByteArray type for binary data | P2 ✅ (v0.5.9 Phase 3) |
| 4.12 | **`net.ry`** | TCPClient, TCPServer (basic socket I/O) | P2 ✅ (v0.5.9 Phase 5) |
| 4.13 | **`encoding.ry`** | Base64/Base64URL/Hex/URL encode/decode | P2 ✅ (v0.5.9 Phase 2) |
| 4.14 | **`fs.ry`** | File system operations (directory listing, metadata, recursive ops) | P2 ✅ (v0.5.9 Phase 3) |
| 4.15 | **`sort.ry`** | Pure .ry sorting algorithms | P2 ✅ (v0.5.9 Phase 2) |
| 4.16 | **`datetime.ry`** | Date class + datetime utilities | P2 ✅ (v0.5.9 Phase 2) |
| 4.17 | **`crypto.ry`** | SHA-256/512/1 + MD5 + HMAC + PBKDF2 + CSPRNG | P2 ✅ (v0.5.9 Phase 4) |
| 4.18 | **`uuid.ry`** | UUID v4 generation | P2 ✅ (v0.5.9 Phase 1) |
| 4.19 | **`bigint.ry`** | Big integer type | P2 ✅ (v0.5.9 Phase 1) |

### v0.5+ — Cryptography Expansion (HTTPS/TLS Prerequisites)

The following modules are required for HTTPS/TLS support, listed in dependency order:

| # | Module | Description | Depends On | Est. LOC |
|---|--------|-------------|------------|----------|
| C1 | **`crypto-aes.ry`** | AES-128/256 encrypt/decrypt + GCM/CBC modes (pure .ry S-box table) | — | ~800 |
| C2 | **`crypto-hkdf.ry`** | HKDF key derivation (RFC 5869), based on HMAC-SHA256 | `crypto.ry` | ~200 |
| C3 | **`crypto-bigint.ry`** | Big integer enhancements: modular exponentiation, Montgomery multiplication, Miller-Rabin primality testing | `bigint.ry` | ~500 |
| C4 | **`crypto-ecc.ry`** | Elliptic curves (secp256r1/Curve25519): finite-field point addition/doubling, ECDH key exchange | `crypto-bigint.ry` | ~1,200 |
| C5 | **`crypto-rsa.ry`** | RSA key generation/encryption/signing (PKCS#1 v1.5 / OAEP / PSS) | `crypto-bigint.ry` | ~800 |
| C6 | **`tls.ry`** | TLS 1.3 protocol: handshake state machine, Record Layer, certificate chain validation, X.509/ASN.1 parsing | All above + `net.ry` | ~2,500+ |

**Total estimated**: ~6,000 lines of pure .ry, zero new FFI (all built on existing primitives).

---

## Phase 2: Ecosystem & Package Management (v0.6–v0.9)

### Goal

Enable developers to share code, manage dependencies, and build multi-package projects.

### v0.6 — Package Manager Foundation (Priority: HIGH)

| # | Task | Description |
|---|------|-------------|
| 5.1 | **Manifest format** | Define `ruyi.pkg` (TOML): `[package]` name/version/edition, `[dependencies]` with semver, `[dev-dependencies]` |
| 5.2 | **Lockfile generation** | `ruyi.lock` with full resolution tree (name, version, source, checksum) |
| 5.3 | **Dependency resolution** | SemVer constraint solving, conflict detection, minimal version selection |
| 5.4 | **Git-based dependencies** | `dep = { git = "url", rev = "abc123" }` support (before registry) |
| 5.5 | **`ruyi build` command** | Compile project with dependency resolution, output to `target/` |
| 5.6 | **`ruyi run` command** | Build + execute in one step |
| 5.7 | **`ruyi add/remove`** | Add or remove dependencies, auto-update lockfile |
| 5.8 | **Module resolution** | Map `import { foo } from "./bar"` to dependency packages; resolve `std::io` to stdlib |

### v0.7 — Package Registry (Priority: HIGH)

| # | Task | Description |
|---|------|-------------|
| 6.1 | **Registry API** | HTTP-based sparse index: `GET /index/{name}`, `GET /api/v1/crates/{name}/{version}` |
| 6.2 | **`ruyi publish`** | Package verification (semver, docs, tests pass) + upload to registry |
| 6.3 | **`ruyi install`** | Download and cache packages from registry |
| 6.4 | **Yank support** | Mark versions as unavailable without deleting them |
| 6.5 | **Search** | `ruyi search <keyword>` to find packages |
| 6.6 | **Documentation hosting** | Auto-generate and host docs on `docs.ruyi-lang.org` |

### v0.8 — Workspace & Build System

| # | Task | Description |
|---|------|-------------|
| 7.1 | **Workspace support** | `[workspace] members = ["crates/*"]` for monorepos |
| 7.2 | **Build profiles** | `[profile.debug]` / `[profile.release]` with optimization/debug/lto settings |
| 7.3 | **`--locked` / `--frozen` flags** | For CI: fail if lockfile is outdated |
| 7.4 | **Cross-compilation** | `--target x86_64-unknown-linux-gnu` via LLVM target triples |
| 7.5 | **Build scripts** | Optional `build.ry` for code generation, custom steps (like build.rs) |
| 7.6 | **Incremental compilation** | Fingerprint-based caching: skip recompilation of unchanged modules |
| 7.7 | **Remote build cache** | Content-addressed cache in `~/.cache/ruyi/` shared across projects |

### v0.9 — Ecosystem Seed

| # | Task | Description |
|---|------|-------------|
| 8.1 | **Featured packages** | `ruyi-http` (HTTP client/server), `ruyi-serialize` (JSON/TOML), `ruyi-cli` (argument parsing) |
| 8.2 | **Package template** | `ruyi init --lib` / `ruyi init --bin` scaffolding |
| 8.3 | **CI template** | `ruyi ci init` generates GitHub Actions workflow |
| 8.4 | **Security audit** | `ruyi audit` checks for known vulnerabilities in dependencies |
| 8.5 | **Dependency tree** | `ruyi tree` shows dependency graph |
| 8.6 | **Outdated check** | `ruyi outdated` reports newer versions available |

---

## Phase 3: Developer Tooling (v1.0+)

### Goal

Provide a world-class developer experience: fast feedback, smart editing, easy debugging.

### v1.0 — LSP & Formatter (Priority: P0)

| # | Task | Description |
|---|------|-------------|
| 9.1 | **tree-sitter-ruyi** | Grammar for syntax highlighting, folding, indentation in any editor |
| 9.2 | **LSP server (v1)** | Diagnostics (parse + type errors), go-to-definition, hover, completion, document symbols |
| 9.3 | **`ruyi fmt`** | Opinionated formatter: 4-space indent, max_width=100, Unix newlines. Minimal config: `ruyifmt.toml` |
| 9.4 | **VS Code extension** | Syntax highlighting via tree-sitter, LSP integration, format-on-save |
| 9.5 | **JetBrains plugin** | Grammar kit + LSP integration for IntelliJ/WebStorm |

### v1.1 — Test Runner & Linter (Priority: P1)

| # | Task | Description |
|---|------|-------------|
| 10.1 | **`ruyi test` runner** | Discover `@test fn` functions, run in parallel, filter by name, capture output |
| 10.2 | **`@test` attribute** | Mark functions as tests; `@test fn test_add() { assert_eq(1+1, 2); }` |
| 10.3 | **`@bench` attribute** | Benchmarking functions with statistical analysis |
| 10.4 | **Test reporter** | TAP, JUnit XML, JSON output formats |
| 10.5 | **`ruyi lint` (clippy equivalent)** | Style issues, common mistakes, performance anti-patterns |
| 10.6 | **Lint categories** | `correctness` (bugs), `style` (conventions), `complexity` (simplification), `performance` (speed) |

### v1.2 — Documentation Generator (Priority: P1)

| # | Task | Description |
|---|------|-------------|
| 11.1 | **`ruyi doc`** | Generate HTML docs from `/** */` doc comments |
| 11.2 | **Doctests** | Extract and run code examples from doc comments as tests |
| 11.3 | **Cross-refs** | Link types, functions, traits across modules |
| 11.4 | **Search index** | Full-text search across all documented items |
| 11.5 | **`ruyi doc --open`** | Build and open in browser |

### v1.3 — Debugger & Advanced Tooling (Priority: P2)

| # | Task | Description |
|---|------|-------------|
| 12.1 | **DWARF debug info** | Emit debug symbols in compiled binaries for LLDB/GDB |
| 12.2 | **DAP integration** | Debug Adapter Protocol for VS Code debugging |
| 12.3 | **`ruyi repl`** | Interactive REPL with incremental compilation |
| 12.4 | **LSP (v2)** | Find references, rename, workspace symbols, code actions (quick fixes) |
| 12.5 | **Inlay hints** | Show inferred types, parameter names inline |
| 12.6 | **Performance profiler** | `ruyi perf record` / `ruyi perf report` using LLVM's perf integration |
| 12.7 | **Fuzzing** | `ruyi fuzz` for lexer/parser fuzz testing |

### v1.4 — IDE Polish

| # | Task | Description |
|---|------|-------------|
| 13.1 | **Code completion** | Context-aware: keywords, identifiers, imports, trait methods |
| 13.2 | **Refactoring** | Extract function, rename symbol, organize imports |
| 13.3 | **Snippets** | Common patterns: `fn`, `class`, `match`, `for-of`, `try-catch` |
| 13.4 | **Inlay type hints** | Show inferred types for `let x = 42` → `let x: int = 42` |
| 13.5 | **Error lens** | Inline error messages in editor |
| 13.6 | **Test explorer** | Tree view of @test functions; run/debug individual tests |

---

## Timeline

```
2026 Q2-Q3  v0.2  Codegen Completion (classes, objects, arrays, for-loops, try/catch)
2026 Q3      v0.3  Runtime Integration (GC wiring, real async, exceptions)
2026 Q3-Q4   v0.4  Typechecker Hardening (trait bounds, impl for, exhaustiveness)
2026 Q4      v0.5  Standard Library Expansion (math/time/json/random/fmt/test)
2026 Q2-Q3   v0.5.x Phase 1 Complete — multithreading, 29 stdlib modules
2026 Q3      v0.6  Package Manager Foundation (manifest, lockfile, deps, build, run)

2027 Q1      v0.6  Package Manager Foundation (manifest, lockfile, deps, build, run)
2027 Q1-Q2   v0.7  Package Registry (publish, install, search, docs hosting)
2027 Q2      v0.8  Workspace & Build System (profiles, cross-compile, incr. compile)
2027 Q3      v0.9  Ecosystem Seed (featured packages, templates, CI, audit)

2027 Q3-Q4   v1.0  LSP & Formatter (tree-sitter, LSP v1, ruyi fmt, VS Code)
2027 Q4      v1.1  Test Runner & Linter (@test, @bench, ruyi lint)
2028 Q1      v1.2  Documentation Generator (ruyi doc, doctests, search)
2028 Q1-Q2   v1.3  Debugger & Advanced Tooling (DWARF, DAP, REPL, perf)
2028 Q2      v1.4  IDE Polish (refactoring, snippets, inlay hints, test explorer)
```

---

## Success Metrics

### Phase 1 Completion Criteria
- [x] Can compile and run a program using classes, objects, arrays, and string concatenation
- [x] `try/catch/finally` works end-to-end with real exception propagation
- [x] Async `fn` actually runs on the work-stealing scheduler (not synchronously)
- [x] GC correctly collects unreferenced objects in a loop
- [x] All stdlib modules pass their integration tests (29 modules, v0.5.10)
- [x] Multithreading support: Channel/Thread/RWLock/TLS/spawn_blocking (v0.5.10)
- [x] `cargo test` passes with solid test coverage (186 runtime tests + ~2400 unit tests)
- [ ] CI pipeline running on every push (GitHub Actions)

### Phase 2 Completion Criteria
- [ ] `ruyi build` compiles a project with 5+ dependencies from the registry
- [ ] `ruyi test` discovers and runs `@test` functions
- [ ] `ruyi publish` uploads a package to the registry
- [ ] Lockfile ensures reproducible builds across machines
- [ ] Workspace with 3+ members builds correctly
- [ ] Cross-compilation to at least 2 targets (linux-x64, macos-arm64)

### Phase 3 Completion Criteria
- [ ] VS Code extension published with syntax highlighting + LSP
- [ ] `ruyi fmt` is idempotent and handles all spec syntax
- [ ] `ruyi test` runs 100+ integration tests in <5 seconds
- [ ] `ruyi doc` generates browsable HTML for any package
- [ ] LSP response time <50ms for completion/hover on files <10K lines
- [ ] Debugger can set breakpoints, step, and inspect variables

---

## Risk Mitigation

| Risk | Impact | Mitigation |
|------|--------|------------|
| LLVM API volatility | High | Pin inkwell to LLVM 14; abstract LLVM calls behind traits |
| Async runtime bugs | High | Extensive async integration tests before v0.3 release |
| Package registry scaling | Medium | Start with Git-based deps (v0.6); add registry incrementally |
| LSP performance | Medium | Use tree-sitter for parsing; incremental type checking |
| Community adoption | Medium | Focus on DX: fast compile, good errors, easy install |
| Stdlib scope creep | Medium | Keep core minimal; community packages for niche needs |

---

## Key Differentiators (Why Ruyi?)

1. **Familiar syntax, no footguns** — JS developers can read Ruyi immediately, but `===`, no `undefined`, no `var`, explicit nullability
2. **Native performance via LLVM** — Zero-cost abstractions, monomorphized generics, zero-cost exceptions
3. **Gradual typing done right** — `dyn` exists but isn't magic; explicit `?` for nullable types
4. **Batteries-included async** — Work-stealing scheduler in stdlib, not a third-party crate
5. **Built-in test framework** — `@test` is a language feature, not a library