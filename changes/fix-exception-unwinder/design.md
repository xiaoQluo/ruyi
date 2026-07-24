# Design: fix-exception-unwinder

## Context

**Current State**: Ruyi v0.5.9 的异常系统呈现"基础设施完整但桥接断裂"的状态：

| 层级 | 组件 | 状态 |
|------|------|------|
| Codegen | `compile_throw` (stmt.rs:718) | ✅ 完整——调用 `ruyi_throw`，处理 try 内/外两种路径 |
| Codegen | try/catch/finally landing pads (stmt.rs:880-958) | ✅ 完整——catch 变量绑定、finally 传播、rethrow |
| Codegen | `declare_ruyi_throw` (builtins.rs:132) | ✅ 已声明 |
| Runtime | `ruyi_throw(exception: *mut ExceptionObject)` (exception/runtime.rs:33) | ✅ 真实 unwinder——调用 `_Unwind_RaiseException` |
| Runtime | `throw_exception(exc: RuyiException)` (exception.rs:286) | ❌ **用 `panic!()` 占位** |
| Runtime | C FFI `ruyi_throw(msg: *const i8)` (c_exports.rs:9) | ❌ **仅存储 pending pointer** |

断裂点：`throw_exception` 和 C FFI `ruyi_throw` 未连接到已实现的 `_Unwind_RaiseException` 路径。

**Constraints**:
- LLVM 14 绑定，Itanium C++ ABI（macOS/Linux 兼容）
- `exception/runtime.rs:ruyi_throw` 为 `unsafe fn`，需在调用处妥善处理 safety
- `_Unwind_RaiseException` 可能返回 `_URC_END_OF_STACK`（无 handler 时）
- 测试环境可能无完整 unwinder 支持

## Goals

1. `throw_exception` 使用 `_Unwind_RaiseException` 代替 `panic!()`
2. C FFI `ruyi_throw` 构建 `ExceptionObject` 并触发真正的 unwinder
3. 编译后的 Ruyi 程序 throw→catch 端到端工作
4. 零回归——现有 codegen 测试全部保持通过

## Decisions

### D1: throw_exception 连接到现有 unwinder

**Choice**: `throw_exception()` 直接调用 `exception/runtime.rs` 中已存在的 `ruyi_throw(exception: *mut ExceptionObject) -> !`。`throw_exception` 负责将 `RuyiException` 包装为 `ExceptionObject`，然后委托给 `ruyi_throw`。

**Rationale**:
- `exception/runtime.rs:ruyi_throw` 已经完整实现了 `UnwindException` 分配、`_Unwind_RaiseException` 调用、cleanup 回调注册
- 避免重复实现相同的 unwinder 逻辑
- `throw_exception` 保持为安全的 `pub fn` 入口，内部 `unsafe` 边界清晰

**Implementation**:
```rust
pub fn throw_exception(exc: RuyiException) -> ! {
    let obj = ExceptionObject::new(exc.type_id, exc.message);
    let ptr = Box::into_raw(Box::new(obj));
    unsafe { crate::exception::runtime::ruyi_throw(ptr); }
}
```

**Alternatives considered**:
- *直接导出 `ruyi_throw` 替代 `throw_exception`*: `ruyi_throw` 接受 `*mut ExceptionObject`（裸指针），破坏了模块的安全抽象边界
- *在 `throw_exception` 中内联 unwinder 逻辑*: 代码重复，且 `_Unwind_RaiseException` FFI 声明分散

### D2: C FFI ruyi_throw 升级

**Choice**: `c_exports.rs:ruyi_throw(msg: *const i8)` 在函数体内构建 `RuyiException`，调用 `throw_exception`。移除 `PENDING_EXCEPTION` 存储逻辑（或保留作为 fallback 但不再为主路径）。

**Rationale**:
- Codegen 通过 `ruyi_throw` FFI 符号调用 throw——这是 codegen 和 runtime 的唯一 throw 接口
- 当前实现仅存储 pending exception 是 return-based 异常模型的遗留代码
- 升级后 codegen 无需改动——调用的还是同一个符号名

**Implementation**:
```rust
#[no_mangle]
pub extern "C" fn ruyi_throw(msg: *const i8) {
    let c_str = unsafe { CStr::from_ptr(msg) };
    let message = c_str.to_string_lossy().into_owned();
    let exc = RuyiException::new(builtin_type_ids::ERROR, message);
    throw_exception(exc);
}
```

