# test/repro/ — 回归测试用例存档

本目录存放历史上用于复现编译器缺陷的最小 `.ry` 用例。这些用例**不属于**用户教程示例(`examples/`),也**不**参与 `examples/run_examples.sh` 的运行。

## 文件来源

文件从 `examples/repro_*.ry` 迁移而来。原来的位置与教程示例混杂,会被 `find` 自动发现并纳入示例运行器,造成:

- 污染示例运行报告(`Pass` / `Fail` 中混入回归用例)
- 与教程示例命名风格不一致
- 无教学价值(只是最小复现代码)

迁移后,这些用例仅作存档,供将来重构时人工验证 codegen 行为是否仍然正确。

## 添加新用例

如果将来某个 change 需要记录一个 bug 的最小复现:

1. 在 `test/repro/` 下新建 `repro_<short_bug_name>.ry`。
2. 在文件头加注释,说明:
   - bug 的历史背景(可关联 `changes/<fix-name>/proposal.md`)
   - 修复后期望的输出(用于人工核对)
3. **不要**把本目录的文件移回 `examples/`,以免污染示例运行报告。

## 如何手动验证

```bash
# 1. 构建编译器
make build-release

# 2. 运行单个回归用例
./target/release/ruyic test/repro/repro_charat.ry -o /tmp/repro_charat
/tmp/repro_charat
```

期望输出与每个文件头注释中的"预期输出"一致。
