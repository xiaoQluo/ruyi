# crypto — 密码学模块

## 概述

`crypto` 模块提供完整的密码学原语，纯 `.ry` 实现，无需额外 FFI 依赖。
涵盖哈希函数、消息认证码、密钥派生和随机数生成。

**源文件**: `stdlib/crypto.ry`

**导入**: `import { ... } from "./crypto"`

---

## 哈希函数

### SHA-256（RFC 6234）

| 函数 | 签名 | 说明 |
|------|------|------|
| `sha256` | `fn sha256(data: string): string` | 计算 SHA-256 哈希，返回 64 字符小写 hex 字符串 |
| `sha256Raw` | `fn sha256Raw(data: Array<int>): Array<int>` | 计算原始 SHA-256 哈希，返回 32 字节数组 |
| `sha256Bytes` | `fn sha256Bytes(data: string): Array<int>` | 计算 SHA-256 哈希，返回 32 字节数组 |

### SHA-512（RFC 6234）

| 函数 | 签名 | 说明 |
|------|------|------|
| `sha512` | `fn sha512(data: string): string` | 计算 SHA-512 哈希，返回 128 字符小写 hex 字符串 |
| `sha512Raw` | `fn sha512Raw(data: Array<int>): Array<int>` | 计算原始 SHA-512 哈希，返回 64 字节数组 |
| `sha512Bytes` | `fn sha512Bytes(data: string): Array<int>` | 计算 SHA-512 哈希，返回 64 字节数组 |

### SHA-1（RFC 3174）

| 函数 | 签名 | 说明 |
|------|------|------|
| `sha1` | `fn sha1(data: string): string` | 计算 SHA-1 哈希，返回 40 字符小写 hex 字符串 |
| `sha1Raw` | `fn sha1Raw(data: Array<int>): Array<int>` | 计算原始 SHA-1 哈希，返回 20 字节数组 |
| `sha1Bytes` | `fn sha1Bytes(data: string): Array<int>` | 计算 SHA-1 哈希，返回 20 字节数组 |

### MD5（RFC 1321）

| 函数 | 签名 | 说明 |
|------|------|------|
| `md5` | `fn md5(data: string): string` | 计算 MD5 哈希，返回 32 字符小写 hex 字符串 |
| `md5Raw` | `fn md5Raw(data: Array<int>): Array<int>` | 计算原始 MD5 哈希，返回 16 字节数组 |
| `md5Bytes` | `fn md5Bytes(data: string): Array<int>` | 计算 MD5 哈希，返回 16 字节数组 |

---

## HMAC（RFC 2104）

| 函数 | 签名 | 说明 |
|------|------|------|
| `hmacSha256` | `fn hmacSha256(key: string, message: string): string` | HMAC-SHA-256，返回 hex 字符串 |
| `hmacSha256Bytes` | `fn hmacSha256Bytes(key: string, message: string): Array<int>` | HMAC-SHA-256，返回 32 字节数组 |
| `hmacSha512` | `fn hmacSha512(key: string, message: string): string` | HMAC-SHA-512，返回 hex 字符串 |
| `hmacSha512Bytes` | `fn hmacSha512Bytes(key: string, message: string): Array<int>` | HMAC-SHA-512，返回 64 字节数组 |
| `hmacSha1` | `fn hmacSha1(key: string, message: string): string` | HMAC-SHA-1，返回 40 字符 hex 字符串 |

---

## PBKDF2（RFC 2898）

| 函数 | 签名 | 说明 |
|------|------|------|
| `pbkdf2Sha256` | `fn pbkdf2Sha256(password: string, salt: string, iterations: int, keyLen: int): string` | PBKDF2-HMAC-SHA256 密钥派生 |
| `pbkdf2Sha512` | `fn pbkdf2Sha512(password: string, salt: string, iterations: int, keyLen: int): string` | PBKDF2-HMAC-SHA512 密钥派生 |

---

## CSPRNG（密码学安全随机数）

| 函数 | 签名 | 说明 |
|------|------|------|
| `cryptoRandomBytes` | `fn cryptoRandomBytes(size: int): Array<int>` | 生成密码学安全随机字节 |
| `cryptoRandomHex` | `fn cryptoRandomHex(size: int): string` | 生成密码学安全随机 hex 字符串 |
| `cryptoRandomInt` | `fn cryptoRandomInt(min: int, max: int): int` | 生成 `[min, max]` 范围内的均匀随机整数 |
| `cryptoRandomUuid` | `fn cryptoRandomUuid(): string` | 生成 UUID v4（RFC 4122） |

---

## 常量时间比较

| 函数 | 签名 | 说明 |
|------|------|------|
| `timingSafeEqual` | `fn timingSafeEqual(a: string, b: string): bool` | 常量时间字符串比较，抵抗时序攻击 |
| `timingSafeEqualBytes` | `fn timingSafeEqualBytes(a: Array<int>, b: Array<int>): bool` | 常量时间字节数组比较，抵抗时序攻击 |

---

## 测试向量

```ruyi
sha256("hello world") === "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9"
sha256("")           === "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
```

---

## 注意事项

- 所有哈希函数基于字符串输入，内部转换为字节数组后计算
- HMAC 使用 `ipad = 0x36`, `opad = 0x5C` 标准构造
- PBKDF2 使用 `keyLen` 参数指定输出字节数
- `cryptoRandomBytes` 通过 `__io_read_random` FFI 获取操作系统熵源
- 常量时间比较函数使用累加器模式，避免早期返回
