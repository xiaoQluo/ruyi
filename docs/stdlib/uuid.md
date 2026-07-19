# uuid — UUID 模块

## 概述

`uuid` 模块提供 UUID（通用唯一标识符）生成功能。纯 `.ry` 实现，无需额外 FFI。

**源文件**: `stdlib/uuid.ry`

**导入**: `import { ... } from "./uuid"`

---

## 函数

| 函数 | 签名 | 说明 |
|------|------|------|
| `v4` | `fn v4(): string` | 生成一个 UUID v4 字符串（36 字符） |
| `v4Batch` | `fn v4Batch(count: int): Array<string>` | 批量生成指定数量的 UUID v4 |

---

## UUID v4 格式

```
xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx
```

其中：
- `x` 为任意十六进制数字
- `4` 为版本位（版本 4）
- `y` 为 8、9、a 或 b（变体位）

---

## 注意事项

- UUID v4 符合 RFC 9562（原 RFC 4122）
- 使用 `Random` 类生成随机字节
- `v4Batch()` 适用于需要一次生成多个 UUID 的场景
