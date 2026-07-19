# net — 网络模块

## 概述

`net` 模块提供 TCP 和 UDP 网络功能，基于 `__net_tcp_*` 和 `__net_udp_*` FFI 原语。

**源文件**: `stdlib/net.ry`

**导入**: `import { ... } from "./net"`

---

## TCPSocket — TCP 客户端

| 方法 | 签名 | 说明 |
|------|------|------|
| `connect` | `static fn connect(host: string, port: int): TCPSocket` | 连接到远程主机 |
| `read` | `fn read(maxBytes: int): string` | 从套接字读取最多 maxBytes 字节 |
| `write` | `fn write(data: string): int` | 向套接字写入数据，返回实际写入字节数 |
| `setTimeout` | `fn setTimeout(timeoutMs: int): int` | 设置读写超时（毫秒），0 表示无限阻塞 |
| `isValid` | `fn isValid(): bool` | 检查连接是否有效 |
| `close` | `fn close(): void` | 关闭套接字连接 |

---

## TCPServer — TCP 服务端

| 方法 | 签名 | 说明 |
|------|------|------|
| `listen` | `static fn listen(host: string, port: int): TCPServer` | 绑定到地址并开始监听 |
| `accept` | `fn accept(): TCPSocket` | 接受新的客户端连接（阻塞） |
| `isValid` | `fn isValid(): bool` | 检查服务端是否有效 |
| `close` | `fn close(): void` | 关闭服务端，停止接受新连接 |

---

## UDPSocket — UDP 数据报套接字

| 方法 | 签名 | 说明 |
|------|------|------|
| `new` | `static fn new(): UDPSocket` | 创建新的 UDP 套接字 |
| `bind` | `fn bind(host: string, port: int): int` | 绑定到本地地址 |
| `sendTo` | `fn sendTo(host: string, port: int, data: string): int` | 向目标发送数据报 |
| `recvFrom` | `fn recvFrom(maxBytes: int): string` | 接收数据报（阻塞） |
| `senderHost` | `fn senderHost(): string` | 返回最近一次接收的发送者 IP |
| `senderPort` | `fn senderPort(): int` | 返回最近一次接收的发送者端口 |
| `isValid` | `fn isValid(): bool` | 检查套接字是否有效 |
| `close` | `fn close(): void` | 关闭 UDP 套接字 |

---

## 示例

### TCP 客户端
```ruyi
let sock = TCPSocket.connect("example.com", 80);
sock.write("GET / HTTP/1.0\r\nHost: example.com\r\n\r\n");
let resp = sock.read(4096);
sock.close();
```

### TCP 服务端
```ruyi
let server = TCPServer.listen("0.0.0.0", 8080);
let client = server.accept();
let data = client.read(1024);
client.write("Hello from Ruyi!\n");
client.close();
server.close();
```

### UDP
```ruyi
// 发送端
let sock = UDPSocket.new();
sock.sendTo("127.0.0.1", 9999, "Hello UDP!");
sock.close();

// 接收端
let sock = UDPSocket.new();
sock.bind("0.0.0.0", 9999);
let data = sock.recvFrom(1024);
print("data: " + data);
sock.close();
```

---

## 注意事项

- TCP `read` 和 `write` 是阻塞操作
- `accept()` 是阻塞的，直到有新连接
- `sendTo` 不需要先 `bind`，但 `recvFrom` 前必须先 `bind`
- 内部使用 `_fd` 句柄（负值表示错误）
