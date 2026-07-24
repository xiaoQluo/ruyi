# process — 进程模块

## 概述

`process` 模块提供进程管理和系统命令执行功能。

**源文件**: `stdlib/process.ry`

**导入**: `import { ... } from "./process"`

---

## Process 类

| 方法 | 签名 | 说明 |
|------|------|------|
| `create` | `static fn create(command: string, options: ProcessOptions? = null): Process` | 创建新进程 |
| `exec` | `static fn exec(command: string): ProcessResult` | 同步执行命令并返回结果 |
| `execWith` | `static fn execWith(command: string, options: ExecOptions): ProcessResult` | 同步执行命令（带选项） |
| `spawn` | `static fn spawn(command: string, args: Array<string>? = null): Process` | 派生子进程 |
| `spawnWith` | `static fn spawnWith(command: string, args: Array<string>?, options: ProcessOptions): Process` | 派生子进程（带选项） |
| `wait` | `fn wait(): int` | 等待进程完成，返回退出码 |
| `waitAsync` | `async fn waitAsync(): Future<int>` | 异步等待进程完成 |
| `kill` | `fn kill(signal: int = 15): void` | 终止进程（默认 SIGTERM） |
| `writeInput` | `fn writeInput(input: string): void` | 向进程 stdin 发送输入 |
| `closeInput` | `fn closeInput(): void` | 关闭进程 stdin |
| `readOutput` | `fn readOutput(): string?` | 读取 stdout 输出 |
| `readError` | `fn readError(): string?` | 读取 stderr 输出 |

**属性**:

| 属性 | 类型 | 说明 |
|------|------|------|
| `pid` | `int` | 进程 PID |
| `command` | `string` | 启动命令 |
| `cwd` | `string` | 工作目录 |
| `env` | `Map<string, string>` | 环境变量 |
| `exitCode` | `int?` | 退出码（null 表示运行中） |
| `isRunning` | `bool` | 是否运行中 |

---

## ProcessOptions 类

| 方法 | 签名 | 说明 |
|------|------|------|
| `default` | `static fn default(): ProcessOptions` | 创建默认选项 |
| `withCwd` | `fn withCwd(path: string): ProcessOptions` | 设置工作目录 |
| `withEnv` | `fn withEnv(env: Map<string, string>): ProcessOptions` | 设置环境变量 |
| `withShell` | `fn withShell(useShell: bool): ProcessOptions` | 设置是否使用 shell |

**属性**:

| 属性 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `cwd` | `string?` | `null` | 工作目录 |
| `env` | `Map<string, string>?` | `null` | 环境变量 |
| `shell` | `bool` | `true` | 是否使用 shell |

---

## ExecOptions 类

| 方法 | 签名 | 说明 |
|------|------|------|
| `default` | `static fn default(): ExecOptions` | 创建默认选项 |
| `withCwd` | `fn withCwd(path: string): ExecOptions` | 设置工作目录 |
| `withEnv` | `fn withEnv(env: Map<string, string>): ExecOptions` | 设置环境变量 |
| `withShell` | `fn withShell(useShell: bool): ExecOptions` | 设置是否使用 shell |
| `withTimeout` | `fn withTimeout(ms: int): ExecOptions` | 设置超时时间（毫秒） |

---

## ProcessResult 类

| 方法 | 签名 | 说明 |
|------|------|------|
| `create` | `static fn create(stdout: string, stderr: string, exitCode: int): ProcessResult` | 创建结果 |
| `ensureSuccess` | `fn ensureSuccess(): void` | 进程失败时抛出 `ProcessException` |

**属性**:

| 属性 | 类型 | 说明 |
|------|------|------|
| `stdout` | `string` | 标准输出 |
| `stderr` | `string` | 标准错误 |
| `exitCode` | `int` | 退出码 |
| `success` | `bool` | 是否成功（exitCode === 0） |

---

## ProcessException 类

| 方法 | 签名 | 说明 |
|------|------|------|
| `create` | `static fn create(message: string, process: Process? = null): ProcessException` | 创建异常 |
| `getMessage` | `fn getMessage(): string` | 返回错误消息 |

---

## Signal 类（信号常量）

| 常量 | 值 | 说明 |
|------|-----|------|
| `Signal.HUP` | `1` | 挂起 |
| `Signal.INT` | `2` | 中断（Ctrl+C） |
| `Signal.QUIT` | `3` | 退出（Ctrl+\） |
| `Signal.KILL` | `9` | 强制终止 |
| `Signal.USR1` | `10` | 用户自定义信号 1 |
| `Signal.USR2` | `12` | 用户自定义信号 2 |
| `Signal.TERM` | `15` | 终止（默认） |

| 方法 | 签名 | 说明 |
|------|------|------|
| `isAvailable` | `static fn isAvailable(signal: int): bool` | 检查信号在当前平台是否可用 |

---

## 环境变量函数

| 函数 | 签名 | 说明 |
|------|------|------|
| `getEnv` | `fn getEnv(name: string): string?` | 获取环境变量值 |
| `setEnv` | `fn setEnv(name: string, value: string): void` | 设置环境变量 |
| `getAllEnv` | `fn getAllEnv(): Map<string, string>` | 获取所有环境变量 |
| `getPID` | `fn getPID(): int` | 获取当前进程 ID |
| `getPPID` | `fn getPPID(): int` | 获取父进程 ID |
| `getPlatform` | `fn getPlatform(): string` | 获取系统平台（`"linux"`, `"macos"`, `"windows"`, `"unknown"`） |
| `getCPUCount` | `fn getCPUCount(): int` | 获取 CPU 核心数 |
| `getTotalMemory` | `fn getTotalMemory(): int` | 获取总系统内存（字节） |
| `getFreeMemory` | `fn getFreeMemory(): int` | 获取空闲系统内存（字节） |

---

## 注意事项

- `Process.exec()` 是同步阻塞调用
- `Process.spawn()` 用于更精细的进程控制（读写 stdin/stdout/stderr）
- `Signal.KILL`（9）在 Windows 上不可用
- 默认使用 shell 执行命令（`shell: true`）
