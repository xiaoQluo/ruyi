# datetime — 日期时间模块

## 概述

`datetime` 模块提供丰富的 `Date` 类和日期工具函数，支持日期时间解析、格式化、算术运算和比较。
所有时间戳以毫秒为单位（Unix 纪元起算）。`Date` 类是不可变的——算术运算返回新实例。

**源文件**: `stdlib/datetime.ry`

**导入**: `import { ... } from "./datetime"`

---

## 时间单位常量

| 常量 | 类型 | 值（毫秒） | 说明 |
|------|------|-----------|------|
| `DAY_MS` | `int` | `86400000` | 一天 |
| `HOUR_MS` | `int` | `3600000` | 一小时 |
| `MINUTE_MS` | `int` | `60000` | 一分钟 |
| `SECOND_MS` | `int` | `1000` | 一秒 |
| `WEEK_MS` | `int` | `604800000` | 一周 |

---

## Date 类

### 构造函数

| 方法 | 签名 | 说明 |
|------|------|------|
| `new` | `fn new(ts: int = -1)` | 创建 Date 实例，默认当前时间 |

### 静态工厂方法

| 方法 | 签名 | 说明 |
|------|------|------|
| `now` | `static fn now(): Date` | 创建当前时刻的 Date |
| `parse` | `static fn parse(isoString: string): Date` | 解析 ISO 8601 字符串（如 `"2026-07-17T15:30:00.000Z"`） |
| `fromParts` | `static fn fromParts(year: int, month: int, day: int, hours: int = 0, minutes: int = 0, seconds: int = 0, ms: int = 0): Date` | 从日期时间分量创建 Date |

### 获取器方法

| 方法 | 签名 | 说明 |
|------|------|------|
| `year` | `fn year(self): int` | 返回年份 |
| `month` | `fn month(self): int` | 返回月份（1-12） |
| `day` | `fn day(self): int` | 返回日（1-31） |
| `hours` | `fn hours(self): int` | 返回小时（0-23） |
| `minutes` | `fn minutes(self): int` | 返回分钟（0-59） |
| `seconds` | `fn seconds(self): int` | 返回秒（0-59） |
| `milliseconds` | `fn milliseconds(self): int` | 返回毫秒（0-999） |
| `dayOfWeek` | `fn dayOfWeek(self): int` | 返回星期几（0=周日, 6=周六） |
| `dayOfYear` | `fn dayOfYear(self): int` | 返回一年中的第几天（1-366） |
| `weekOfYear` | `fn weekOfYear(self): int` | 返回一年中的第几周（1-53） |
| `timestamp` | `fn timestamp(self): int` | 返回 Unix 时间戳（毫秒） |
| `isLeapYear` | `fn isLeapYear(self): bool` | 检查是否为闰年 |

### 格式化方法

| 方法 | 签名 | 说明 |
|------|------|------|
| `toISOString` | `fn toISOString(self): string` | ISO 8601 格式 `"YYYY-MM-DDThh:mm:ss.sssZ"` |
| `toString` | `fn toString(self): string` | 可读格式 `"YYYY-MM-DD hh:mm:ss"` |
| `toDateString` | `fn toDateString(self): string` | 仅日期 `"YYYY-MM-DD"` |
| `toTimeString` | `fn toTimeString(self): string` | 仅时间 `"hh:mm:ss"` |
| `format` | `fn format(self, pattern: string): string` | 自定义格式（支持 `YYYY`, `YY`, `MM`, `DD`, `hh`, `mm`, `ss`, `SSS`） |

### 算术方法（返回新 Date 实例）

| 方法 | 签名 | 说明 |
|------|------|------|
| `addDays` | `fn addDays(self, n: int): Date` | 加/减天 |
| `addHours` | `fn addHours(self, n: int): Date` | 加/减小时 |
| `addMinutes` | `fn addMinutes(self, n: int): Date` | 加/减分钟 |
| `addSeconds` | `fn addSeconds(self, n: int): Date` | 加/减秒 |
| `addMonths` | `fn addMonths(self, n: int): Date` | 加/减月（超限日自动截断） |
| `addYears` | `fn addYears(self, n: int): Date` | 加/减年（闰年 2 月 29 日自动截断） |

### 比较方法

| 方法 | 签名 | 说明 |
|------|------|------|
| `isBefore` | `fn isBefore(self, other: Date): bool` | 是否早于另一个日期 |
| `isAfter` | `fn isAfter(self, other: Date): bool` | 是否晚于另一个日期 |
| `equals` | `fn equals(self, other: Date): bool` | 是否表示同一时刻 |
| `diffDays` | `fn diffDays(self, other: Date): int` | 整日差异 |
| `diffHours` | `fn diffHours(self, other: Date): int` | 整小时差异 |
| `diffMinutes` | `fn diffMinutes(self, other: Date): int` | 整分钟差异 |

---

## 自由函数

| 函数 | 签名 | 说明 |
|------|------|------|
| `now` | `fn now(): Date` | 返回当前时刻，等价于 `Date.now()` |
| `parse` | `fn parse(isoString: string): Date` | 解析 ISO 8601 字符串，等价于 `Date.parse()` |
| `daysInMonth` | `fn daysInMonth(year: int, month: int): int` | 返回指定月份的天数 |
| `isLeapYear` | `fn isLeapYear(year: int): bool` | 判断是否为闰年 |

---

## 注意事项

- `Date` 类是不可变的，所有算术方法返回新实例
- 时间戳为 UTC 时间，所有方法返回 UTC 值
- `addMonths` 在目标月份天数较少时自动截断（如 1 月 31 日加 1 月 → 2 月 28/29 日）
- `Date.parse()` 支持的格式：`"YYYY-MM-DD"`, `"YYYY-MM-DDThh:mm:ss"`, `"YYYY-MM-DDThh:mm:ss.sssZ"`
- `format()` 方法支持自定义模式字符串，较长的标记（如 `YYYY`）优先替换