**Alternatives considered**:
- *新符号名（如 `ruyi_unwind`）*: 需要修改 codegen 的调用符号，无必要增加复杂度
- *保留 return-based 模型作为主路径*: 与 landing pad 基础设施不匹配，try/catch 无法正常工作

### D3: compile_throw 的 return-based 模式处理

**Choice**: 保留 `compile_throw` 现有的 "pending exception + return" 模式作为**无 try 块时的 fallback**，但当代码在 try 块内时，让 `ruyi_throw`（现在调用 `_Unwind_RaiseException`）接管。

**Rationale**:
- `compile_throw` 在 try 块内时已正确分支到 `catch_bb`（line 728）——这要求 `ruyi_throw` 返回后才能继续
- 但是 `_Unwind_RaiseException` 调用后**不返回**——控制流直接跳转到 landing pad
- 这是一个架构不匹配：如果用真正的 unwinder，`compile_throw` 中 `ruyi_throw` 调用后的所有代码（line 726-755）都不会执行

**Decision**: 采用两阶段方案：
1. **此 Change 内**：先使 `c_exports.rs:ruyi_throw` 调用 `throw_exception` → `_Unwind_RaiseException`，验证 landing pad 正确捕获异常
2. **后续优化**：若 unwinder 工作正常，简化 `compile_throw` 移除 return-based fallback

为保障此 Change 的稳定性：在 `compile_throw` 的 `ruyi_throw` 调用后保留现有的 `build_unreachable()`（line 738），因为 `ruyi_throw` 现在是 `-> !` 函数。

**Alternatives considered**:
- *同时重构 compile_throw*: 风险过高——landing pad 验证和 codegen 重构应分两个 change
- *保持 return-based，不启用 unwinder*: 无法验证 landing pad 的正确性

### D4: _Unwind_RaiseException 返回值处理

**Choice**: 当 `_Unwind_RaiseException` 返回 `_URC_END_OF_STACK`（无 handler）时，调用 `ruyi_exception_cleanup` 释放资源并 `std::process::abort()`。

**Rationale**:
- `exception/runtime.rs:ruyi_throw` 已有此逻辑（line 47-48）
- 无 handler 的异常是编程错误——abort 优于静默忽略

**Alternatives considered**:
- *打印错误并 exit(1)*: 语义不清——unwinder 未找到 handler 通常表示 landing pad 表不完整
- *调用 Rust panic!*: 我们正在消除 panic!，不应引入新的使用

### D5: 测试策略

**Choice**: 三层验证——unit（Rust `#[test]`）、integration（Rust 测试驱动 .ry 编译执行）、end-to-end（.ry 集成测试 fixtures）。

**Rationale**:
- unit 层验证 `throw_exception` 和 C FFI 的正确性
- integration 层通过 `try_catch_invoke` 和 `codegen` 测试验证 codegen→runtime 桥接
- end-to-end 层通过新增 .ry fixtures 验证编译后的真实程序行为

## Risks And Trade-Offs

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| `_Unwind_RaiseException` 在 CI/测试环境返回 `_URC_END_OF_STACK` | Medium | 测试失败 | 为 affected 测试添加 `#[should_panic]` 或在无 handler 场景中验证 abort |
| 与 Change B 的 codegen "Complex new expression" 修复有时序依赖 | Low | try_catch_invoke 测试可能仍失败 | Change B 完成后此风险消除；本 Change 可先验证非 new 表达式的 throw |
| macOS ARM64 `_Unwind_RaiseException` ABI 差异 | Low | SIGSEGV | `exception/runtime.rs` 已验证 Itanium ABI（macOS 使用相同 ABI） |
| PENDING_EXCEPTION 原子变量移除影响现有调用方 | Low | 编译错误 | 保留 `PENDING_EXCEPTION` 作为 deprecated 字段，移除直接引用 |

### Trade-off: 两阶段 vs 一次性完整重构

选择两阶段方案（先连接 unwinder，后优化 codegen）意味着 `compile_throw` 中存在 dead code（`ruyi_throw` 调用后的分支）。这是一个可接受的 trade-off——landing pad 验证优先级高于代码清理。
