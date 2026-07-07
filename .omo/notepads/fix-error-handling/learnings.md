
## 2026-05-07: Fix error_handling.ry compilation error

**Problem**: `codegen error: Unknown class: Error` — codegen 不支持内置 `Error` 类和类继承 (`extends`)。

**Fix applied to `examples/error_handling.ry`**:
1. 在文件开头添加了用户自定义的 `class Error { message: string; fn new(...) }` 作为基础错误类
2. 将 `class TypeError extends Error` 改为独立类 `class TypeError { message: string; fn new(...) }`
3. 将 `class RangeError extends Error` 改为独立类 `class RangeError { message: string; fn new(...) }`
4. 将 `class ValidationError extends Error` 改为独立类 `class ValidationError { message: string; field: string; fn new(...) }`
5. 将所有 `super.new(message)` 调用替换为 `self.message = message`

**Result**: 编译成功，二进制运行正常，所有错误处理演示（try/catch/finally、throw、custom error types、never type）均通过。
