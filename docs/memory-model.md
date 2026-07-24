# Ruyi 内存模型

> 版本: 0.6.0-draft | 日期: 2026-07-25

## 1. 概述

Ruyi 采用 **每线程独立 GC 堆**（thread-local GC）模型。每个 OS 线程拥有自己的分代垃圾回收器实例（`GenerationalCollector`），GC 对象在不同线程之间相互隔离。

## 2. 线程与 GC

### 2.1 线程创建

```
主线程 (main)
  └── thread::spawn() → 子线程
       ├── 自动初始化 thread_local CURRENT_COLLECTOR
       ├── 独立 young generation (nursery)
       ├── 独立 old generation
       └── 独立 write barrier / root set
```

每个线程在首次访问 GC 分配（`ruyi_gc_alloc`）时，会自动通过 `gc_exports.rs` 中的 `thread_local!` 宏创建自己的 `GenerationalCollector` 实例。

### 2.2 跨线程数据传递

| 传递方式 | 安全 | 说明 |
|----------|------|------|
| 原始类型 (int, float, bool, string) | ✅ | 值拷贝，无共享 |
| `Arc<T>` | ✅ | 原子引用计数，线程安全 |
| `Mutex<T>` | ✅ | 互斥锁保护 |
| `RWLock<T>` | ✅ | 读写锁保护 |
| `Channel<T>` | ✅ | 消息传递（MPSC） |
| `Atomic<int>` | ✅ | 原子操作 |
| GC 对象（裸指针） | ❌ | 编译期/运行期未强制，但会导致 use-after-free |
| Fiber 句柄 | ❌ | 纤程绑定其创建线程 |

### 2.3 GC 安全性保证

- **写屏障**（`WriteBarrier`）：跨代引用追踪使用 per-thread `Mutex<Vec>`，不跨线程共享
- **根集合**（`RootSet`）：每个线程独立维护 stack roots 和 global roots
- **Async GC roots**：挂起的 async task 通过 `GLOBAL_SCHEDULER` 的锁协调跨线程扫描

## 3. 同步原语

### 3.1 原子操作

所有原子操作使用 `Ordering::SeqCst` 排序（最强一致性保证）：

```
__atomic_i64_load   → AtomicI64::load(SeqCst)
__atomic_i64_store  → AtomicI64::store(SeqCst)
__atomic_i64_cas    → AtomicI64::compare_exchange(SeqCst, SeqCst)
__atomic_i64_fetch_add → AtomicI64::fetch_add(SeqCst)
```

### 3.2 Happens-Before 关系

1. **Mutex unlock → subsequent Mutex lock**: unlock happens-before lock（通过 `std::sync::Mutex` 保证）
2. **Atomic store → subsequent Atomic load**: store happens-before load（`SeqCst` 全局全序）
3. **Channel send → corresponding Channel recv**: send happens-before recv（通过 `mpsc` 内部同步）
4. **Thread spawn → thread entry point**: spawn happens-before entry（`std::thread::spawn` 保证）
5. **Thread join → after join returns**: thread completion happens-before join return

### 3.3 线程本地存储

`ThreadLocal`（`stdlib/thread_local.ry`）基于 `thread_local! { RefCell<HashMap<i64, i64>> }` 实现。每个线程的存储完全独立，不需要同步。详见 `tls_store_ffi.rs`。

## 4. spawn_blocking

`spawn_blocking` 使用 oneshot channel（`std::sync::mpsc::channel`）在调用线程和 worker 线程之间传递结果：

```
调用线程                    Worker 线程
  │                            │
  ├── spawn_blocking(fn, arg) ─┤
  │   ├── 创建 channel        │
  │   ├── thread::spawn ──────→ 执行 fn(arg)
  │   │                        │ 发送结果 → tx.send(result)
  │   ├── await future         │
  │   ├── poll → rx.try_recv() │
  │   └── Ready(result)        │
```

## 5. 已知限制

- **无全局 GC**：当前不支持跨线程共享 GC 对象。需使用 `Arc<T>` 或 `Channel<T>` 进行跨线程数据传递
- **Select 有限**：`Channel::select` 使用轮询而非高效的事件通知（受限于 stable Rust `mpsc` API）
- **Fiber 未实现**：纤程（stackful coroutine）计划在 v0.8 实现
- **thread_local let 语法未实现**：当前使用 `ThreadLocal` 类代替语言级 `thread_local` 关键字
