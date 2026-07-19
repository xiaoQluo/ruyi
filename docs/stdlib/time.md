# time — 时间模块

## 概述

`time` 模块提供时间相关函数，包括获取当前时间戳、休眠和时间格式化。
所有时间戳均为 Unix 时间戳（自 1970-01-01 00:00:00 UTC 以来的秒数）。

**源文件**: `stdlib/time.ry`

**导入**: `import { ... } from "./time"`

---

## 函数

| 函数 | 签名 | 说明 |
|------|------|------|
| `now` | `fn now(): int` | 返回当前 Unix 时间戳（秒） |
| `timestamp` | `fn timestamp(): int` | 返回当前 Unix 时间戳（毫秒） |
| `sleep` | `fn sleep(seconds: float): void` | 休眠指定秒数 |
| `format_time` | `fn format_time(timestamp: int): string` | 格式化时间戳为 `"YYYY-MM-DD HH:MM:SS"` 格式 |
| `now_string` | `fn now_string(): string` | 返回当前时间的格式化字符串 |

---

## 注意事项

- `now()` 返回秒级精度，`timestamp()` 返回毫秒级精度
- `sleep()` 接受浮点数，可实现亚秒级休眠（如 `sleep(0.5)` 休眠 500ms）
- `format_time()` 接受秒级时间戳作为参数
