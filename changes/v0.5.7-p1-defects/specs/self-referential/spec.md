# 间接自引用类型

## ADDED Requirements

### Requirement: 间接 Self 引用的类字段可编译

`typechecker::class::infer_field_types` MUST 接受通过容器（`Box` / `Option` / `List` / 自定义泛型容器）包装的 `Self` 引用。字段类型必须包含一层或以上的间接包装，禁止裸 `Self` 直接作为字段类型。

#### Scenario: 同类指针字段编译通过

- **WHEN** 用户声明 `class Node { next: Node?; }`
- **THEN** 类型检查器 MUST 接受该声明并产出非错误诊断，字段类型在 LLVM 布局阶段必须解析为对 `Node` 的指针

#### Scenario: 自定义容器间接引用编译通过

- **WHEN** 用户声明 `class Box<T> { value: T; }` 后在另一个类中以 `Box<Self>` 作为字段
- **THEN** 类型检查器 MUST 接受间接自引用并完成字段布局，编译产物运行期可正确分配

### Requirement: 裸 Self 字段保持拒绝

`typechecker::class::infer_field_types` MUST 继续拒绝裸 `Self` 作为字段类型（无任何间接包装），保持 v0.5.5 已有行为不变。

#### Scenario: 裸 Self 字段报错

- **WHEN** 用户声明 `class Bad { me: Self; }`
- **THEN** 类型检查器 MUST 产出明确的诊断错误（带行号），指出不允许裸 `Self` 作为字段类型

#### Scenario: 自引用环长度上限

- **WHEN** 类型检查器在类型展开过程中检测到递归深度超过预设阈值
- **THEN** MUST 产出诊断错误而非无限递归，确保 `class A { b: B?; } class B { a: A?; }` 之类的对偶链在有限步内终止